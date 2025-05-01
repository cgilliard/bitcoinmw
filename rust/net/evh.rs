#![allow(dead_code)]
#![allow(unused_variables)]

use core::marker::PhantomData;
use core::mem::forget;
use core::ops::FnMut;
use net::constants::*;
use net::errors::*;
use net::multiplex::Event;
use net::multiplex::Multiplex;
use net::multiplex::RegisterType;
use net::socket::Socket;
use prelude::*;
use util::channel::{channel, Receiver, Sender};
use util::lock::{LockReadGuard, LockWriteGuard};

pub type OnRecv<T, V> = Box<dyn FnMut(&mut T, &mut Connection<T, V>, &[u8]) -> Result<()>>;
pub type OnAccept<T, V> = Box<dyn FnMut(&mut T, &Connection<T, V>) -> Result<()>>;
pub type OnClose<T, V> = Box<dyn FnMut(&mut T, &Connection<T, V>) -> Result<()>>;

struct AcceptorData<T, V>
where
	T: Clone,
	V: Clone,
{
	socket: Socket,
	on_recv: Rc<OnRecv<T, V>>,
	on_accept: Rc<OnAccept<T, V>>,
	on_close: Rc<OnClose<T, V>>,
	attach: T,
}

struct InboundData<T, V>
where
	T: Clone,
	V: Clone,
{
	socket: Socket,
	acceptor: Connection<T, V>,
	is_closed: bool,
	lock: Lock,
	multiplex: Multiplex,
	opt: Option<V>,
}

struct OutboundData<T, V>
where
	T: Clone,
	V: Clone,
{
	socket: Socket,
	on_recv: Rc<OnRecv<T, V>>,
	on_close: Rc<OnClose<T, V>>,
	is_closed: bool,
	lock: Lock,
	attach: T,
	opt: Option<V>,
}

enum ConnectionData<T, V>
where
	T: Clone,
	V: Clone,
{
	Inbound(InboundData<T, V>),
	Outbound(OutboundData<T, V>),
	Acceptor(AcceptorData<T, V>),
	Close,
}

#[derive(Clone)]
pub struct Connection<T, V>
where
	T: Clone,
	V: Clone,
{
	inner: Rc<ConnectionData<T, V>>,
}

struct CloseData {
	flag: bool,
	port: u16,
	lock: Lock,
	socket: Socket,
	recv: Receiver<()>,
	send: Sender<()>,
}

#[derive(Clone)]
pub struct Evh<T, V>
where
	T: Clone,
	V: Clone,
{
	multiplex: Multiplex,
	close: Rc<CloseData>,
	_phantom_data: PhantomData<(T, V)>,
}

