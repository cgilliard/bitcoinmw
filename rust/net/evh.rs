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

type OnRecv<T> = Box<dyn FnMut(&mut T, &mut Connection<T>, &[u8]) -> Result<()>>;
type OnAccept<T> = Box<dyn FnMut(&mut T, &Connection<T>) -> Result<()>>;
type OnClose<T> = Box<dyn FnMut(&mut T, &Connection<T>) -> Result<()>>;
type OnWritable<T> = Box<dyn FnMut(&mut Connection<T>) -> Result<()>>;

struct AcceptorData<T>
where
	T: Clone,
{
	socket: Socket,
	on_recv: Rc<OnRecv<T>>,
	on_accept: Rc<OnAccept<T>>,
	on_close: Rc<OnClose<T>>,
	attach: T,
}

struct InboundData<T>
where
	T: Clone,
{
	socket: Socket,
	acceptor: Connection<T>,
	is_closed: bool,
	lock: Lock,
	multiplex: Multiplex,
	on_writable: Option<OnWritable<T>>,
}

struct OutboundData<T>
where
	T: Clone,
{
	socket: Socket,
	on_recv: Rc<OnRecv<T>>,
	on_close: Rc<OnClose<T>>,
	attach: T,
}

enum ConnectionData<T>
where
	T: Clone,
{
	Inbound(InboundData<T>),
	Outbound(OutboundData<T>),
	Acceptor(AcceptorData<T>),
	Close,
}

#[derive(Clone)]
pub struct Connection<T>
where
	T: Clone,
{
	inner: Rc<ConnectionData<T>>,
}

struct CloseData {
	flag: bool,
	port: u16,
	lock: Lock,
	socket: Socket,
	recv: Receiver<()>,
	send: Sender<()>,
}

pub struct Evh<T>
where
	T: Clone,
{
	multiplex: Multiplex,
	close: Rc<CloseData>,
	_phantom_data: PhantomData<T>,
}

