#![allow(dead_code)]
#![allow(unused_variables)]

use core::marker::PhantomData;
use core::mem::forget;
use core::ops::FnMut;
use net::constants::*;
use net::errors::*;
use net::multiplex::{Event, Multiplex, RegisterType};
use net::socket::Socket;
use prelude::*;

type OnRecv<T> = Box<dyn FnMut(&T, &Connection<T>, &[u8]) -> Result<()>>;
type OnAccept<T> = Box<dyn FnMut(&T, &Connection<T>) -> Result<()>>;
type OnClose<T> = Box<dyn FnMut(&T, &Connection<T>) -> Result<()>>;
type OnWritable<T> = Box<dyn FnMut(&Connection<T>) -> Result<()>>;

struct AcceptorData<T>
where
	T: Clone,
{
	socket: Socket,
	// closure the Evh will call when a connection that was accepted by this acceptor recvs
	// data.
	on_recv: Rc<OnRecv<T>>,
	// closure the Evh will call when the acceptor accepts a new connection.
	on_accept: Rc<OnAccept<T>>,
	// closure the Evh will call when the acceptor closes a connection that was associated with
	// this acceptor.
	on_close: Rc<OnClose<T>>,
	// attachment passed on all requests - used for identifiers and context data.
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
}

struct OutboundData<T>
where
	T: Clone,
{
	socket: Socket,
	_phantom_data: PhantomData<T>,
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

// Main connection type used to create outbound connections and servers.
#[derive(Clone)]
pub struct Connection<T>
where
	T: Clone,
{
	inner: Rc<ConnectionData<T>>,
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
			_ => err!(IllegalState),
		}
	}

	pub fn close(&self) -> Result<()> {
		match &*self.inner {
			ConnectionData::Inbound(inbound) => inbound.socket.shutdown(),
			_ => err!(IllegalState),
		}
	}

	pub fn on_writable(&self, on_write: OnWritable<T>) -> Result<()> {
		Ok(())
	}

	pub unsafe fn drop_rc(&mut self) {
		self.inner.set_to_drop();
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

	fn on_recv(&mut self, conn: &Connection<T>, b: &[u8]) -> Result<()> {
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

pub struct Evh<T>
where
	T: Clone,
{
	multiplex: Multiplex,
	close_port: u16,
	close_flag: Rc<bool>,
	close_lock: LockBox,
	close_socket: Socket,
	_phantom_data: PhantomData<T>,
}

impl<T> Evh<T>
where
	T: Clone,
{
	pub fn new() -> Result<Self> {
		let (close_port, close_socket) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let multiplex = Multiplex::new()?;
		let close_lock = lock_box!()?;
		let close_flag = Rc::new(false)?;
		let inner = Rc::new(ConnectionData::<T>::Close)?;
		let ptr = inner.into_raw();
		multiplex.register(close_socket, RegisterType::Read, Some(ptr))?;
		Ok(Self {
			multiplex,
			close_flag,
			close_port,
			close_lock,
			close_socket,
			_phantom_data: PhantomData,
		})
	}
	pub fn register(&mut self, conn: Connection<T>) -> Result<()> {
		let inner_clone = conn.inner.clone();
		match &*conn.inner {
			ConnectionData::Acceptor(s) => {
				let ptr = inner_clone.into_raw();
				self.multiplex
					.register(s.socket, RegisterType::Read, Some(ptr))?;
				Ok(())
			}
			_ => err!(Todo),
		}
	}

	pub fn stop(&mut self) -> Result<()> {
		*self.close_flag = true;
		let mut client = Socket::connect([127, 0, 0, 1], self.close_port)?;
		client.close()?;
		Ok(())
	}

	pub fn start(&mut self) -> Result<()> {
		let multiplex = self.multiplex;
		let close_flag = self.close_flag.clone();
		let close_socket = self.close_socket.clone();
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
					match Self::proc_event(events[i], multiplex, &close_flag, close_socket) {
						Ok(exit) => {
							if exit {
								do_exit = true;
								break;
							}
						}
						Err(e) => {
							println!("FATAL: unexpected error in proc_event(): {}. Halting!", e);
							break;
						}
					}
				}
			}
			let _ = multiplex.close();
		})?;

		Ok(())
	}

	fn proc_event(
		evt: Event,
		multiplex: Multiplex,
		close_flag: &Rc<bool>,
		mut close_socket: Socket,
	) -> Result<bool> {
		let mut inner: Rc<ConnectionData<T>> = Rc::from_raw(evt.attachment());
		match &*inner {
			ConnectionData::Close => {
				if **close_flag {
					close_socket.close()?;
					return Ok(true);
				}
				loop {
					match close_socket.accept() {
						Ok(mut s) => {
							s.close()?;
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
			ConnectionData::Outbound(_) => false,
			ConnectionData::Inbound(_) => Self::proc_recv(conn)?,
			ConnectionData::Close => false,
		};
		// we don't want to drop the rc now unless the connection closed
		if !drop {
			forget(inner);
		}

		Ok(false)
	}

	fn proc_recv(conn: Connection<T>) -> Result<bool> {
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
			if len == 0 {
				let mut conn_clone = conn.clone();
				let acc = conn_clone.get_acceptor()?;
				acc.on_close(&conn)?;
				let _ = socket.close();
				return Ok(true);
			} else {
				let mut conn_clone = conn.clone();
				let acc = conn_clone.get_acceptor()?;
				acc.on_recv(&conn, bytes.subslice(0, len)?)?;
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
						// if there's an error, close the acceptor and return
						let _ = acc.close();
						return Ok(true);
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
					nsock.close()?;
					continue;
				}
			};
			let ptr = nconn.inner.clone().into_raw();
			match multiplex.register(nsock, RegisterType::Read, Some(ptr)) {
				Ok(_) => {}
				Err(e) => {
					println!(
						"WARN: Could not register inbound connection due to error: {}",
						e
					);
					// drop connection
					nsock.close()?;
					// free rc memory
					let rc: Rc<ConnectionData<T>> = Rc::from_raw(ptr);
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
			move |attach: &u64, conn: &Connection<u64>, bytes: &[u8]| -> Result<()> {
				let _l = lock_clone.write();
				*count += 1;
				Ok(())
			},
		)?;
		let accept: OnAccept<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> {
				let _l = lock_clone2.write();
				*acc_count += 1;
				Ok(())
			})?;
		let close: OnClose<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> {
				let _l = lock_clone3.write();
				*close_count += 1;
				Ok(())
			})?;

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
		let mut evh: Evh<u64> = Evh::new()?;
		let lock = lock_box!()?;
		let lock_clone = lock.clone();
		let count = Rc::new(0)?;
		let mut count_clone = count.clone();

		let (port, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64> = Box::new(
			move |attach: &u64, conn: &Connection<u64>, bytes: &[u8]| -> Result<()> {
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
		let accept: OnAccept<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> { Ok(()) })?;
		let close: OnClose<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> {
				let _l = lock_clone.write();
				*count_clone += 1;
				Ok(())
			})?;

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
		let mut evh: Evh<u64> = Evh::new()?;
		let lock = lock_box!()?;
		let lock_clone = lock.clone();
		let count = Rc::new(0)?;
		let mut count_clone = count.clone();

		let (port, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64> = Box::new(
			move |attach: &u64, conn: &Connection<u64>, bytes: &[u8]| -> Result<()> {
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
		let accept: OnAccept<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> { Ok(()) })?;
		let close: OnClose<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> {
				let _l = lock_clone.write();
				*count_clone += 1;
				Ok(())
			})?;

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
		let mut evh: Evh<u64> = Evh::new()?;

		let lock = lock_box!()?;
		let count = Rc::new(0)?;

		let lock_clone = lock.clone();
		let mut count_clone = count.clone();

		let (port1, mut s) = Socket::listen_rand([127, 0, 0, 1], 10)?;
		let recv: OnRecv<u64> = Box::new(
			move |attach: &u64, conn: &Connection<u64>, bytes: &[u8]| -> Result<()> {
				let _l = lock_clone.write();
				*count_clone += attach;
				Ok(())
			},
		)?;
		let accept: OnAccept<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> { Ok(()) })?;
		let close: OnClose<u64> =
			Box::new(move |attach: &u64, conn: &Connection<u64>| -> Result<()> { Ok(()) })?;

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

	/*

		#[test]
	fn test_show_sample() -> Result<()> {
		let evh = evh!()?;
		let l1 = listen!(Port(9090))?;
		let l2 = listen!(Addr([0,0,0,0]), Port(8080))?;
		let l3 = listen!(Backlog(10), Addr([0,0,0,0]), Port(9091))?;

		let recv = recv!({
			println!("recv[{}][{}][{}]", attach, conn.socket(), data);
		Ok(())
		})?;
		let accept = accept!({
			println!("accept[{}][{}]", attach, conn.socket());
			Ok(())
		})?;
		let close = close!({
			println!("close[{}][{}]", attach, conn.socket());
			Ok(())
		})?;

		let accept2 = accept!({
			println!("accept2[{}][{}]", attach, conn.socket());
			Ok(())
		})?;

		let a1 = acceptor!(Attachment(0u64), Listener(l1), Recv(recv), Accept(accept))?;
		let a2 = acceptor!(Listener(l2), Recv(recv), Accept(accept), Close(close))?;
		let a3 =  acceptor!(Listener(l3), Recv(recv), Accept(accept2))?;
		evh.register(a1)?;
		evh.register(a2)?;
		evh.register(a3)?;
		evh.start()?;
		park();

	}


				 */
}