impl<T, V> Connection<T, V>
where
	T: Clone,
	V: Clone,
{
	pub fn acceptor(
		socket: Socket,
		on_recv: Rc<OnRecv<T, V>>,
		on_accept: Rc<OnAccept<T, V>>,
		on_close: Rc<OnClose<T, V>>,
		attach: T,
	) -> Result<Self> {
		let inner = Rc::new(ConnectionData::Acceptor(AcceptorData {
			socket,
			on_recv,
			on_accept,
			on_close,
			attach,
		}))?;

		Ok(Self { inner })
	}

	pub fn outbound(
		socket: Socket,
		on_recv: Rc<OnRecv<T, V>>,
		on_close: Rc<OnClose<T, V>>,
		attach: T,
	) -> Result<Self> {
		let inner = Rc::new(ConnectionData::Outbound(OutboundData {
			socket,
			on_recv,
			on_close,
			lock: lock!(),
			is_closed: false,
			attach,
			opt: None,
		}))?;

		Ok(Self { inner })
	}

	pub fn socket(&self) -> Socket {
		match &*self.inner {
			ConnectionData::Acceptor(x) => x.socket,
			ConnectionData::Outbound(x) => x.socket,
			ConnectionData::Inbound(x) => x.socket,
			ConnectionData::Close => Socket::new(),
		}
	}

	pub fn write(&self, b: &[u8]) -> Result<usize> {
		let _l = self.rlock()?;
		if self.is_closed()? {
			return err!(SocketClosed);
		}
		let res = match &*self.inner {
			ConnectionData::Inbound(inbound) => {
				let res = match inbound.socket.send(b) {
					Ok(res) => Ok(res),
					Err(e) => {
						if e != EAgain {
							let _ = inbound.socket.shutdown();
						}
						Err(e)
					}
				};
				res
			}
			ConnectionData::Outbound(outbound) => {
				let res = match outbound.socket.send(b) {
					Ok(res) => Ok(res),
					Err(e) => {
						if e != EAgain {
							let _ = outbound.socket.shutdown();
						}
						Err(e)
					}
				};
				res
			}
			_ => err!(IllegalState),
		};

		res
	}

	pub fn close(&self) -> Result<()> {
		let _l = self.rlock()?;
		if self.is_closed()? {
			return err!(SocketClosed);
		}
		match &*self.inner {
			ConnectionData::Inbound(inbound) => inbound.socket.shutdown(),
			ConnectionData::Outbound(outbound) => outbound.socket.shutdown(),
			_ => err!(IllegalState),
		}
	}

	pub fn opt(&mut self) -> Result<Option<&mut V>> {
		match &mut *self.inner {
			ConnectionData::Inbound(conn) => Ok(conn.opt.as_mut()),
			ConnectionData::Outbound(conn) => Ok(conn.opt.as_mut()),
			_ => err!(IllegalState),
		}
	}

	pub fn set_opt(&mut self, v: V) -> Result<()> {
		match &mut *self.inner {
			ConnectionData::Inbound(conn) => conn.opt = Some(v),
			ConnectionData::Outbound(conn) => conn.opt = Some(v),
			_ => return err!(IllegalState),
		}

		Ok(())
	}

	pub unsafe fn drop_rc(&mut self) {
		self.inner.set_to_drop();
	}

	fn from_inner(inner: Rc<ConnectionData<T, V>>) -> Self {
		Self { inner }
	}

	fn inbound(socket: Socket, acceptor: Connection<T, V>, multiplex: Multiplex) -> Result<Self> {
		Ok(Self {
			inner: Rc::new(ConnectionData::Inbound(InboundData {
				socket,
				acceptor,
				is_closed: false,
				lock: lock!(),
				multiplex,
				opt: None,
			}))?,
		})
	}

	fn get_acceptor(&mut self) -> Result<&mut Connection<T, V>> {
		match &mut *self.inner {
			ConnectionData::Acceptor(_) => err!(IllegalState),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(x) => Ok(&mut x.acceptor),
			ConnectionData::Close => err!(IllegalState),
		}
	}

	fn attach(&mut self) -> Result<&mut T> {
		match &mut *self.inner {
			ConnectionData::Acceptor(x) => Ok(&mut x.attach),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
			ConnectionData::Close => err!(IllegalState),
		}
	}

	fn on_recv(&mut self, conn: &mut Connection<T, V>, b: &[u8]) -> Result<()> {
		match &mut *self.inner {
			ConnectionData::Acceptor(acc) => (acc.on_recv)(&mut acc.attach, conn, b),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
			ConnectionData::Close => err!(IllegalState),
		}
	}

	fn on_close(&mut self, conn: &Connection<T, V>) -> Result<()> {
		match &mut *self.inner {
			ConnectionData::Acceptor(acc) => (acc.on_close)(&mut acc.attach, conn),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
			ConnectionData::Close => err!(IllegalState),
		}
	}

	fn rlock(&self) -> Result<LockReadGuard<'_>> {
		match &*self.inner {
			ConnectionData::Outbound(x) => {
				let l = x.lock.read();
				Ok(l)
			}
			ConnectionData::Inbound(x) => {
				let l = x.lock.read();
				Ok(l)
			}
			_ => err!(IllegalState),
		}
	}

	fn wlock(&self) -> Result<LockWriteGuard<'_>> {
		match &*self.inner {
			ConnectionData::Outbound(x) => {
				let l = x.lock.write();
				Ok(l)
			}
			ConnectionData::Inbound(x) => {
				let l = x.lock.write();
				Ok(l)
			}
			_ => err!(IllegalState),
		}
	}

	fn is_closed(&self) -> Result<bool> {
		match &*self.inner {
			ConnectionData::Outbound(x) => Ok(x.is_closed),
			ConnectionData::Inbound(x) => Ok(x.is_closed),
			_ => err!(IllegalState),
		}
	}

	fn close_impl(&mut self) -> Result<()> {
		match &mut *self.inner {
			ConnectionData::Outbound(x) => {
				let _l = x.lock.write();
				x.is_closed = true;
				Ok(())
			}
			ConnectionData::Inbound(x) => {
				let _l = x.lock.write();
				x.is_closed = true;
				Ok(())
			}
			_ => err!(IllegalState),
		}
	}
}