impl<T> Connection<T>
where
	T: Clone,
{
	pub fn acceptor(
		socket: Socket,
		on_recv: Rc<OnRecv<T>>,
		on_accept: Rc<OnAccept<T>>,
		on_close: Rc<OnClose<T>>,
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
		on_recv: Rc<OnRecv<T>>,
		on_close: Rc<OnClose<T>>,
		attach: T,
	) -> Result<Self> {
		let inner = Rc::new(ConnectionData::Outbound(OutboundData {
			socket,
			on_recv,
			on_close,
			attach,
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
		match &*self.inner {
			ConnectionData::Inbound(inbound) => inbound.socket.send(b),
			ConnectionData::Outbound(outbound) => outbound.socket.send(b),
			_ => err!(IllegalState),
		}
	}

	pub fn close(&self) -> Result<()> {
		match &*self.inner {
			ConnectionData::Inbound(inbound) => inbound.socket.shutdown(),
			ConnectionData::Outbound(outbound) => outbound.socket.shutdown(),
			_ => err!(IllegalState),
		}
	}

	pub fn on_writable(&mut self, on_writable: OnWritable<T>) -> Result<()> {
		let socket = self.socket();
		let ptr = unsafe { self.inner.clone().into_raw().raw() };
		match &mut *self.inner {
			ConnectionData::Inbound(inbound) => {
				Self::do_on_writable(socket, inbound, on_writable, ptr)
			}
			// TODO: add support for on writable for outbound connections
			/*
			ConnectionData::Outbound(outbound) => {
				Self::do_on_writable(socket, outbound, on_writable, ptr)
			}
						*/
			_ => {
				// drop rc to avoid memory leak
				let rc: Rc<ConnectionData<T>> =
					unsafe { Rc::from_raw(Ptr::new(ptr as *const ConnectionData<T>)) };
				err!(IllegalState)
			}
		}
	}

	pub unsafe fn drop_rc(&mut self) {
		self.inner.set_to_drop();
	}

	fn do_on_writable(
		socket: Socket,
		inbound: &mut InboundData<T>,
		on_writable: OnWritable<T>,
		ptr: *const ConnectionData<T>,
	) -> Result<()> {
		inbound.on_writable = Some(on_writable);
		Evh::try_register(inbound.multiplex, socket, RegisterType::RW, Ptr::new(ptr))
	}

	fn from_inner(inner: Rc<ConnectionData<T>>) -> Self {
		Self { inner }
	}

	fn inbound(socket: Socket, acceptor: Connection<T>, multiplex: Multiplex) -> Result<Self> {
		Ok(Self {
			inner: Rc::new(ConnectionData::Inbound(InboundData {
				socket,
				acceptor,
				is_closed: false,
				lock: lock!(),
				multiplex,
				on_writable: None,
			}))?,
		})
	}

	fn get_acceptor(&mut self) -> Result<&mut Connection<T>> {
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

	fn on_recv(&mut self, conn: &mut Connection<T>, b: &[u8]) -> Result<()> {
		match &mut *self.inner {
			ConnectionData::Acceptor(acc) => (acc.on_recv)(&mut acc.attach, conn, b),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
			ConnectionData::Close => err!(IllegalState),
		}
	}

	fn on_close(&mut self, conn: &Connection<T>) -> Result<()> {
		match &mut *self.inner {
			ConnectionData::Acceptor(acc) => (acc.on_close)(&mut acc.attach, conn),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
			ConnectionData::Close => err!(IllegalState),
		}
	}
}

impl<T> Evh<T>
where
	T: Clone,
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

		let inner = Rc::new(ConnectionData::<T>::Close)?;
		Self::try_register(multiplex, socket, RegisterType::Read, unsafe {
			inner.into_raw()
		})?;
		Ok(Self {
			multiplex,
			close,
			_phantom_data: PhantomData,
		})
	}

	pub fn register(&mut self, conn: Connection<T>) -> Result<()> {
		let inner_clone = conn.inner.clone();

		match &*conn.inner {
			ConnectionData::Acceptor(c) => {
				Self::try_register(self.multiplex, c.socket, RegisterType::Read, unsafe {
					inner_clone.into_raw()
				})
			}
			ConnectionData::Outbound(c) => {
				Self::try_register(self.multiplex, c.socket, RegisterType::Read, unsafe {
					inner_clone.into_raw()
				})
			}
			_ => err!(IllegalArgument),
		}
	}

	pub fn stop(&mut self) -> Result<()> {
		self.close.flag = true;
		let mut client = Socket::connect([127, 0, 0, 1], self.close.port)?;
		client.close()?;
		self.close.recv.recv();
		Ok(())
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
					if events[i].is_read() {
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
					if events[i].is_write() {
						match Self::proc_write(events[i], multiplex) {
							Ok(_) => {}
							Err(e) => {
								println!(
									"FATAL: unexpected error in proc_write(): {}. Halting!",
									e
								);
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
		let mut inner: Rc<ConnectionData<T>> =
			unsafe { Rc::from_raw(Ptr::new(evt.attachment() as *const ConnectionData<T>)) };
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

	fn proc_write(evt: Event, mut multiplex: Multiplex) -> Result<()> {
		let ptr = evt.attachment();
		let mut inner: Rc<ConnectionData<T>> =
			unsafe { Rc::from_raw(Ptr::new(ptr as *const ConnectionData<T>)) };
		let mut conn = Connection::from_inner(inner.clone());
		let socket = conn.socket();
		match &mut *inner {
			ConnectionData::Inbound(ref mut ib) => match &mut ib.on_writable {
				Some(on_writable) => match (on_writable)(&mut conn) {
					Ok(_) => {}
					Err(e) => println!("WARN: on_writable closure generated error: {}", e),
				},
				None => {}
			},
			_ => {}
		}
		forget(inner);

		// TODO: what if user registers here? Have closure return a boolean to signify
		// whether to unregister or not
		multiplex.unregister_write(socket, Some(ptr))
	}

	fn try_register(
		multiplex: Multiplex,
		socket: Socket,
		rt: RegisterType,
		ptr: Ptr<ConnectionData<T>>,
	) -> Result<()> {
		match multiplex.register(socket, rt, Some(ptr.raw() as *const u8)) {
			Ok(_) => Ok(()),
			Err(e) => {
				// if register fails, we must free the Rc.
				let rc: Rc<ConnectionData<T>> = unsafe { Rc::from_raw(ptr) };
				println!(
					"WARN: failed to register socket: {} with multiplex: {} due to {}",
					socket, multiplex, e
				);
				Err(e)
			}
		}
	}

	fn proc_recv(mut conn: Connection<T>) -> Result<bool> {
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
						let acc = conn_clone.get_acceptor()?;
						match acc.on_close(&conn) {
							Ok(_) => {}
							Err(e) => println!("WARN: on_close closure generated error: {}", e),
						}
						let _ = socket.close();
						return Ok(true);
					} else {
						let acc = conn_clone.get_acceptor()?;
						match acc.on_recv(&mut conn, &bytes[0..len]) {
							Ok(_) => {}
							Err(e) => println!("WARN: on_recv closure generated error: {}", e),
						}
					}
				}
				ConnectionData::Outbound(ob) => {
					if len == 0 {
						match (ob.on_close)(&mut ob.attach, &mut conn_clone) {
							Ok(_) => {}
							Err(e) => println!("WARN: on_close closure generated error: {}", e),
						}
						let _ = socket.close();
						return Ok(true);
					} else {
						match (ob.on_recv)(&mut ob.attach, &mut conn_clone, &bytes[0..len]) {
							Ok(_) => {}
							Err(e) => println!("WARN: on_recv closure generated error: {}", e),
						}
					}
				}
				_ => return err!(IllegalState),
			}
		}
	}

	fn proc_accept(mut conn: Connection<T>, multiplex: Multiplex) -> Result<bool> {
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

			match Self::try_register(multiplex, nsock, RegisterType::Read, unsafe {
				nconn.inner.clone().into_raw()
			}) {
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
		let mut evh: Evh<u64> = Evh::new()?;
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
		let recv: OnRecv<u64> = Box::new(
			move |attach: &mut u64, conn: &mut Connection<u64>, bytes: &[u8]| -> Result<()> {
				let _l = lock_clone.write();
				*count += 1;
				Ok(())
			},
		)?;
		let accept: OnAccept<u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64>| -> Result<()> {
				let _l = lock_clone2.write();
				*acc_count += 1;
				Ok(())
			},
		)?;
		let close: OnClose<u64> = Box::new(
			move |attach: &mut u64, conn: &Connection<u64>| -> Result<()> {
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
}
