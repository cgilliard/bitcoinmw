#![allow(dead_code)]
#![allow(unused_variables)]

use core::marker::PhantomData;
use core::mem::forget;
use net::constants::*;
use net::errors::*;
use net::multiplex::{Event, Multiplex, RegisterType};
use net::socket::Socket;
use prelude::*;

struct AcceptorData<T>
where
	T: Clone,
{
	socket: Socket,
	// closure the Evh will call when a connection that was accepted by this acceptor recvs
	// data.
	on_recv: fn(&mut T, &Connection<T>, &[u8]) -> Result<()>,
	// closure the Evh will call when the acceptor accepts a new connection.
	on_accept: fn(&mut T, &Connection<T>) -> Result<()>,
	// closure the Evh will call when the acceptor closes a connection that was associated with
	// this acceptor.
	on_close: fn(&mut T, &Connection<T>) -> Result<()>,
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
		on_recv: fn(&mut T, &Connection<T>, &[u8]) -> Result<()>,
		on_accept: fn(&mut T, &Connection<T>) -> Result<()>,
		on_close: fn(&mut T, &Connection<T>) -> Result<()>,
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
		}
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
		}
	}

	fn attach(&mut self) -> Result<&mut T> {
		match &mut *self.inner {
			ConnectionData::Acceptor(x) => Ok(&mut x.attach),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
		}
	}

	fn on_close(&self) -> Result<fn(&mut T, &Connection<T>) -> Result<()>> {
		match &*self.inner {
			ConnectionData::Acceptor(x) => Ok(x.on_close),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
		}
	}

	fn on_recv(&self) -> Result<fn(&mut T, &Connection<T>, &[u8]) -> Result<()>> {
		match &*self.inner {
			ConnectionData::Acceptor(x) => Ok(x.on_recv),
			ConnectionData::Outbound(_) => err!(IllegalState),
			ConnectionData::Inbound(_) => err!(IllegalState),
		}
	}
}

pub struct Evh<T>
where
	T: Clone,
{
	multiplex: Multiplex,
	_phantom_data: PhantomData<T>,
}

impl<T> Drop for Evh<T>
where
	T: Clone,
{
	fn drop(&mut self) {
		let _ = self.stop();
	}
}

impl<T> Evh<T>
where
	T: Clone,
{
	pub fn new() -> Result<Self> {
		let multiplex = Multiplex::new()?;
		Ok(Self {
			multiplex,
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
		Ok(())
	}

	pub fn start(&mut self) -> Result<()> {
		let multiplex = self.multiplex;
		spawn(move || {
			let mut events = [Event::new(); EVH_MAX_EVENTS];
			loop {
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
					match Self::proc_event(events[i], multiplex) {
						Ok(_) => {}
						Err(e) => {
							println!("FATAL: unexpected error in proc_event(): {}. Halting!", e);
							break;
						}
					}
				}
			}
		})?;

		Ok(())
	}

	fn proc_event(evt: Event, multiplex: Multiplex) -> Result<()> {
		let mut inner: Rc<ConnectionData<T>> = Rc::from_raw(evt.attachment());
		let conn = Connection::from_inner(inner.clone());
		let drop = match &mut *inner {
			ConnectionData::Acceptor(_) => Self::proc_accept(conn, multiplex)?,
			ConnectionData::Outbound(_) => false,
			ConnectionData::Inbound(_) => Self::proc_recv(conn)?,
		};
		// we don't want to drop the rc now unless the connection closed
		if !drop {
			forget(inner);
		}

		Ok(())
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
			if len == 0 {
				let conn_clone = conn.clone();
				let acc = conn.get_acceptor()?;
				let closure = acc.on_close()?.clone();
				let attach = acc.attach()?;
				match closure(attach, &conn_clone) {
					Ok(_) => {}
					Err(e) => println!("WARN: on_close closure generated error: {}", e),
				}
				let _ = socket.close();
				return Ok(true);
			} else {
				let conn_clone = conn.clone();
				let acc = conn.get_acceptor()?;
				let closure = acc.on_recv()?.clone();
				let attach = acc.attach()?;
				match closure(attach, &conn_clone, bytes.subslice(0, len)?) {
					Ok(_) => {}
					Err(e) => println!("WARN: on_recv closure generated error: {}", e),
				}
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

/*
#[cfg(test)]
mod test {
	use super::*;
	use core::mem::size_of;

	#[test]
	fn test_evh1() -> Result<()> {
		println!(
			"sz InboundData = {}, sz OutboundData = {}, sz AcceptorData = {}, sz Connection = {}",
			size_of::<InboundData<u64>>(),
			size_of::<OutboundData<u64>>(),
			size_of::<AcceptorData<u64>>(),
			size_of::<ConnectionData<u64>>()
		);

		let mut evh = Evh::new()?;
		let lock = lock_box!()?;
		let count = Rc::new(064)?;

		let port = 9900;
		let mut s = Socket::listen([127, 0, 0, 1], port, 10)?;
		let recv = |attach: &mut u64, conn: &Connection<u64>, bytes: &[u8]| -> Result<()> {
			unsafe {
				use core::str::from_utf8_unchecked;
				let s = from_utf8_unchecked(bytes);
				println!("handler_recv[{}][{}]: '{}'", attach, conn.socket(), s);
			}
			//let _l = lock.write();
			// *count += 1;
			Ok(())
		};
		let accept = |attach: &mut u64, conn: &Connection<u64>| -> Result<()> {
			println!("handler_accept[{}][{}]", attach, conn.socket());
			Ok(())
		};
		let close = |attach: &mut u64, conn: &Connection<u64>| -> Result<()> {
			println!("handler_close[{}][{}]", attach, conn.socket());
			Ok(())
		};

		let server = Connection::acceptor(s, recv, accept, close, 0u64)?;
		evh.register(server.clone())?;

		let port = 9901;
		let mut s2 = Socket::listen([0, 0, 0, 0], port, 10)?;
		let server2 = Connection::acceptor(s2, recv, accept, close, 1u64)?;
		evh.register(server2.clone())?;

		evh.start()?;

		park();

		Ok(())
	}
}
*/
