#![allow(dead_code)]
use core::ops::FnMut;
use core::ptr::copy;
use net::errors::EAgain;
use net::evh::*;
use net::socket::Socket;
use net::util::websocket_accept_key;
use prelude::*;
use std::misc::{from_utf8, to_be_bytes_u16, to_be_bytes_u64};

pub struct Handle {
	conn: Connection<WsContext, WsConnection>,
}

pub type WsOnRecv = Box<dyn FnMut(&mut Handle, &[u8], bool, u8) -> Result<()>>;
pub type WsOnAccept = Box<dyn FnMut(&mut Handle) -> Result<()>>;
pub type WsOnClose = Box<dyn FnMut(&mut Handle) -> Result<()>>;

#[derive(Clone, PartialEq)]
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
	handler: Option<Handler>,
}

#[derive(Clone)]
struct WsConnection {
	inner: Rc<WsConnectionInner>,
}

impl Handle {
	pub fn send(&mut self, bytes: &[u8]) -> Result<()> {
		let len = bytes.len();
		if len <= 125 {
			Ws::write_fully(&mut self.conn, &[0x82, bytes.len() as u8])?;
		} else if len <= 65535 {
			let mut buf = [0x82, 126, 0, 0];
			to_be_bytes_u16(len as u16, &mut buf[2..]);
			Ws::write_fully(&mut self.conn, &buf)?;
		} else {
			let mut buf = [0x82, 127, 0, 0, 0, 0, 0, 0, 0, 0];
			to_be_bytes_u64(len as u64, &mut buf[2..]);
			Ws::write_fully(&mut self.conn, &buf)?;
		}
		Ws::write_fully(&mut self.conn, bytes)?;

		Ok(())
	}
}

impl WsConnection {
	fn new() -> Result<Self> {
		let inner = Rc::new(WsConnectionInner {
			rbuf: Vec::new(),
			is_upgraded: false,
			handler: None,
		})?;
		Ok(Self { inner })
	}
}

pub struct Ws {
	evh: Evh<WsContext, WsConnection>,
	state: WsState,
	handlers: Rc<RbTree<Handler>>,
	sockets: Vec<Socket>,
	acceptors: Vec<Connection<WsContext, WsConnection>>,
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

struct HandlerInner {
	handlers: Option<HandlerProc>,
	path: String,
}

#[derive(Clone)]
pub struct Handler {
	inner: Rc<HandlerInner>,
}

impl PartialOrd for Handler {
	fn partial_cmp(&self, other: &Handler) -> Option<Ordering> {
		self.inner.path.partial_cmp(&other.inner.path)
	}
}

impl Ord for Handler {
	fn cmp(&self, other: &Self) -> Ordering {
		self.inner.path.cmp(&other.inner.path)
	}
}

impl PartialEq for Handler {
	fn eq(&self, other: &Handler) -> bool {
		self.inner.path.eq(&other.inner.path)
	}
}

impl Eq for Handler {}

impl Handler {
	fn new(
		path: &str,
		on_recv: Rc<WsOnRecv>,
		on_accept: Rc<WsOnAccept>,
		on_close: Rc<WsOnClose>,
	) -> Result<Self> {
		Ok(Self {
			inner: Rc::new(HandlerInner {
				path: String::new(path)?,
				handlers: Some(HandlerProc {
					on_recv,
					on_accept,
					on_close,
				}),
			})?,
		})
	}
	fn with_path(path: &str) -> Result<Self> {
		Ok(Self {
			inner: Rc::new(HandlerInner {
				path: String::new(path)?,
				handlers: None,
			})?,
		})
	}
}

impl Drop for Ws {
	fn drop(&mut self) {
		let binding = self.handlers.clone();
		let _ = self.acceptors.clear();
		let _ = self.sockets.clear();

		{
			let mut to_del = Vec::new();
			for handler in binding.iter() {
				let _ = to_del.push(handler);
			}

			let mut ptrs = Vec::new();
			for del in to_del {
				let v = self.handlers.remove(del.clone());
				match v {
					Some(v) => {
						let _ = ptrs.push(v);
					}
					None => {}
				}
			}

			for ptr in ptrs {
				ptr.release();
			}
		}
	}
}

impl Ws {
	pub fn new() -> Result<Self> {
		let evh = Evh::new()?;
		let state = WsState::Init;
		let handlers = Rc::new(RbTree::new())?;
		Ok(Self {
			evh,
			state,
			handlers,
			acceptors: Vec::new(),
			sockets: Vec::new(),
		})
	}

