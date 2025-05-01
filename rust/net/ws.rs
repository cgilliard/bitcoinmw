#![allow(dead_code)]
use core::ops::FnMut;
use net::evh::*;
use net::socket::Socket;
use prelude::*;

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
pub struct WsContext {
	//connections: RbTree<WsConnection>,
}

#[derive(Clone)]
pub struct ConnectionState {}

struct WsConnection {
	conn: Connection<WsContext, ConnectionState>,
	handler: Option<Handler>,
}

impl PartialEq for WsConnection {
	fn eq(&self, other: &WsConnection) -> bool {
		self.conn.socket() == other.conn.socket()
	}
}

impl Eq for WsConnection {}

impl PartialOrd for WsConnection {
	fn partial_cmp(&self, other: &WsConnection) -> Option<Ordering> {
		self.conn.socket().partial_cmp(&other.conn.socket())
	}
}

impl Ord for WsConnection {
	fn cmp(&self, other: &WsConnection) -> Ordering {
		self.conn.socket().cmp(&other.conn.socket())
	}
}

pub struct Ws {
	evh: Evh<WsContext, ConnectionState>,
	state: WsState,
	handlers: RbTree<Handler>,
	connections: RbTree<WsConnection>,
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
		let connections = RbTree::new();
		Ok(Self {
			evh,
			state,
			handlers,
			connections,
		})
	}

	pub fn add_listener(&mut self, listener: Listener) -> Result<()> {
		match &self.state {
			WsState::Init => {}
			_ => return err!(IllegalState),
		}

		let ctx = WsContext {};
		let socket = Socket::listen(listener.addr, listener.port, listener.backlog)?;

		let on_recv: OnRecv<WsContext, ConnectionState> = Box::new(
			move |ctx: &mut WsContext,
			      conn: &mut Connection<WsContext, ConnectionState>,
			      bytes: &[u8]|
			      -> Result<()> { Self::proc_on_recv(ctx, conn, bytes) },
		)?;

		let on_accept: OnAccept<WsContext, ConnectionState> = Box::new(
			move |ctx: &mut WsContext,
			      conn: &mut Connection<WsContext, ConnectionState>|
			      -> Result<()> { Self::proc_on_accept(ctx, conn) },
		)?;
		let on_close: OnClose<WsContext, ConnectionState> = Box::new(
			move |ctx: &mut WsContext,
			      conn: &mut Connection<WsContext, ConnectionState>|
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
		_ctx: &mut WsContext,
		conn: &Connection<WsContext, ConnectionState>,
		bytes: &[u8],
	) -> Result<()> {
		println!("recv {}: {}", conn.socket(), bytes);
		Ok(())
	}

	fn proc_on_accept(
		_ctx: &mut WsContext,
		conn: &Connection<WsContext, ConnectionState>,
	) -> Result<()> {
		println!("acc {}", conn.socket());

		Ok(())
	}

	fn proc_on_close(
		_ctx: &mut WsContext,
		conn: &Connection<WsContext, ConnectionState>,
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
