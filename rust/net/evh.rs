#![allow(dead_code)]
use core::marker::PhantomData;
use core::mem::forget;
use net::constants::*;
use net::errors::*;
use net::multiplex::{Event, Multiplex, RegisterType};
use net::socket::Socket;
use prelude::*;
//use std::ffi::sleep_millis;

// Goal: To create a lightweight abstraction over my Multiplex/Socket APIs
// to allow for simple and fast client/server communication.
// The Evh (Event Handler) mostly is just the traffic cop, letting the caller
// know when an i/o event occurs. Most allocation is not done in the Evh itself (perhaps none?)
// The only place where the Evh may need to allocate memory is when accepting a new connection.
// However, we might do this in a callback and thus completely eliminate memory allocation..

// Write Handle inner type: just the socket and a closed flag. Lock used for protecting state.
pub struct WriteHandle {
	socket: Socket,
	is_closed: bool,
	lock: Lock,
	multiplex: Multiplex,
}

impl WriteHandle {
	// create a new WriteHandle for the specified socket.
	fn new(socket: Socket) -> Self {
		Self {
			socket,
			is_closed: false,
			lock: lock!(),
			multiplex: Multiplex::uninit(),
		}
	}

	// write to the socket. This method does not block and returns the amount of bytes written
	// or an error.
	pub fn write(&self, _buf: &[u8]) -> Result<u64> {
		err!(Todo)
	}

	// Close the underlying socket and mark this WriteHandle as closed
	pub fn close(&self) -> Result<()> {
		err!(Todo)
	}

	fn set_multiplex(&mut self, multiplex: Multiplex) {
		self.multiplex = multiplex;
	}
}

// internal representation for Inbound connection (one that is accepted by another connection (i.e.
// a listener)).
struct InboundData<T: Clone> {
	socket: Socket,            // socket
	write_handle: WriteHandle, // write handle associated with this connection
	acceptor: Connection<T>,   // the connection that accepted this inbound connection
}

struct OutboundData<T: Clone> {
	// socket
	socket: Socket,
	// closure the Evh will call when this outbound connection reads data
	on_recv: fn(&mut T, Connection<T>, &[u8]) -> Result<()>,
	// write handle to use to write data for this connection
	write_handle: WriteHandle,
	// an attachment for this outbound connection that will be passed to the on_recv function
	// through a mutable reference.
	attach: T,
}

struct AcceptorData<T: Clone> {
	// socket
	socket: Socket,
	// closure the Evh will call when a connection that was accepted by this acceptor recvs
	// data.
	on_recv: fn(&mut T, Connection<T>, &[u8]) -> Result<()>,
	// closure the Evh will call when the acceptor accepts a new connection.
	on_accept: fn(&mut T, Connection<T>) -> Result<()>,
	// closure the Evh will call when the acceptor closes a connection that was associated with
	// this acceptor.
	on_close: fn(&mut T, Connection<T>) -> Result<()>,
	attach: T,
}

enum ConnectionData<T: Clone> {
	Inbound(InboundData<T>),
	Outbound(OutboundData<T>),
	Acceptor(AcceptorData<T>),
}

struct ConnectionInner<T: Clone>(ConnectionData<T>);

// Main connection type used to create outbound connections and servers.
#[derive(Clone)]
pub struct Connection<T: Clone> {
	inner: Rc<ConnectionInner<T>>,
}

impl<T: Clone> Connection<T> {
	// create an acceptor with the specified socket, callbacks, and attachment
	pub fn acceptor(
		socket: Socket,
		on_recv: fn(&mut T, Connection<T>, &[u8]) -> Result<()>,
		on_accept: fn(&mut T, Connection<T>) -> Result<()>,
		on_close: fn(&mut T, Connection<T>) -> Result<()>,
		attach: T,
	) -> Result<Self> {
		let inner = Rc::new(ConnectionInner(ConnectionData::Acceptor(AcceptorData {
			socket,
			on_recv,
			on_accept,
			on_close,
			attach,
		})))?;

		Ok(Self { inner })
	}