impl<T, V> Evh<T, V>
where
	T: Clone,
	V: Clone,
{
	pub fn new() -> Result<Self> {
		let (port, socket) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let multiplex = Multiplex::new()?;
		let (send, recv) = channel()?;
		let lock = lock!();
		let flag = false;
		let close = Rc::new(CloseData {
			flag,
			port,
			lock,
			socket,
			send,
			recv,
		})?;

		let inner = Rc::new(ConnectionData::<T, V>::Close)?;
		Self::try_register(multiplex, socket, unsafe { inner.into_raw() })?;
		Ok(Self {
			multiplex,
			close,
			_phantom_data: PhantomData,
		})
	}

	pub fn register(&mut self, conn: Connection<T, V>) -> Result<()> {
		let inner_clone = conn.inner.clone();

		match &*conn.inner {
			ConnectionData::Acceptor(c) => {
				Self::try_register(self.multiplex, c.socket, unsafe { inner_clone.into_raw() })
			}
			ConnectionData::Outbound(c) => {
				Self::try_register(self.multiplex, c.socket, unsafe { inner_clone.into_raw() })
			}
			_ => err!(IllegalArgument),
		}
	}

	pub fn stop(&mut self) -> Result<()> {
		self.close.flag = true;
		let mut client = Socket::connect([127, 0, 0, 1], self.close.port)?;
		client.close()?;
		self.close.recv.recv()
	}

	pub fn start(&mut self) -> Result<()> {
		let multiplex = self.multiplex;
		let mut close = self.close.clone();
		spawn(move || {
			let mut events = [Event::new(); EVH_MAX_EVENTS];
			let mut do_exit = false;
			while !do_exit {
				let count = match multiplex.wait(&mut events, None) {
					Ok(count) => count,
					Err(e) => {
						println!(
							"FATAL: unexpected error in multiplex.wait(): {}. Halting!",
							e
						);
						break;
					}
				};
				for i in 0..count {
					if i < events.len() && events[i].is_read() {
						match Self::proc_read(events[i], multiplex, &mut close) {
							Ok(exit) => {
								if exit {
									do_exit = true;
									break;
								}
							}
							Err(e) => {
								println!("FATAL: unexpected error in proc_read(): {}. Halting!", e);
								break;
							}
						}
					}
				}
			}
			let _ = multiplex.close();
			let _ = close.send.send(());
		})?;

		Ok(())
	}

	fn proc_read(evt: Event, multiplex: Multiplex, close: &mut Rc<CloseData>) -> Result<bool> {
		let mut inner: Rc<ConnectionData<T, V>> =
			unsafe { Rc::from_raw(Ptr::new(evt.attachment() as *const ConnectionData<T, V>)) };
		match &*inner {
			ConnectionData::Close => {
				if close.flag {
					let _ = close.socket.close();
					return Ok(true);
				}
				loop {
					match close.socket.accept() {
						Ok(mut s) => {
							let _ = s.close();
						}
						Err(e) => {
							if e == EAgain {
								break;
							}
						}
					};
				}

				forget(inner);
				return Ok(false);
			}
			_ => {}
		}
		let conn = Connection::from_inner(inner.clone());
		let drop = match &mut *inner {
			ConnectionData::Acceptor(_) => Self::proc_accept(conn, multiplex)?,
			ConnectionData::Outbound(_) => Self::proc_recv(conn)?,
			ConnectionData::Inbound(_) => Self::proc_recv(conn)?,
			ConnectionData::Close => false,
		};
		// we don't want to drop the rc now unless the connection closed
		if !drop {
			forget(inner);
		}

		Ok(false)
	}

	fn try_register(
		multiplex: Multiplex,
		socket: Socket,
		ptr: Ptr<ConnectionData<T, V>>,
	) -> Result<()> {
		match multiplex.register(socket, RegisterType::Read, Some(ptr.raw() as *const u8)) {
			Ok(_) => Ok(()),
			Err(e) => {
				// if register fails, we must free the Rc.
				let rc: Rc<ConnectionData<T, V>> = unsafe { Rc::from_raw(ptr) };
				println!(
					"WARN: failed to register socket: {} with multiplex: {} due to {}",
					socket, multiplex, e
				);
				Err(e)
			}
		}
	}

	fn proc_recv(mut conn: Connection<T, V>) -> Result<bool> {
		let mut bytes = [0u8; EVH_MAX_BYTES_PER_READ];
		let mut socket = conn.socket();
		loop {
			let len = match socket.recv(&mut bytes) {
				Ok(len) => len,
				Err(e) => {
					match e == EAgain {
						true => return Ok(false),
						false => 0, // close on other errors
					}
				}
			};
			let mut conn_clone = conn.clone();
			match &mut *conn.inner {
				ConnectionData::Inbound(_) => {
					if len == 0 {
						conn_clone.close_impl()?;
						let acc = conn_clone.get_acceptor()?;
						match acc.on_close(&conn) {
							Ok(_) => {}
							Err(e) => println!("WARN: on_close closure generated error: {}", e),
						}
						let _ = socket.close();
						return Ok(true);
					} else {
						let acc = conn_clone.get_acceptor()?;
						if len <= bytes.len() {
							match acc.on_recv(&mut conn, &bytes[0..len]) {
								Ok(_) => {}
								Err(e) => println!("WARN: on_recv closure generated error: {}", e),
							}
						}
					}
				}
				ConnectionData::Outbound(ob) => {
					if len == 0 {
						conn_clone.close_impl()?;
						match (ob.on_close)(&mut ob.attach, &mut conn_clone) {
							Ok(_) => {}
							Err(e) => println!("WARN: on_close closure generated error: {}", e),
						}
						let _ = socket.close();
						return Ok(true);
					} else {
						if len <= bytes.len() {
							match (ob.on_recv)(&mut ob.attach, &mut conn_clone, &bytes[0..len]) {
								Ok(_) => {}
								Err(e) => println!("WARN: on_recv closure generated error: {}", e),
							}
						}
					}
				}
				_ => return err!(IllegalState),
			}
		}
	}

	fn proc_accept(mut conn: Connection<T, V>, multiplex: Multiplex) -> Result<bool> {
		let mut acc = conn.socket();
		loop {
			let mut nsock = match acc.accept() {
				Ok(s) => s,
				Err(e) => {
					if e != EAgain {
						// if there's an error we still keep acceptor open
						// because the error could have just been with the
						// acceptence of this connection, but we print a
						// warning
						println!("WARN: error while trying to accept a connection: {}", e);
						return Ok(false);
					} else {
						// we return here because no more can be accepted now
						// but keep acceptor open
						return Ok(false);
					}
				}
			};
			let nconn = match Connection::inbound(nsock, conn.clone(), multiplex) {
				Ok(nconn) => nconn,
				Err(e) => {
					println!(
						"WARN: Could not create inbound connection structure due to error: {}",
						e
					);
					// drop connection
					let _ = nsock.close();
					continue;
				}
			};

			match Self::try_register(multiplex, nsock, unsafe { nconn.inner.clone().into_raw() }) {
				Ok(_) => {}
				Err(_e) => {
					// WARN already printed and raw pointer dropped, just drop connection here
					let _ = nsock.close();
					continue;
				}
			}

			match &mut *conn.inner {
				ConnectionData::Acceptor(acc) => match (acc.on_accept)(&mut acc.attach, &nconn) {
					Ok(_) => {}
					Err(e) => println!("WARN: on_accept closure generated error: {}", e),
				},
				_ => {
					println!("WARN: unexpected state, trying to accept on a non acceptor!");
					acc.close()?;
					return Ok(true);
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_evh1() -> Result<()> {
		let mut evh: Evh<u64, u64> = Evh::new()?;
		let lock = lock_box!()?;
		let lock_clone = lock.clone();
		let lock_clone2 = lock.clone();
		let lock_clone3 = lock.clone();
		let mut count = Rc::new(0u64)?;
		let count_clone = count.clone();

		let mut acc_count = Rc::new(0u64)?;
		let mut close_count = Rc::new(0u64)?;

		let acc_count_clone = acc_count.clone();
		let close_count_clone = close_count.clone();

		let (port, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
				let _l = lock_clone.write();
				*count += 1;
				Ok(())
			},
		)?;
		let accept: OnAccept<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> {
				let _l = lock_clone2.write();
				*acc_count += 1;
				Ok(())
			},
		)?;
		let close: OnClose<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> {
				let _l = lock_clone3.write();
				*close_count += 1;
				Ok(())
			},
		)?;

		let rc_close = Rc::new(close)?;
		let rc_accept = Rc::new(accept)?;
		let rc_recv = Rc::new(recv)?;

		let mut server = Connection::acceptor(s, rc_recv, rc_accept, rc_close, 0u64)?;
		evh.register(server.clone())?;

		evh.start()?;
		sleep(1); // 1ms sleep to prevent intermittent connect issues.

		let mut client = Socket::connect([127, 0, 0, 1], port)?;

		loop {
			match client.send(b"test") {
				Ok(v) => {
					assert_eq!(v, 4);
					break;
				}
				Err(e) => {
					if e == EAgain {
						continue;
					} else {
						return err!(e);
					}
				}
			}
		}

		loop {
			{
				let _l = lock.read();
				if *count_clone == 1 && *acc_count_clone == 1 && *close_count_clone == 0 {
					break;
				}
			}
			sleep(1);
		}

		client.close()?;

		loop {
			{
				let _l = lock.read();
				if *count_clone == 1 && *acc_count_clone == 1 && *close_count_clone == 1 {
					break;
				}
			}
			sleep(1);
		}

		evh.stop()?;
		s.close()?;
		// just to make address sanitizer report no memory leaks - normal case server just
		// runs forever.
		unsafe {
			server.drop_rc();
		}

		Ok(())
	}

	#[test]
	fn test_evh_reply() -> Result<()> {
		let mut evh: Evh<u64, u64> = Evh::new()?;
		let lock = lock_box!()?;
		let lock_clone = lock.clone();
		let count = Rc::new(0)?;
		let mut count_clone = count.clone();

		let (port, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
				let len = loop {
					match conn.write(bytes) {
						Ok(len) => break len,
						Err(e) => assert_eq!(e, EAgain),
					}
				};
				assert_eq!(len, 6);

				Ok(())
			},
		)?;
		let accept: OnAccept<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> { Ok(()) },
		)?;
		let close: OnClose<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> {
				let _l = lock_clone.write();
				*count_clone += 1;
				Ok(())
			},
		)?;

		let rc_close = Rc::new(close)?;
		let rc_accept = Rc::new(accept)?;
		let rc_recv = Rc::new(recv)?;

		let mut server = Connection::acceptor(s, rc_recv, rc_accept, rc_close, 0u64)?;
		evh.register(server.clone())?;

		evh.start()?;
		sleep(1); // 1ms sleep to prevent intermittent connect issues.

		let mut client = Socket::connect([127, 0, 0, 1], port)?;

		loop {
			match client.send(b"test37") {
				Ok(v) => {
					assert_eq!(v, 6);
					break;
				}
				Err(e) => {
					if e == EAgain {
						continue;
					} else {
						return err!(e);
					}
				}
			}
		}

		let mut buf = [0u8; 10];
		let len = loop {
			match client.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e, EAgain),
			}
		};
		assert_eq!(len, 6);
		assert_eq!(buf[0], b't');
		assert_eq!(buf[1], b'e');
		assert_eq!(buf[2], b's');
		assert_eq!(buf[3], b't');
		assert_eq!(buf[4], b'3');
		assert_eq!(buf[5], b'7');

		client.close()?;

		loop {
			{
				let _l = lock.read();
				if *count == 1 {
					break;
				}
			}
			sleep(1);
		}

		evh.stop()?;
		s.close()?;
		// just to make address sanitizer report no memory leaks - normal case server just
		// runs forever.
		unsafe {
			server.drop_rc();
		}

		Ok(())
	}

	#[test]
	fn test_evh_close() -> Result<()> {
		let mut evh: Evh<u64, u64> = Evh::new()?;
		let lock = lock_box!()?;
		let lock_clone = lock.clone();
		let count = Rc::new(0)?;
		let mut count_clone = count.clone();

		let (port, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
				let len = loop {
					match conn.write(bytes) {
						Ok(len) => break len,
						Err(e) => assert_eq!(e, EAgain),
					}
				};

				if len > 0 && bytes[0] == b'x' {
					conn.close()?;
				}

				Ok(())
			},
		)?;
		let accept: OnAccept<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> { Ok(()) },
		)?;
		let close: OnClose<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> {
				assert!(conn.write(b"test").is_err());
				let _l = lock_clone.write();
				*count_clone += 1;
				Ok(())
			},
		)?;

		let rc_close = Rc::new(close)?;
		let rc_accept = Rc::new(accept)?;
		let rc_recv = Rc::new(recv)?;

		let mut server = Connection::acceptor(s, rc_recv, rc_accept, rc_close, 0u64)?;
		evh.register(server.clone())?;

		evh.start()?;

		sleep(1); // 1ms sleep to prevent intermittent connect issues.

		let mut client = Socket::connect([127, 0, 0, 1], port)?;

		loop {
			match client.send(b"test37") {
				Ok(v) => {
					assert_eq!(v, 6);
					break;
				}
				Err(e) => {
					if e == EAgain {
						continue;
					} else {
						return err!(e);
					}
				}
			}
		}

		let mut buf = [0u8; 10];
		let len = loop {
			match client.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e, EAgain),
			}
		};
		assert_eq!(len, 6);
		assert_eq!(buf[0], b't');
		assert_eq!(buf[1], b'e');
		assert_eq!(buf[2], b's');
		assert_eq!(buf[3], b't');
		assert_eq!(buf[4], b'3');
		assert_eq!(buf[5], b'7');

		loop {
			match client.send(b"x") {
				Ok(v) => {
					assert_eq!(v, 1);
					break;
				}
				Err(e) => {
					if e == EAgain {
						continue;
					} else {
						return err!(e);
					}
				}
			}
		}

		loop {
			{
				let _l = lock.read();
				if *count == 1 {
					break;
				}
			}
			sleep(1);
		}

		client.close()?;
		evh.stop()?;
		s.close()?;
		// just to make address sanitizer report no memory leaks - normal case server just
		// runs forever.
		unsafe {
			server.drop_rc();
		}

		Ok(())
	}

	#[test]
	fn test_evh2servers() -> Result<()> {
		let mut evh: Evh<u64, u64> = Evh::new()?;

		let lock = lock_box!()?;
		let count = Rc::new(0)?;

		let lock_clone = lock.clone();
		let mut count_clone = count.clone();

		let (port1, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
				let _l = lock_clone.write();
				*count_clone += *attach;
				Ok(())
			},
		)?;
		let accept: OnAccept<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> { Ok(()) },
		)?;
		let close: OnClose<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> { Ok(()) },
		)?;

		let rc_close = Rc::new(close)?;
		let rc_accept = Rc::new(accept)?;
		let rc_recv = Rc::new(recv)?;

		let rc_close2 = rc_close.clone();
		let rc_accept2 = rc_accept.clone();
		let rc_recv2 = rc_recv.clone();

		let mut server = Connection::acceptor(s, rc_recv, rc_accept, rc_close, 3u64)?;

		evh.register(server.clone())?;

		let (port2, s2) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let mut server2 = Connection::acceptor(s2, rc_recv2, rc_accept2, rc_close2, 7u64)?;
		evh.register(server2.clone())?;

		evh.start()?;

		{
			let _l = lock.read();
			assert_eq!(*count, 0);
		}

		let mut client1 = Socket::connect([127, 0, 0, 1], port1)?;
		loop {
			match client1.send(b"hi") {
				Ok(_) => break,
				Err(e) => assert_eq!(e, EAgain),
			}
		}
		client1.close()?;

		let mut client2 = Socket::connect([127, 0, 0, 1], port2)?;

		loop {
			match client2.send(b"hi") {
				Ok(_) => break,
				Err(e) => assert_eq!(e, EAgain),
			}
		}
		client2.close()?;

		loop {
			sleep(1);
			let _l = lock.read();
			if *count == 10 {
				break;
			}
		}

		evh.stop()?;
		s.close()?;
		// just to make address sanitizer report no memory leaks - normal case server just
		// runs forever.
		unsafe {
			server.drop_rc();
			server2.drop_rc();
		}

		Ok(())
	}

	#[test]
	fn test_evh_client_reg() -> Result<()> {
		let mut evh: Evh<u64, u64> = Evh::new()?;
		let lock = lock_box!()?;
		let lock_clone = lock.clone();
		let count = Rc::new(0)?;
		let mut count_clone = count.clone();

		let (port, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
				let len = loop {
					match conn.write(bytes) {
						Ok(len) => break len,
						Err(e) => assert_eq!(e, EAgain),
					}
				};
				assert_eq!(len, 6);

				Ok(())
			},
		)?;
		let accept: OnAccept<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> { Ok(()) },
		)?;
		let close: OnClose<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> { Ok(()) },
		)?;

		let rc_close = Rc::new(close)?;
		let rc_accept = Rc::new(accept)?;
		let rc_recv = Rc::new(recv)?;

		let mut server = Connection::acceptor(s, rc_recv, rc_accept, rc_close, 0u64)?;
		evh.register(server.clone())?;

		let mut client = Socket::connect([127, 0, 0, 1], port)?;

		let recv_client: OnRecv<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
				assert_eq!(bytes.len(), 6);
				assert_eq!(bytes[0], b't');
				assert_eq!(bytes[1], b'e');
				assert_eq!(bytes[2], b's');
				assert_eq!(bytes[3], b't');
				assert_eq!(bytes[4], b'5');
				assert_eq!(bytes[5], b'7');
				let _l = lock_clone.write();
				*count_clone += 1;
				Ok(())
			},
		)?;
		let close_client: OnClose<u64, u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64, u64>| -> Result<()> { Ok(()) },
		)?;

		let rc_recv_client = Rc::new(recv_client)?;
		let rc_close_client = Rc::new(close_client)?;

		let mut connector = Connection::outbound(client, rc_recv_client, rc_close_client, 1u64)?;
		evh.register(connector.clone())?;

		evh.start()?;

		loop {
			match client.send(b"test57") {
				Ok(v) => {
					assert_eq!(v, 6);
					break;
				}
				Err(e) => {
					if e == EAgain {
						continue;
					} else {
						return err!(e);
					}
				}
			}
		}

		loop {
			sleep(1);
			let _l = lock.read();
			if *count == 1 {
				break;
			}
		}

		client.close()?;
		evh.stop()?;
		s.close()?;

		// just to make address sanitizer report no memory leaks - normal case server just
		// runs forever.
		unsafe {
			server.drop_rc();
			connector.drop_rc();
		}

		Ok(())
	}
}
