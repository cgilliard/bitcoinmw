#![allow(dead_code)]
use core::ops::FnMut;
use core::ptr::copy;
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
		let inner = Rc::new(WsConnectionInner { rbuf: Vec::new() })?;
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

	fn proc_on_recv(
		ctx: &mut WsContext,
		conn: &mut Connection<WsContext, WsConnection>,
		bytes: &[u8],
	) -> Result<()> {
		let connection = conn.clone();
		// check for attachment
		match conn.attach()? {
			Some(att) => {
				// if the read buffer is len == 0, there's no buffered data
				if att.inner.rbuf.len() == 0 {
					let mut offset = 0;
					loop {
						if offset < bytes.len() {
							let b = &bytes[offset..];
							// try to process bytes directly
							let clear_through = Self::proc_data(ctx, connection.clone(), b)?;
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
						let clear_through =
							Self::proc_data(ctx, connection.clone(), &att.inner.rbuf[..])?;
						println!("clear_through={}", clear_through);
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
						if clear_through == 0 {
							break;
						}
					}
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
		println!("proc[{}]: {}", conn.socket(), bytes);

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

				let target = b"\r\nSec-WebSocket-Key: ";
				if let Some(start) = bytes
					.windows(target.len())
					.position(|window| window == target)
				{
					if start + 1 < bytes.len() {
						if let Some(end) = bytes[(start + 1)..]
							.windows(2)
							.position(|window| window == b"\r\n")
						{
							let start = target.len() + start;
							let end = start + end + 1;
							if start < end && end < bytes.len() {
								key = from_utf8(&bytes[start..end])?;
							}
						}
					}
				};

				Self::proc_handshake(ctx, conn, &bytes[0..pos], url, key)?;
			}
			return Ok(pos + 4);
		}

		Ok(0)
	}

	fn proc_handshake(
		_ctx: &mut WsContext,
		_conn: Connection<WsContext, WsConnection>,
		bytes: &[u8],
		url: &str,
		key: &str,
	) -> Result<()> {
		println!("proc_handshake[{}][key='{}']: {}", url, key, bytes);
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
