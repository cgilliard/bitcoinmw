#![allow(dead_code)]
use core::ops::FnMut;
use core::ptr::copy;
use net::errors::EAgain;
use net::evh::*;
use net::socket::Socket;
use prelude::*;
use std::misc::from_utf8;

pub struct Handle {}

pub type WsOnRecv = Box<dyn FnMut(&Handle, &[u8]) -> Result<()>>;
pub type WsOnAccept = Box<dyn FnMut(&Handle) -> Result<()>>;
pub type WsOnClose = Box<dyn FnMut(&Handle) -> Result<()>>;

#[derive(PartialEq)]
enum WsState {
	Init,
	Started,
	Stopping,
	Stopped,
}

#[derive(Clone)]
pub struct WsContext {}

struct WsConnectionInner {
	rbuf: Vec<u8>,
	is_upgraded: bool,
}

#[derive(Clone)]
struct WsConnection {
	inner: Rc<WsConnectionInner>,
}

impl Drop for WsConnectionInner {
	fn drop(&mut self) {
		println!("wsconn inner drop");
	}
}

impl WsConnection {
	fn new() -> Result<Self> {
		let inner = Rc::new(WsConnectionInner {
			rbuf: Vec::new(),
			is_upgraded: false,
		})?;
		Ok(Self { inner })
	}
}

pub struct Ws {
	evh: Evh<WsContext, WsConnection>,
	state: WsState,
	handlers: RbTree<Handler>,
}

pub struct Listener {
	addr: [u8; 4],
	port: u16,
	backlog: i32,
}

struct HandlerProc {
	on_recv: Rc<WsOnRecv>,
	on_accept: Rc<WsOnAccept>,
	on_close: Rc<WsOnClose>,
}

pub struct Handler {
	handlers: Option<HandlerProc>,
	path: String,
}

impl PartialOrd for Handler {
	fn partial_cmp(&self, other: &Handler) -> Option<Ordering> {
		self.path.partial_cmp(&other.path)
	}
}

impl Ord for Handler {
	fn cmp(&self, other: &Self) -> Ordering {
		self.path.cmp(&other.path)
	}
}

impl PartialEq for Handler {
	fn eq(&self, other: &Handler) -> bool {
		self.path.eq(&other.path)
	}
}

impl Eq for Handler {}

impl Handler {
	fn with_path(path: String) -> Self {
		Self {
			path,
			handlers: None,
		}
	}
}

impl Ws {
	pub fn new() -> Result<Self> {
		let evh = Evh::new()?;
		let state = WsState::Init;
		let handlers = RbTree::new();
		Ok(Self {
			evh,
			state,
			handlers,
		})
	}

	pub fn add_listener(&mut self, listener: Listener) -> Result<()> {
		match &self.state {
			WsState::Init => {}
			_ => return err!(IllegalState),
		}

		let ctx = WsContext {};
		let socket = Socket::listen(listener.addr, listener.port, listener.backlog)?;

		let on_recv: OnRecv<WsContext, WsConnection> = Box::new(
			move |ctx: &mut WsContext,
			      conn: &mut Connection<WsContext, WsConnection>,
			      bytes: &[u8]|
			      -> Result<()> {
				match Self::proc_on_recv(ctx, conn, bytes) {
					Ok(_) => {}
					Err(e) => println!("WARN: proc_on_recv generated error: {}", e),
				};
				Ok(())
			},
		)?;

		let on_accept: OnAccept<WsContext, WsConnection> = Box::new(
			move |ctx: &mut WsContext,
			      conn: &mut Connection<WsContext, WsConnection>|
			      -> Result<()> { Self::proc_on_accept(ctx, conn) },
		)?;
		let on_close: OnClose<WsContext, WsConnection> = Box::new(
			move |ctx: &mut WsContext,
			      conn: &mut Connection<WsContext, WsConnection>|
			      -> Result<()> { Self::proc_on_close(ctx, conn) },
		)?;

		let on_recv = Rc::new(on_recv)?;
		let on_accept = Rc::new(on_accept)?;
		let on_close = Rc::new(on_close)?;

		let conn = Connection::acceptor(socket, on_recv, on_accept, on_close, ctx)?;
		self.evh.register(conn)?;

		Ok(())
	}

	pub fn start(&mut self) -> Result<()> {
		match &self.state {
			WsState::Init => {}
			_ => return err!(IllegalState),
		}
		self.state = WsState::Started;
		self.evh.start()
	}

	pub fn stop(&mut self) -> Result<()> {
		self.evh.stop()
	}

	pub fn add_handler(&mut self, handler: Handler) -> Result<()> {
		match &self.state {
			WsState::Init => {}
			_ => return err!(IllegalState),
		}

		let node = RbTreeNode::alloc(handler)?;
		self.handlers.try_insert(node)
	}

	fn proc_upgraded(
		_ctx: &mut WsContext,
		_conn: Connection<WsContext, WsConnection>,
		_bytes: &[u8],
	) -> Result<()> {
		Ok(())
	}

