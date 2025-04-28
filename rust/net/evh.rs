#![allow(dead_code)]
use core::marker::PhantomData;
use core::mem::forget;
use net::constants::*;
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
}

// internal representation for Inbound connection (one that is accepted by another connection (i.e.
// a listener)).
struct InboundData<T> {
	socket: Socket,            // socket
	write_handle: WriteHandle, // write handle associated with this connection
	acceptor: Connection<T>,   // the connection that accepted this inbound connection
}

struct OutboundData<T> {
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

struct AcceptorData<T> {
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

enum ConnectionData<T> {
	Inbound(InboundData<T>),
	Outbound(OutboundData<T>),
	Acceptor(AcceptorData<T>),
}

struct ConnectionInner<T>(ConnectionData<T>);

// Main connection type used to create outbound connections and servers.
#[derive(Clone)]
pub struct Connection<T> {
	inner: Rc<ConnectionInner<T>>,
}

impl<T> Connection<T> {
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
}

// The event handler struct
pub struct Evh<T> {
	multiplex: Multiplex,
	count: u64,
	_phantom_data: PhantomData<T>,
}

impl<T> Drop for Evh<T> {
	fn drop(&mut self) {
		let _ = self.stop();
	}
}

impl<T> Evh<T> {
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
			//println!("in start");
			let _ = Self::start_server(self.multiplex);
		})?;

		Ok(())
	}

	// stop the evh
	pub fn stop(&mut self) -> Result<()> {
		err!(Todo)
	}

	fn start_server(mplex: Multiplex) -> Result<()> {
		//println!("in start_server");
		let mut events = [Event::new(); EVH_MAX_EVENTS];
		loop {
			let count = mplex.wait(&mut events, None)?;
			//println!("got {} events", count);
			for i in 0..count {
				let evt = events[i];
				let attachment = evt.attachment();
				println!("att={}", Ptr::new(attachment));
				let rc: Rc<ConnectionInner<T>> = Rc::from_raw(attachment);
				match &rc.0 {
					ConnectionData::Acceptor(_acc) => {
						//println!("acc {}", acc.socket);
					}
					ConnectionData::Outbound(_ob) => {
						//println!("ob {}", ob.socket);
					}
					ConnectionData::Inbound(_ib) => {
						//println!("inb {}", ib.socket);
						return Ok(());
					}
				}
				//println!("is_read={},is_write={}", evt.is_read(), evt.is_write());
				// we don't want to drop the rc now.
				forget(rc);
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
		use core::mem::size_of;
		let _x = size_of::<u8>();
		/*
		println!(
			"sz InboundData = {}, sz OutboundData = {}, sz AcceptorData = {}",
			size_of::<InboundData<u64>>(),
			size_of::<OutboundData<u64>>(),
			size_of::<AcceptorData<u64>>()
		);
				*/

		/*
		let mut evh = Evh::new()?;

		let mut s = Socket::new();
		//println!("s.socket={}", s);
		let port = s.listen([127, 0, 0, 1], 9900, 10)?;
		//println!("s.socket={}", s);
		//println!("listen on port {}", port);
		let recv = |attach: &mut u64, conn: Connection<u64>, bytes: &[u8]| -> Result<()> { Ok(()) };
		let accept = |attach: &mut u64, conn: Connection<u64>| -> Result<()> { Ok(()) };
		let close = |attach: &mut u64, conn: Connection<u64>| -> Result<()> { Ok(()) };

		let server = Connection::acceptor(s, recv, accept, close, 0u64)?;
		evh.register(server)?;

		let mut s2 = Socket::new();
		s2.listen([127, 0, 0, 1], 9901, 10)?;
		let server2 = Connection::acceptor(s2, recv, accept, close, 1u64)?;
		evh.register(server2)?;

		evh.start()?;
				*/

		//park();

		Ok(())
	}
}
