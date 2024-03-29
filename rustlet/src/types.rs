// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bmw_deps::dyn_clone::{clone_trait_object, DynClone};
use bmw_err::*;
use bmw_evh::{WriteHandle, WriteState};
use bmw_http::{
	HttpConfig, HttpContentReader, HttpMethod, HttpServer, HttpVersion, WebSocketData,
	WebSocketHandle, WebSocketMessage,
};
use bmw_util::*;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;

pub type Rustlet = Pin<
	Box<
		dyn Fn(&mut Box<dyn RustletRequest>, &mut Box<dyn RustletResponse>) -> Result<(), Error>
			+ Send
			+ Sync,
	>,
>;

pub type Websocket =
	Pin<Box<dyn Fn(&mut Box<dyn WebSocketRequest>) -> Result<(), Error> + Send + Sync>>;

/// The main trait used for processing requests in a rustlet. It has all the information needed in
/// it. It can be accessed by the [`crate::request`] macro.
pub trait RustletRequest: DynClone + Send + Sync {
	fn header_host(&self) -> &String;
	fn method(&self) -> &HttpMethod;
	fn version(&self) -> &HttpVersion;
	fn path(&self) -> &String;
	fn query(&self) -> &String;
	fn headers(&self) -> &Vec<(String, String)>;
	fn content_reader(&self) -> HttpContentReader;
}

clone_trait_object!(RustletRequest);

pub trait RustletResponse: DynClone + Send + Sync {
	fn write(&mut self, bytes: &[u8]) -> Result<(), Error>;
	fn flush(&mut self) -> Result<(), Error>;
	fn set_async(&mut self) -> Result<(), Error>;
	fn add_header(&mut self, name: &str, value: &str) -> Result<(), Error>;
	fn set_content_type(&mut self, value: &str) -> Result<(), Error>;
	fn redirect(&mut self, url: &str) -> Result<(), Error>;
	fn set_connection_close(&mut self) -> Result<(), Error>;
	fn async_complete(&mut self) -> Result<(), Error>;
}

clone_trait_object!(RustletResponse);

pub trait WebSocketRequest: DynClone {
	fn handle(&mut self) -> &mut WebSocketHandle;
	fn message(&self) -> &WebSocketMessage;
	fn data(&self) -> &WebSocketData;
	fn path(&self) -> &String;
	fn query(&self) -> &String;
	fn protocol(&self) -> Option<&String>;
}

clone_trait_object!(WebSocketRequest);

pub struct RustletConfig {
	pub http_config: HttpConfig,
	pub rustlet_config: RustletContainerConfig,
}

#[derive(Clone, Debug)]
pub struct RustletContainerConfig {
	pub main_log_file: Option<String>,
}

pub struct RustletContainer {
	pub(crate) rustlets: HashMap<String, Rustlet>,
	pub(crate) websockets: HashMap<String, Websocket>,
	pub(crate) rustlet_mappings: HashMap<String, String>,
	pub(crate) websocket_mappings: HashMap<String, (String, HashSet<String>)>,
	pub(crate) config: RustletConfig,
	pub(crate) http_server: Option<Box<dyn HttpServer + Send + Sync>>,
}

// Crate local structs

#[derive(Clone)]
pub(crate) struct RustletRequestImpl {
	pub(crate) path: String,
	pub(crate) header_host: String,
	pub(crate) query: String,
	pub(crate) method: HttpMethod,
	pub(crate) version: HttpVersion,
	pub(crate) headers: Vec<(String, String)>,
	pub(crate) reader: HttpContentReader,
}

#[derive(Clone)]
pub(crate) struct RustletResponseImpl {
	pub(crate) wh: WriteHandle,
	pub(crate) state: Box<dyn LockBox<RustletResponseState>>,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) depth: usize,
}

pub(crate) struct RustletResponseState {
	pub(crate) sent_headers: bool,
	pub(crate) completed: bool,
	pub(crate) close: bool,
	pub(crate) content_type: String,
	pub(crate) buffer: Vec<u8>,
	pub(crate) redirect: Option<String>,
	pub(crate) additional_headers: Vec<(String, String)>,
	pub(crate) is_async: bool,
}

#[derive(Clone)]
pub(crate) struct WebSocketRequestImpl {
	pub(crate) handle: WebSocketHandle,
	pub(crate) message: WebSocketMessage,
	pub(crate) data: WebSocketData,
}

pub(crate) struct RustletContext {
	pub(crate) search_trie: Box<dyn SearchTrie + Sync + Send>,
	pub(crate) matches: [Match; 1_000],
}