	fn proc_proto_negotiation(
		ctx: &mut WsContext,
		conn: Connection<WsContext, WsConnection>,
		bytes: &[u8],
		att: &mut WsConnection,
	) -> Result<()> {
		// if the read buffer is len == 0, there's no buffered data
		if att.inner.rbuf.len() == 0 {
			let mut offset = 0;
			loop {
				if offset < bytes.len() {
					let b = &bytes[offset..];
					// try to process bytes directly
					let clear_through = Self::proc_data(ctx, conn.clone(), b)?;
					// adjust offset as we go
					offset += clear_through;
					if clear_through == 0 {
						break;
					}
				}
			}
			// if we don't process to the end of bytes append
			if offset < bytes.len() {
				att.inner.rbuf.extend_from_slice(&bytes[offset..])?;
			}
		} else {
			att.inner.rbuf.extend_from_slice(bytes)?;
			loop {
				let clear_through = Self::proc_data(ctx, conn.clone(), &att.inner.rbuf[..])?;
				let rbuf_len = att.inner.rbuf.len();
				if clear_through >= rbuf_len {
					att.inner.rbuf.clear()?;
				} else if clear_through != 0 {
					let nlen = rbuf_len - clear_through;
					let ptr = att.inner.rbuf.as_mut_ptr();
					unsafe {
						copy(ptr.add(clear_through), ptr, nlen);
					}
					att.inner.rbuf.truncate(rbuf_len - clear_through)?;
				}
				if clear_through == 0 || att.inner.rbuf.len() == 0 {
					break;
				}
			}
		}

		Ok(())
	}

	fn proc_on_recv(
		ctx: &mut WsContext,
		conn: &mut Connection<WsContext, WsConnection>,
		bytes: &[u8],
	) -> Result<()> {
		let connection = conn.clone();
		// check for attachment
		match conn.attach()? {
			Some(att) => {
				if att.inner.is_upgraded {
					Self::proc_upgraded(ctx, connection, bytes)?;
				} else {
					Self::proc_proto_negotiation(ctx, connection, bytes, att)?;
				}
			}
			None => {
				println!("WARN: invalid state: connection with no attachment. Droping conn!");
				conn.close()?;
			}
		}
		Ok(())
	}

	fn proc_data(
		ctx: &mut WsContext,
		conn: Connection<WsContext, WsConnection>,
		bytes: &[u8],
	) -> Result<usize> {
		if let Some(pos) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
			if pos <= bytes.len() {
				let mut url = "";
				let mut key = "";
				if let Some(start) = bytes.windows(1).position(|window| window == b" ") {
					if start + 1 < bytes.len() {
						if let Some(end) = bytes[(start + 1)..]
							.windows(1)
							.position(|window| window == b" ")
						{
							if start + end + 1 < bytes.len() && 1 + start < start + end + 1 {
								url = from_utf8(&bytes[(1 + start)..(start + end + 1)])?;
							}
						}
					}
				}

				let target = b"\nSec-WebSocket-Key: ";
				if let Some(start) = bytes
					.windows(target.len())
					.position(|window| window == target)
				{
					if start + 1 < bytes.len() {
						if let Some(end) = bytes[(start + 1)..]
							.windows(1)
							.position(|window| window == b"\r")
						{
							let end = end + start + 1;
							let start = target.len() + start;
							if start < end && end <= bytes.len() {
								key = from_utf8(&bytes[start..end])?;
							}
						}
					}
				};

				Self::proc_handshake(ctx, conn, &bytes[0..pos], url, key)?;
			}
			Ok(pos + 4)
		} else {
			Ok(0)
		}
	}

	fn proc_handshake(
		_ctx: &mut WsContext,
		mut conn: Connection<WsContext, WsConnection>,
		bytes: &[u8],
		url: &str,
		key: &str,
	) -> Result<()> {
		println!("proc_handshake[{}][key='{}']: {}", url, key, bytes);

		let mykey = "somekeyhere";
		let msg = format!(
			"HTTP/1.1 101 Switching Protocols\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Accept: {}\r\n\
Sec-WebSocket-Protocol: chat\r\n\r\n",
			mykey
		)?;

		Self::write_fully(&mut conn, msg.as_bytes())?;

		Ok(())
	}

	fn proc_on_accept(
		_ctx: &mut WsContext,
		conn: &mut Connection<WsContext, WsConnection>,
	) -> Result<()> {
		println!("acc {}", conn.socket());

		let att = WsConnection::new()?;
		conn.set_attach(att.clone())?;

		Ok(())
	}

	fn proc_on_close(
		_ctx: &mut WsContext,
		conn: &mut Connection<WsContext, WsConnection>,
	) -> Result<()> {
		println!("close {}", conn.socket());
		Ok(())
	}

	fn write_fully(conn: &mut Connection<WsContext, WsConnection>, bytes: &[u8]) -> Result<()> {
		let mut offset = 0;
		let blen = bytes.len();
		while offset < blen {
			match conn.write(&bytes[offset..]) {
				Ok(wlen) => {
					offset += wlen;
				}
				Err(e) => {
					if e != EAgain {
						return Err(e);
					}
				}
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	// ws example
	/*
	let ws = ws!()?;

	// register a listener on port 9090 backlog 10, addr 0.0.0.0
	let l1 = listen!(Port(9090), Backlog(10), Addr([0,0,0,0]))?;
	ws.register(l1)?;

	// register a listener on port 8080 (default backlog = ?, Addr = 127.0.0.1)
	let l2 = listen!(Port(8080))?;
	ws.register(l2)?;

	// create a websocket responder at the uri /hello
	#[uri(/hello)]
	on!(ws, handle, {
		send!(handle, "hello world!)?;
	})?;

	// create a websocket responder at the uri /name
	#[uri(/name)]
	onmessage!(ws, handle, {
		send!(handle, "My name is sam!")?;
	})?;

	// start the websocket server
	ws.start()?;
	park();
	*/

	#[test]
	fn test_ws1() -> Result<()> {
		/*
		let mut ws = Ws::new()?;
		ws.add_listener(Listener {
			addr: [0, 0, 0, 0],
			port: 9090,
			backlog: 10,
		})?;

		ws.start()?;

		park();
			*/
		Ok(())
	}
}
