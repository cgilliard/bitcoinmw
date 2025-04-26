use core::mem::size_of;
use net::constants::*;
use net::ffi::{
	socket_accept, socket_connect, socket_listen, socket_recv, socket_send, socket_size,
};
use prelude::*;

#[repr(C)]
pub struct Socket(i32);

impl Socket {
	pub fn new() -> Self {
		unsafe {
			if socket_size() != size_of::<i32>() {
				exit!(
					"socket_size() ({}) != size_of::<i32() ({}). Halting!",
					socket_size(),
					size_of::<i32>()
				);
			}
		}
		Socket(-1)
	}

	pub fn connect(&mut self, addr: [u8; 4], port: u16) -> Result<(), Error> {
		let res = unsafe { socket_connect(self as *mut Socket, addr.as_ptr(), port) };
		if res == 0 {
			Ok(())
		} else if res == ERROR_SOCKET {
			Err(Error::new(SocketError))
		} else if res == ERROR_CONNECT {
			Err(Error::new(ConnectError))
		} else if res == ERROR_FCNTL {
			Err(Error::new(FcntlError))
		} else {
			Err(Error::new(Unknown))
		}
	}

	pub fn listen(&mut self, addr: [u8; 4], port: u16, backlog: i32) -> Result<u16, Error> {
		let res = unsafe { socket_listen(self as *mut Socket, addr.as_ptr(), port, backlog) };
		if res >= 0 && res <= 0xFFFF {
			Ok(res as u16)
		} else if res == ERROR_SOCKET {
			Err(Error::new(SocketError))
		} else if res == ERROR_SETSOCKOPT {
			Err(Error::new(SetSockOpt))
		} else if res == ERROR_FCNTL {
			Err(Error::new(FcntlError))
		} else if res == ERROR_BIND {
			Err(Error::new(BindError))
		} else if res == ERROR_LISTEN {
			Err(Error::new(ListenError))
		} else if res == ERROR_GETSOCKNAME {
			Err(Error::new(GetSockNameError))
		} else {
			Err(Error::new(Unknown))
		}
	}

	pub fn accept(&mut self) -> Result<Socket, Error> {
		let mut ret = Socket::new();
		let res = unsafe { socket_accept(self as *mut Socket, &mut ret as *mut Socket) };
		if res == 0 {
			Ok(ret)
		} else if res == ERROR_ACCEPT {
			Err(Error::new(AcceptError))
		} else if res == ERROR_EAGAIN {
			Err(Error::new(EAgain))
		} else if res == ERROR_FCNTL {
			Err(Error::new(FcntlError))
		} else {
			Err(Error::new(Unknown))
		}
	}

	pub fn recv(&self, buf: &mut [u8]) -> Result<usize, Error> {
		let res = unsafe { socket_recv(self as *const Socket, buf.as_mut_ptr(), buf.len()) };
		if res >= 0 {
			Ok(res as usize)
		} else {
			if res == ERROR_EAGAIN {
				Err(Error::new(EAgain))
			} else if res == ERROR_SOCKET {
				Err(Error::new(SocketError))
			} else {
				Err(Error::new(Unknown))
			}
		}
	}

	pub fn send(&self, buf: &[u8]) -> Result<usize, Error> {
		let res = unsafe { socket_send(self as *const Socket, buf.as_ptr(), buf.len()) };
		if res >= 0 {
			Ok(res as usize)
		} else {
			if res == ERROR_EAGAIN {
				Err(Error::new(EAgain))
			} else if res == ERROR_SOCKET {
				Err(Error::new(SocketError))
			} else {
				Err(Error::new(Unknown))
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_socket1() -> Result<(), Error> {
		let mut s1 = Socket::new();
		let port = s1.listen([127, 0, 0, 1], 9090, 10)?;
		let mut s2 = Socket::new();
		s2.connect([127, 0, 0, 1], port)?;
		let s3 = loop {
			match s1.accept() {
				Ok(s3) => break s3,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		};
		loop {
			match s2.send(b"hi") {
				Ok(_) => break,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		}
		let mut buf = [0u8; 5];
		let len = loop {
			match s3.recv(&mut buf) {
				Ok(len) => break len,
				Err(e) => assert_eq!(e.kind(), ErrorKind::EAgain),
			}
		};
		assert_eq!(len, 2);
		assert_eq!(buf[0], b'h');
		assert_eq!(buf[1], b'i');

		Ok(())
	}
}
