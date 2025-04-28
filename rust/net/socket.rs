use core::mem::size_of;
use net::constants::*;
use net::errors::*;
use net::ffi::{
	socket_accept, socket_close, socket_connect, socket_listen, socket_recv, socket_send,
	socket_shutdown, socket_size,
};
use prelude::*;

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub struct Socket(i32);

impl Debug for Socket {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> FmtResult {
		#[cfg(test)]
		{
			if self.0 < 0 {
				write!(_f, "Socket[closed]")?;
			} else {
				write!(_f, "{}", self.0)?;
			}
		}
		Ok(())
	}
}

impl Display for Socket {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		writef!(f, "Socket[fd={}]", self.0)
	}
}

impl Socket {
	pub fn new() -> Self {
		unsafe {
			if socket_size() != size_of::<i32>() {
				exit!(
					"socket_size() ({}) != size_of::<i32>() ({}). Halting!",
					socket_size(),
					size_of::<i32>()
				);
			}
		}
		Socket(-1)
	}

	pub fn connect(addr: [u8; 4], port: u16) -> Result<Self> {
		let mut socket = Self::new();
		let res = unsafe { socket_connect(&mut socket as *mut Socket, addr.as_ptr(), port) };
		if res == 0 {
			Ok(socket)
		} else if res == ERROR_SOCKET {
			err!(SocketError)
		} else if res == ERROR_CONNECT {
			err!(ConnectError)
		} else if res == ERROR_FCNTL {
			err!(FcntlError)
		} else {
			err!(Unknown)
		}
	}

	pub fn listen(addr: [u8; 4], port: u16, backlog: i32) -> Result<Self> {
		if port == 0 || backlog <= 0 {
			return err!(IllegalArgument);
		}

		let mut socket = Self::new();
		let res =
			unsafe { socket_listen(&mut socket as *mut Socket, addr.as_ptr(), port, backlog) };
		if res >= 0 && res <= 0xFFFF {
			Ok(socket)
		} else if res == ERROR_SOCKET {
			err!(SocketError)
		} else if res == ERROR_SETSOCKOPT {
			err!(SetSockOpt)
		} else if res == ERROR_FCNTL {
			err!(FcntlError)
		} else if res == ERROR_BIND {
			err!(BindError)
		} else if res == ERROR_LISTEN {
			err!(ListenError)
		} else if res == ERROR_GETSOCKNAME {
			err!(GetSockNameError)
		} else {
			err!(Unknown)
		}
	}

	pub fn listen_rand(addr: [u8; 4], backlog: i32) -> Result<(u16, Self)> {
		if backlog <= 0 {
			return err!(IllegalArgument);
		}
		let mut socket = Self::new();
		let res = unsafe { socket_listen(&mut socket as *mut Socket, addr.as_ptr(), 0, backlog) };
		if res >= 0 && res <= 0xFFFF {
			Ok((res as u16, socket))
		} else if res == ERROR_SOCKET {
			err!(SocketError)
		} else if res == ERROR_SETSOCKOPT {
			err!(SetSockOpt)
		} else if res == ERROR_FCNTL {
			err!(FcntlError)
		} else if res == ERROR_BIND {
			err!(BindError)
		} else if res == ERROR_LISTEN {
			err!(ListenError)
		} else if res == ERROR_GETSOCKNAME {
			err!(GetSockNameError)
		} else {
			err!(Unknown)
		}
	}

	pub fn accept(&self) -> Result<Self> {
		if self.0 < 0 {
			return err!(IllegalState);
		}
		let mut ret = Socket::new();
		let res = unsafe { socket_accept(self as *const Socket, &mut ret as *mut Socket) };
		if res == 0 {
			Ok(ret)
		} else if res == ERROR_ACCEPT {
			err!(AcceptError)
		} else if res == ERROR_EAGAIN {
			err!(EAgain)
		} else if res == ERROR_FCNTL {
			err!(FcntlError)
		} else {
			err!(Unknown)
		}
	}

	pub fn recv(&self, buf: &mut [u8]) -> Result<usize> {
		if self.0 < 0 {
			return err!(IllegalState);
		}
		let res = unsafe { socket_recv(self as *const Socket, buf.as_mut_ptr(), buf.len()) };
		if res >= 0 {
			Ok(res as usize)
		} else {
			if res == ERROR_EAGAIN {
				err!(EAgain)
			} else if res == ERROR_SOCKET {
				err!(SocketError)
			} else {
				err!(Unknown)
			}
		}
	}

	pub fn send(&self, buf: &[u8]) -> Result<usize> {
		if self.0 < 0 {
			return err!(IllegalState);
		}
		let res = unsafe { socket_send(self as *const Socket, buf.as_ptr(), buf.len()) };
		if res >= 0 {
			Ok(res as usize)
		} else {
			if res == ERROR_EAGAIN {
				err!(EAgain)
			} else if res == ERROR_SOCKET {
				err!(SocketError)
			} else {
				err!(Unknown)
			}
		}
	}

	pub fn close(&mut self) -> Result<()> {
		if self.0 < 0 {
			return err!(IllegalState);
		}
		let res = unsafe { socket_close(self as *const Socket) };

		if res == 0 {
			self.0 = -1; // prevent double close
			Ok(())
		} else {
			err!(SocketError)
		}
	}

	pub fn shutdown(&self) -> Result<()> {
		if self.0 < 0 {
			return err!(IllegalState);
		}
		let res = unsafe { socket_shutdown(self as *const Socket) };
		if res == 0 {
			Ok(())
		} else {
			err!(SocketError)
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_socket1() -> Result<()> {
		let (port, mut s1) = Socket::listen_rand([127, 0, 0, 1], 1)?;
		let mut s2 = Socket::connect([127, 0, 0, 1], port)?;
		let mut s3 = loop {
			match s1.accept() {
				Ok(s3) => break s3,
				Err(e) => assert_eq!(e, EAgain),
			}
		};

		assert_ne!(s1, s2);
		assert_ne!(s1, s3);
		assert_ne!(s2, s3);

		loop {
			match s2.send(b"hi") {
				Ok(_) => break,
				Err(e) => assert_eq!(e, EAgain),
			}
		}
		let mut buf = [0u8; 5];
		let len = loop {
			match s3.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e, EAgain),
			}
		};
		assert_eq!(len, 2);
		assert_eq!(buf[0], b'h');
		assert_eq!(buf[1], b'i');

		s1.close()?;
		s2.close()?;
		s3.close()?;

		assert!(s1.close().is_err());
		assert!(s2.close().is_err());
		assert!(s3.close().is_err());

		let x = Socket::new();
		assert!(x.accept().is_err());
		assert!(x.recv(&mut []).is_err());
		assert!(x.send(&[]).is_err());
		assert!(x.shutdown().is_err());

		Ok(())
	}
}