	// create an outbound connection with the specified socket, callback, and attachment
	pub fn outbound(
		socket: Socket,
		on_recv: fn(&mut T, Connection<T>, &[u8]) -> Result<()>,
		attach: T,
	) -> Result<Self> {
		let write_handle = WriteHandle::new(socket);
		let inner = Rc::new(ConnectionInner(ConnectionData::Outbound(OutboundData {
			socket,
			on_recv,
			attach,
			write_handle,
		})))?;
		Ok(Self { inner })
	}

	fn inbound(socket: Socket, acceptor: Connection<T>) -> Result<Self> {
		let write_handle = WriteHandle::new(socket);
		let inner = Rc::new(ConnectionInner(ConnectionData::Inbound(InboundData {
			socket,
			write_handle,
			acceptor,
		})))?;
		Ok(Self { inner })
	}

	// return the write handle associated with this connection.
	pub fn write_handle(&self) -> Result<&WriteHandle> {
		match &self.inner.0 {
			ConnectionData::Inbound(ib) => Ok(&ib.write_handle),
			ConnectionData::Outbound(ob) => Ok(&ob.write_handle),
			ConnectionData::Acceptor(_acc) => err!(IllegalState),
		}
	}

	// this function can be used to instruct the Evh to issue a callback to continue write
	// operations.
	pub fn wakeup(&self, _on_write_event: fn(Connection<T>) -> Result<()>) -> Result<()> {
		err!(Todo)
	}

	pub fn socket(&self) -> Socket {
		match &self.inner.0 {
			ConnectionData::Inbound(x) => x.socket,
			ConnectionData::Outbound(x) => x.socket,
			ConnectionData::Acceptor(x) => x.socket,
		}
	}

	fn write_handle_mut(&mut self) -> Result<&mut WriteHandle> {
		match &mut self.inner.0 {
			ConnectionData::Inbound(ib) => Ok(&mut ib.write_handle),
			ConnectionData::Outbound(ob) => Ok(&mut ob.write_handle),
			ConnectionData::Acceptor(_acc) => err!(IllegalState),
		}
	}
}

// The event handler struct
pub struct Evh<T: Clone> {
	multiplex: Multiplex,
	count: u64,
	_phantom_data: PhantomData<T>,
}

impl<T: Clone> Drop for Evh<T> {
	fn drop(&mut self) {
		let _ = self.stop();
	}
}

impl<T: Clone> Evh<T> {
	// create a new event handler instance
	pub fn new() -> Result<Self> {
		let multiplex = Multiplex::new()?;
		let count = 0;
		Ok(Self {
			multiplex,
			count,
			_phantom_data: PhantomData,
		})
	}

	// register a connection (acceptor or outbound) with this event handler.
	pub fn register(&mut self, connection: Connection<T>) -> Result<()> {
		let inner_clone = connection.inner.clone();
		match &connection.inner.0 {
			ConnectionData::Acceptor(s) => {
				let ptr = inner_clone.into_raw();
				self.multiplex
					.register(s.socket, RegisterType::Read, Some(ptr))?;
				Ok(())
			}
			_ => err!(Todo),
		}
	}

	// start the evh
	pub fn start(&mut self) -> Result<()> {
		spawn(|| {
			match Self::start_server(self.multiplex) {
				Ok(_) => {}
				Err(e) => {
					println!("FATAL: Server returned unexpected error: {}", e);
				}
			}
			println!("FATAL: Server returned unexpectedly without error!");
		})?;

		Ok(())
	}

	// stop the evh
	pub fn stop(&mut self) -> Result<()> {
		err!(Todo)
	}

