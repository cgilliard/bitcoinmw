use core::fmt::Error as FormatError;
use core::fmt::Formatter as CoreFormatter;
use core::mem::size_of;
use core::ptr::null;
use net::constants::*;
use net::ffi::{
	event_handle, event_is_read, event_is_write, event_size, multiplex_close, multiplex_init,
	multiplex_register, multiplex_size, multiplex_unregister_write, multiplex_wait,
};
use net::Socket;
use prelude::*;

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct Multiplex(i32);

#[derive(Clone, Copy, PartialEq)]
pub enum RegisterType {
	Read,
	Write,
	RW,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Event([u8; EVENT_SIZE]);

impl Event {
	pub fn new() -> Self {
		unsafe {
			if event_size() != size_of::<Event>() {
				exit!(
					"event_size() ({}) != size_of::<Event>() ({}). Halting!",
					event_size(),
					size_of::<Event>()
				);
			}
		}

		Self([0u8; EVENT_SIZE])
	}

	pub fn is_read(&self) -> bool {
		unsafe { event_is_read(self as *const Event) }
	}

	pub fn is_write(&self) -> bool {
		unsafe { event_is_write(self as *const Event) }
	}

	pub fn socket(&self) -> Socket {
		let mut ret = Socket::new();
		unsafe { event_handle(&mut ret as *mut Socket, self as *const Event) }
		ret
	}
}

impl Debug for Multiplex {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> Result<(), FormatError> {
		#[cfg(test)]
		write!(_f, "{}", self.0)?;
		Ok(())
	}
}

impl Multiplex {
	pub fn new() -> Result<Multiplex, Error> {
		unsafe {
			if multiplex_size() != size_of::<i32>() {
				exit!(
					"multiplex_size() ({}) != size_of::<i32>() ({}). Halting!",
					multiplex_size(),
					size_of::<i32>()
				);
			}

			let ret = Self(-1);
			let res = multiplex_init(&ret as *const Multiplex);
			if res == 0 {
				Ok(ret)
			} else {
				Err(Error::new(SocketError))
			}
		}
	}

	pub fn register(
		&mut self,
		socket: Socket,
		rt: RegisterType,
		opt: Option<*const u8>,
	) -> Result<(), Error> {
		let flag = match rt {
			RegisterType::Read => MULTIPLEX_REGISTER_TYPE_FLAG_READ,
			RegisterType::Write => MULTIPLEX_REGISTER_TYPE_FLAG_WRITE,
			RegisterType::RW => {
				MULTIPLEX_REGISTER_TYPE_FLAG_READ | MULTIPLEX_REGISTER_TYPE_FLAG_WRITE
			}
		};
		let res = unsafe {
			multiplex_register(
				self as *const Multiplex,
				&socket as *const Socket,
				flag,
				match opt {
					Some(opt) => opt,
					None => null(),
				},
			)
		};
		if res == 0 {
			Ok(())
		} else {
			Err(Error::new(OperationFailed))
		}
	}

	pub fn unregister_write(
		&mut self,
		socket: Socket,
		opt: Option<*const u8>,
	) -> Result<(), Error> {
		let res = unsafe {
			multiplex_unregister_write(
				self as *const Multiplex,
				&socket as *const Socket,
				match opt {
					Some(opt) => opt,
					None => null(),
				},
			)
		};
		if res == 0 {
			Ok(())
		} else {
			Err(Error::new(OperationFailed))
		}
	}

	pub fn wait(&self, events: &mut [Event], timeout: Option<i64>) -> Result<u64, Error> {
		let timeout = match &timeout {
			Some(t) => *t,
			None => -1,
		};
		let len = events.len();
		if len == 0 || len > 0x7FFFFFFF {
			return Err(Error::new(IllegalArgument));
		}
		let res = unsafe {
			multiplex_wait(
				self as *const Multiplex,
				events.as_mut_ptr() as *mut *mut Event,
				len as i32,
				timeout,
			)
		};
		if res >= 0 {
			Ok(res as u64)
		} else {
			Err(Error::new(OperationFailed))
		}
	}