	pub fn add_listener(&mut self, listener: Listener) -> Result<()> {
		match &self.state {
			WsState::Init => {}
			_ => return err!(IllegalState),
		}

		let ctx = WsContext {};
		let socket = Socket::listen(listener.addr, listener.port, listener.backlog)?;
		let handlers = self.handlers.clone();

		let on_recv: OnRecv<WsContext, WsConnection> = Box::new(
			move |ctx: &mut WsContext,
			      conn: &mut Connection<WsContext, WsConnection>,
			      bytes: &[u8]|
			      -> Result<()> {
				match Self::proc_on_recv(ctx, conn, bytes, handlers.clone()) {
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
		self.evh.register(conn.clone())?;

		self.sockets.push(socket)?;
		self.acceptors.push(conn)?;

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
		match &self.state {
			WsState::Started => self.evh.stop(),
			_ => err!(IllegalState),
		}
	}

	pub fn add_handler(&mut self, handler: Handler) -> Result<()> {
		match &self.state {
			WsState::Init => {}
			_ => return err!(IllegalState),
		}

		//let node = Ptr::alloc(RbTreeNode::new(handler))?;
		let node = RbTreeNode::alloc(handler)?;
		self.handlers.try_insert(node)
	}

	fn proc_upgraded(
		ctx: &mut WsContext,
		conn: Connection<WsContext, WsConnection>,
		bytes: &[u8],
		handler: Option<Handler>,
	) -> Result<usize> {
		let len = bytes.len();
		let fin;
		let op;
		let mask;
		if len < 2 {
			return Ok(0);
		} else {
			fin = bytes[0] & 0x80 != 0;

			if bytes[0] & 0x70 != 0 {
				conn.close()?;
				return Ok(0);
			}

			op = bytes[0] & !0x80;
			mask = bytes[1] & 0x80 != 0;
		}

		// determine variable payload len
		let payload_len = bytes[1] & 0x7F;
		let (payload_len, mut offset) = if payload_len == 126 {
			if len < 4 {
				return Ok(0);
			}
			((bytes[2] as usize) << 8 | bytes[3] as usize, 4)
		} else if payload_len == 127 {
			if len < 10 {
				return Ok(0);
			}
			(
				(bytes[2] as usize) << 56
					| (bytes[3] as usize) << 48
					| (bytes[4] as usize) << 40
					| (bytes[5] as usize) << 32
					| (bytes[6] as usize) << 24
					| (bytes[7] as usize) << 16
					| (bytes[8] as usize) << 8
					| (bytes[9] as usize),
				10,
			)
		} else {
			(payload_len as usize, 2)
		};

		if offset + payload_len > len {
			return Ok(0);
		}

		let mut payload_vec: Vec<u8> = Vec::new();

		// if masking set we add 4 bytes for the masking key

		let payload_bytes = if mask {
			offset += 4;
			let masking_key = if offset - 1 < bytes.len() {
				[
					bytes[offset - 4],
					bytes[offset - 3],
					bytes[offset - 2],
					bytes[offset - 1],
				]
			} else {
				[0, 0, 0, 0]
			};

			payload_vec.resize(payload_len)?;
			for i in 0..payload_len {
				if i % 4 < masking_key.len() && offset + i < bytes.len() {
					payload_vec[i] = bytes[offset + i] ^ masking_key[i % 4];
				}
			}
			&payload_vec[..]
		} else if offset < offset + payload_len && offset + payload_len < bytes.len() {
			&bytes[offset..(offset + payload_len)]
		} else {
			&[]
		};

		Self::proc_payload(ctx, conn, payload_bytes, op, fin, handler)?;
		Ok(offset + payload_len)
	}

	fn proc_payload(
		_ctx: &mut WsContext,
		conn: Connection<WsContext, WsConnection>,
		bytes: &[u8],
		op: u8,
		fin: bool,
		handler: Option<Handler>,
	) -> Result<()> {
		let mut handle = Handle { conn };
		match handler {
			Some(mut handler) => match &mut handler.inner.handlers {
				Some(ref mut handlers) => (handlers.on_recv)(&mut handle, bytes, fin, op)?,
				None => println!("WARN: no handler found1!"),
			},
			None => println!("WARN: no handler found2!"),
		}
		Ok(())
	}

	fn proc_loop(
		ctx: &mut WsContext,
		conn: Connection<WsContext, WsConnection>,
		bytes: &[u8],
		att: &mut WsConnection,
		handlers: Rc<RbTree<Handler>>,
	) -> Result<()> {
		// if the read buffer is len == 0, there's no buffered data
		if att.inner.rbuf.len() == 0 {
			let mut offset = 0;
			while offset < bytes.len() {
				let b = &bytes[offset..];
				// try to process bytes directly
				let (clear_through, upgraded, handler) = Self::proc_data(
					ctx,
					conn.clone(),
					b,
					att.inner.is_upgraded,
					handlers.clone(),
					att.inner.handler.clone(),
				)?;
				// adjust offset as we go
				offset += clear_through;
				if upgraded {
					att.inner.is_upgraded = true;
					att.inner.handler = handler;
					let mut handle = Handle { conn: conn.clone() };
					match att.inner.handler {
						Some(ref mut handler) => match handler.inner.handlers {
							Some(ref mut handlers) => (handlers.on_accept)(&mut handle)?,
							None => println!("WARN: no handler1"),
						},
						None => println!("WARN: no handler2"),
					}
					break;
				} else if clear_through == 0 {
					break;
				}
			}
			// if we don't process to the end of bytes append
			if offset < bytes.len() {
				att.inner.rbuf.extend_from_slice(&bytes[offset..])?;
			}
		} else {
			att.inner.rbuf.extend_from_slice(bytes)?;
			loop {
				let (clear_through, upgraded, handler) = Self::proc_data(
					ctx,
					conn.clone(),
					&att.inner.rbuf[..],
					att.inner.is_upgraded,
					handlers.clone(),
					att.inner.handler.clone(),
				)?;
				if upgraded {
					att.inner.is_upgraded = true;
					att.inner.handler = handler;
					// clear the buffer here. should not be anything, but we
					// explicitly start from scratch
					att.inner.rbuf.clear();
					break;
				}
				let rbuf_len = att.inner.rbuf.len();
				if clear_through >= rbuf_len {
					att.inner.rbuf.clear();
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
		handlers: Rc<RbTree<Handler>>,
	) -> Result<()> {
		let connection = conn.clone();
		// check for attachment
		match conn.attach()? {
			Some(att) => {
				Self::proc_loop(ctx, connection, bytes, att, handlers)?;
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
		upgraded: bool,
		handlers: Rc<RbTree<Handler>>,
		handler: Option<Handler>,
	) -> Result<(usize, bool, Option<Handler>)> {
		if upgraded {
			return Ok((Self::proc_upgraded(ctx, conn, bytes, handler)?, false, None));
		}
		if let Some(pos) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
			let mut upgraded = false;
			let mut handler: Option<Handler> = None;
			if pos <= bytes.len() {
				let mut key = "";
				let mut url = "";
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

				if url == "" {
					// invalid url specified TODO: return bad request
					conn.close()?;
					return Ok((0, false, None));
				} else {
					//let node = Ptr::alloc(RbTreeNode::new(Handler::with_path(url)?))?;
					let node = RbTreeNode::alloc(Handler::with_path(url)?)?;
					let pair = handlers.search(node);
					node.release();
					if pair.cur.is_null() {
						// path not found TODO: return 404
						conn.close()?;
						return Ok((0, false, None));
					} else {
						handler = Some((&*(pair.cur.value)).clone());
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

				if Self::proc_handshake(ctx, conn, key)? {
					upgraded = true;
				}
			}
			Ok((pos + 4, upgraded, handler))
		} else {
			Ok((0, false, None))
		}
	}

	fn proc_handshake(
		_ctx: &mut WsContext,
		mut conn: Connection<WsContext, WsConnection>,
		key: &str,
	) -> Result<bool> {
		let msg = if key.len() == 0 {
			format!("HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n")?
		} else {
			let accept = websocket_accept_key(key)?;
			format!(
				"HTTP/1.1 101 Switching Protocols\r\n\
Upgrade: websocket\r\n\
Connection: Upgrade\r\n\
Sec-WebSocket-Accept: {}\r\n\r\n",
				accept
			)?
		};

		Self::write_fully(&mut conn, msg.as_bytes())?;
		if key.len() == 0 {
			conn.close()?;
			Ok(false)
		} else {
			Ok(true)
		}
	}

	fn proc_on_accept(
		_ctx: &mut WsContext,
		conn: &mut Connection<WsContext, WsConnection>,
	) -> Result<()> {
		let att = WsConnection::new()?;
		conn.set_attach(att.clone())?;

		Ok(())
	}

	fn proc_on_close(
		_ctx: &mut WsContext,
		conn: &mut Connection<WsContext, WsConnection>,
	) -> Result<()> {
		let mut handle = Handle { conn: conn.clone() };
		// check for attachment
		match conn.attach()? {
			Some(att) => match att.inner.handler {
				Some(ref mut handler) => match handler.inner.handlers {
					Some(ref mut handlers) => (handlers.on_close)(&mut handle)?,
					None => println!("WARN: no handler1"),
				},
				None => println!("WARN: no handler2"),
			},
			None => {
				println!("WARN: invalid state: connection with no attachment. Droping conn!");
			}
		}
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

	#[cfg(test)]
	fn get_sockets(&self) -> &Vec<Socket> {
		&self.sockets
	}

	#[cfg(test)]
	fn get_acceptors(&mut self) -> &mut Vec<Connection<WsContext, WsConnection>> {
		&mut self.acceptors
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

		let on_recv: WsOnRecv = Box::new(
			|handle: &mut Handle, bytes: &[u8], fin: bool, op: u8| -> Result<()> {
				println!("test_ws1 recv: {}, op = {}, fin = {}", bytes, op, fin);
				if op == 2 && fin {
					handle.send(b"okokok")?;
				}
				Ok(())
			},
		)?;
		let on_accept: WsOnAccept = Box::new(|handle: &mut Handle| -> Result<()> {
			println!("accepted {}", handle.conn.socket());
			handle.send(b"hi1234")?;
			Ok(())
		})?;
		let on_close: WsOnClose = Box::new(|handle: &mut Handle| -> Result<()> {
			println!("closed {}", handle.conn.socket());
			Ok(())
		})?;
		let on_recv = Rc::new(on_recv)?;
		let on_accept = Rc::new(on_accept)?;
		let on_close = Rc::new(on_close)?;

		let handler = Handler::new("/abc", on_recv, on_accept, on_close)?;
		ws.add_handler(handler)?;

		ws.start()?;

		sleep(100);
		ws.stop()?;
		sleep(100);

		for s in ws.get_sockets() {
			s.clone().close()?;
		}

		for a in ws.get_acceptors().iter_mut() {
			unsafe {
				a.drop_rc();
			}
		}

		park();
				*/
		Ok(())
	}
}