	fn start_server(mplex: Multiplex) -> Result<()> {
		let mut events = [Event::new(); EVH_MAX_EVENTS];
		loop {
			let count = mplex.wait(&mut events, None)?;
			for i in 0..count {
				let evt = events[i];
				let attachment = evt.attachment();
				let mut rc: Rc<ConnectionInner<T>> = Rc::from_raw(attachment);
				let conn = Connection { inner: rc.clone() };
				let drop = match &mut rc.0 {
					ConnectionData::Acceptor(acc) => {
						Self::proc_accept(mplex, acc, conn)?;
						false
					}
					ConnectionData::Outbound(_ob) => false,
					ConnectionData::Inbound(ib) => Self::proc_event(ib, conn)?,
				};
				// we don't want to drop the rc now unless the connection closed
				if !drop {
					forget(rc);
				}
			}
		}
	}

	fn proc_accept(
		mut mplex: Multiplex,
		acc: &mut AcceptorData<T>,
		conn: Connection<T>,
	) -> Result<()> {
		let nsock = acc.socket.accept()?;
		let mut conn = Connection::inbound(nsock, conn)?;
		let wh = conn.write_handle_mut()?;
		wh.set_multiplex(mplex);
		let inner_clone = conn.inner.clone();
		let ptr = inner_clone.into_raw();
		mplex.register(nsock, RegisterType::Read, Some(ptr))?;
		(acc.on_accept)(&mut acc.attach, conn)?;
		Ok(())
	}

	// keep reading from connection until EAGAIN.
	// Return whether to drop the connection or not (close case)
	fn proc_event(ib: &mut InboundData<T>, conn: Connection<T>) -> Result<bool> {
		let mut bytes = [0u8; EVH_MAX_BYTES_PER_READ];
		loop {
			let len = match ib.socket.recv(&mut bytes) {
				Ok(len) => len,
				Err(e) => {
					match e == EAgain {
						true => return Ok(false),
						false => 0, // close on other errors
					}
				}
			};
			if len == 0 {
				match &mut ib.acceptor.inner.0 {
					ConnectionData::Acceptor(acc) => {
						let _ = (acc.on_close)(&mut acc.attach, conn.clone());
					}
					_ => {}
				}
				ib.socket.close()?;
				return Ok(true);
			} else {
				match &mut ib.acceptor.inner.0 {
					ConnectionData::Acceptor(acc) => {
						let _ =
							(acc.on_recv)(&mut acc.attach, conn.clone(), bytes.subslice(0, len)?);
					}
					_ => {}
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	#![allow(dead_code)]
	#![allow(unused_variables)]
	use super::*;
	#[test]
	fn test_evh1() -> Result<()> {
		/*
		use core::mem::size_of;
		let _x = size_of::<u8>();
		println!(
			"sz InboundData = {}, sz OutboundData = {}, sz AcceptorData = {}",
			size_of::<InboundData<u64>>(),
			size_of::<OutboundData<u64>>(),
			size_of::<AcceptorData<u64>>()
		);

		let mut evh = Evh::new()?;

		let mut s = Socket::new();
		let port = s.listen([127, 0, 0, 1], 9900, 10)?;
		let recv = |attach: &mut u64, conn: Connection<u64>, bytes: &[u8]| -> Result<()> {
			unsafe {
				use core::str::from_utf8_unchecked;
				let s = from_utf8_unchecked(bytes);
				println!("recv[{}]: '{}'", conn.socket(), s);
			}
			Ok(())
		};
		let accept = |attach: &mut u64, conn: Connection<u64>| -> Result<()> {
			println!("accept: {}", conn.socket());
			Ok(())
		};
		let close = |attach: &mut u64, conn: Connection<u64>| -> Result<()> {
			println!("close: {}", conn.socket());
			Ok(())
		};

		let server = Connection::acceptor(s, recv, accept, close, 0u64)?;
		evh.register(server)?;

		let mut s2 = Socket::new();
		s2.listen([127, 0, 0, 1], 9901, 10)?;
		let accept = |attach: &mut u64, conn: Connection<u64>| -> Result<()> {
			println!("accept2: {}", conn.socket());
			Ok(())
		};
		let server2 = Connection::acceptor(s2, recv, accept, close, 1u64)?;
		evh.register(server2)?;

		evh.start()?;

		park();
			*/

		Ok(())
	}
}