	pub fn close(&self) -> Result<(), Error> {
		let res = unsafe { multiplex_close(self as *const Multiplex) };

		if res == 0 {
			Ok(())
		} else {
			Err(Error::new(OperationFailed))
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use net::ffi::getfdcount;
	use std::ffi::sleep_millis;

	#[test]
	fn test_multiplex1() -> Result<(), Error> {
		// get initial file descriptor count
		let initial_fds = unsafe { getfdcount() };

		// create a multiplex
		let mut m1 = Multiplex::new()?;

		// create a socket
		let mut s1 = Socket::new();
		// start listening on the socket (allow system to choose unused port)
		let port = s1.listen([127, 0, 0, 1], 0, 1)?;

		// register for read events (accept = read), no timeout
		m1.register(s1, RegisterType::Read, None)?;

		// create a new socket
		let mut s2 = Socket::new();
		// connect on the port we've bound to
		s2.connect([127, 0, 0, 1], port)?;

		// create an event slice and wait for events
		let mut events = [Event::new(); 3];
		// assert that only 1 event is returned (our accept event)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);

		// confirm the event (at the first index in our slice) is the read event on our
		// listener)
		assert!(events[0].is_read());
		assert!(!events[0].is_write());
		assert_eq!(events[0].socket(), s1);

		// accept a new socket on our listener
		let mut s3 = events[0].socket().accept()?;

		// register s3 without our multiplex
		m1.register(s3, RegisterType::Read, None)?;

		// send a message back to the client
		loop {
			match s3.send(b"hi") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}

		// recieve the message on the client
		let mut buf = [0u8; 50];
		let len = loop {
			match s2.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		};

		// confirm message
		assert_eq!(len, 2);
		assert_eq!(buf[0], b'h');
		assert_eq!(buf[1], b'i');

		// confirm no messages waiting
		assert_eq!(m1.wait(&mut events, Some(10))?, 0);

		// write a message back
		// send a message back to the client
		loop {
			match s2.send(b"test") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}

		// assert that only 1 event is returned (our new message event)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);

		// confirm expected values
		assert!(events[0].is_read());
		assert!(!events[0].is_write());
		assert_eq!(events[0].socket(), s3);

		// recv 'test'
		assert_eq!(events[0].socket().recv(&mut buf)?, 4);
		assert_eq!(buf[0], b't');
		assert_eq!(buf[1], b'e');
		assert_eq!(buf[2], b's');
		assert_eq!(buf[3], b't');

		// confirm no messages waiting
		assert_eq!(m1.wait(&mut events, Some(10))?, 0);

		// close our connection
		s2.close()?;

		// assert that only 1 event is returned (our closed connection event)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);
		// confirm expected values

		assert!(events[0].is_read());
		assert!(!events[0].is_write());
		assert_eq!(events[0].socket(), s3);
		s3.close()?; // now we can call close on socket s3

		// confirm no messages waiting
		assert_eq!(m1.wait(&mut events, Some(10))?, 0);

		// close listener and multiplex
		s1.close()?;
		m1.close()?;

		// assert that no file descriptors are open
		assert_eq!(unsafe { getfdcount() }, initial_fds);

		Ok(())
	}

	#[test]
	fn test_multiple_events() -> Result<(), Error> {
		// get initial file descriptor count
		let initial_fds = unsafe { getfdcount() };

		// create a multiplex
		let mut m1 = Multiplex::new()?;

		// create a socket
		let mut listen = Socket::new();
		// start listening on the socket (allow system to choose unused port)
		let port = listen.listen([127, 0, 0, 1], 0, 3)?;

		// register for read events (accept = read), no timeout
		m1.register(listen, RegisterType::Read, None)?;

		// create a new socket
		let mut c1 = Socket::new();
		// connect on the port we've bound to
		c1.connect([127, 0, 0, 1], port)?;

		// create a new socket
		let mut c2 = Socket::new();
		// connect on the port we've bound to
		c2.connect([127, 0, 0, 1], port)?;

		// create a new socket
		let mut c3 = Socket::new();
		// connect on the port we've bound to
		c3.connect([127, 0, 0, 1], port)?;

		// create an event slice and wait for events
		let mut events = [Event::new(); 5];

		// we should have an event
		assert_eq!(m1.wait(&mut events, Some(10))?, 1);

		let mut r1 = events[0].socket().accept()?;
		let mut r2 = events[0].socket().accept()?;
		let mut r3 = events[0].socket().accept()?;

		// no more to accept
		assert_eq!(
			events[0].socket().accept().unwrap_err().kind(),
			ErrorKind::EAgain
		);

		m1.register(r1, RegisterType::Read, None)?;
		m1.register(r2, RegisterType::Read, None)?;
		m1.register(r3, RegisterType::Read, None)?;

		// send a message back to the server on all three clients
		loop {
			match c1.send(b"test1") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}

		loop {
			match c2.send(b"test2") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}

		loop {
			match c3.send(b"test3") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}

		// we should have an event on r1, r2, and r3
		// they might arrive at different times though
		let mut total_events = 0;
		let mut loop_count = 0;
		// ideally we get multiple events in the loop so sleep 10ms to
		// generally make that happen.
		unsafe {
			sleep_millis(10);
		}
		loop {
			unsafe {
				let evts = m1.wait(&mut events, Some(10))? as usize;
				for i in 0..evts {
					let socket = events[i].socket();
					let mut buf = [0u8; 10];
					assert_eq!(socket.recv(&mut buf)?, 5);
					assert_eq!(buf[0], b't');
					assert_eq!(buf[1], b'e');
					assert_eq!(buf[2], b's');
					assert_eq!(buf[3], b't');
					// test1 2 or 3
					assert!(buf[4] == b'1' || buf[4] == b'2' || buf[4] == b'3');
				}
				total_events += evts;
				if total_events == 3 {
					break;
				}
				sleep_millis(1);

				// ensure we don't infinite loop
				assert!(loop_count < 1_000);
				loop_count += 1;
			}
		}

		// sleep to allow errant events to propogate.
		unsafe {
			sleep_millis(10);
		}

		// make sure we have no more events
		assert_eq!(total_events, 3);
		assert_eq!(m1.wait(&mut events, Some(10))?, 0);

		// close resources
		c1.close()?;
		c2.close()?;
		c3.close()?;
		r1.close()?;
		r2.close()?;
		r3.close()?;
		listen.close()?;
		m1.close()?;

		// assert that no file descriptors are open
		assert_eq!(unsafe { getfdcount() }, initial_fds);

		Ok(())
	}

	#[test]
	fn test_multiplex_shutdown() -> Result<(), Error> {
		// get initial file descriptor count
		let initial_fds = unsafe { getfdcount() };

		// create a multiplex
		let mut m1 = Multiplex::new()?;

		// create a socket
		let mut s1 = Socket::new();
		// start listening on the socket (allow system to choose unused port)
		let port = s1.listen([127, 0, 0, 1], 0, 1)?;

		// register for read events (accept = read), no timeout
		m1.register(s1, RegisterType::Read, None)?;

		// create a new socket
		let mut s2 = Socket::new();
		// connect on the port we've bound to
		s2.connect([127, 0, 0, 1], port)?;

		// create an event slice and wait for events
		let mut events = [Event::new(); 3];
		// assert that only 1 event is returned (our accept event)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);

		// confirm the event (at the first index in our slice) is the read event on our
		// listener)
		assert!(events[0].is_read());
		assert!(!events[0].is_write());
		assert_eq!(events[0].socket(), s1);

		// accept a new socket on our listener
		let s3 = events[0].socket().accept()?;

		// register s3 without our multiplex
		m1.register(s3, RegisterType::Read, None)?;

		// send a message back to the client
		loop {
			match s3.send(b"hi") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}

		let mut buf = [0u8; 32];
		let len = loop {
			match s2.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		};

		assert_eq!(len, 2);
		assert_eq!(buf[0], b'h');
		assert_eq!(buf[1], b'i');

		// shutdown connection
		s3.shutdown()?;

		// assert that only 1 event is returned (our shutdown event)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);

		// we get a read event on our inbound socket 's3'.
		assert!(events[0].is_read());
		assert!(!events[0].is_write());
		assert_eq!(events[0].socket(), s3);

		// read 0 bytes here (closed connection)
		assert_eq!(events[0].socket().recv(&mut buf)?, 0);
		// now we can close it
		events[0].socket().close()?;

		// ensure client gets close
		let len = loop {
			match s2.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		};

		// read 0 bytes here (closed connection)
		assert_eq!(len, 0);

		// now we can close the fd
		s2.close()?;

		// close remaining resources
		s1.close()?; // listener
		m1.close()?; // multiplex

		// assert that no file descriptors are open
		assert_eq!(unsafe { getfdcount() }, initial_fds);

		Ok(())
	}

	#[test]
	fn test_multiplex_write() -> Result<(), Error> {
		// get initial file descriptor count
		let initial_fds = unsafe { getfdcount() };

		// create a multiplex
		let mut m1 = Multiplex::new()?;

		// create a socket
		let mut s1 = Socket::new();
		// start listening on the socket (allow system to choose unused port)
		let port = s1.listen([127, 0, 0, 1], 0, 1)?;

		// register for read events (accept = read), no timeout
		m1.register(s1, RegisterType::Read, None)?;

		// create a new socket
		let mut s2 = Socket::new();
		// connect on the port we've bound to
		s2.connect([127, 0, 0, 1], port)?;

		// create an event slice and wait for events
		let mut events = [Event::new(); 3];
		// assert that only 1 event is returned (our accept event)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);

		// confirm the event (at the first index in our slice) is the read event on our
		// listener)
		assert!(events[0].is_read());
		assert!(!events[0].is_write());
		assert_eq!(events[0].socket(), s1);

		// accept a new socket on our listener
		let mut s3 = events[0].socket().accept()?;

		// register s3 without our multiplex
		m1.register(s3, RegisterType::Read, None)?;

		// send a message back to the client
		loop {
			match s3.send(b"hi") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}

		// recieve the message on the client
		let mut buf = [0u8; 50];
		let len = loop {
			match s2.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		};

		// confirm message
		assert_eq!(len, 2);

		// we have established the connection, how let's write from the server. Generally,
		// you attempt a write first and only if you can't fully write do you schedule a
		// write, but to exercise the functionality we'll just register for write

		m1.register(s3, RegisterType::RW, None)?;

		// assert that only 1 event is returned (our write event)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);

		assert!(!events[0].is_read());
		assert!(events[0].is_write());
		assert_eq!(events[0].socket(), s3);

		events[0].socket().send(b"complete")?;

		// recieve the message on the client
		let mut buf = [0u8; 50];
		let len = loop {
			match s2.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		};

		// confirm message
		assert_eq!(len, 8);

		// assert that only 1 event is returned (since we're still registered for writing,
		// we get events until we unreg)
		assert_eq!(m1.wait(&mut events, Some(10_000))?, 1);

		assert!(!events[0].is_read());
		assert!(events[0].is_write());
		assert_eq!(events[0].socket(), s3);

		// ungregister
		m1.unregister_write(s3, None)?;

		// we should no longer have events waiting
		assert_eq!(m1.wait(&mut events, Some(10))?, 0);

		// cleanup handles
		m1.close()?;
		s1.close()?;
		s2.close()?;
		s3.close()?;

		assert!(s3.close().is_err());

		// assert that no file descriptors are open
		assert_eq!(unsafe { getfdcount() }, initial_fds);

		Ok(())
	}
}
