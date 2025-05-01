#![allow(dead_code)]
use net::evh::Evh;
use prelude::*;

#[derive(PartialEq)]
enum WsState {
	Init,
	Started,
	Stopping,
	Stopped,
}

#[derive(Clone)]
pub struct WsContext {}

pub struct Ws {
	evh: Evh<WsContext>,
	state: WsState,
}

pub struct Listener {}

pub struct Handler {}

impl Ws {
	pub fn new() -> Result<Self> {
		let evh = Evh::new()?;
		let state = WsState::Init;
		Ok(Self { evh, state })
	}

	pub fn register(&mut self, _listener: Listener) -> Result<()> {
		nop!();
	}

	pub fn start(&mut self) -> Result<()> {
		nop!();
	}

	pub fn stop(&mut self) -> Result<()> {
		nop!();
	}

	pub fn add_handler(&mut self, _handler: Handler) -> Result<()> {
		nop!();
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
		onmessage!(ws, handle, {
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
		//let _ws = Ws::new()?;
		Ok(())
	}
}
