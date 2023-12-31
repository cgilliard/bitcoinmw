// Copyright (c) 2023, The BitcoinMW Developers
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

use crate::constants::*;
use bmw_err::*;
use bmw_evh::{ConnectionData, EventHandlerConfig, WriteHandle, WriteState};
use bmw_util::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq)]
pub enum HttpRequestType {
	GET,
	POST,
	HEAD,
	PUT,
	DELETE,
	OPTIONS,
	CONNECT,
	TRACE,
	PATCH,
	UNKNOWN,
}

#[derive(Debug, PartialEq)]
pub enum HttpVersion {
	HTTP10,
	HTTP11,
	UNKNOWN,
	OTHER,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ConnectionType {
	KeepAlive,
	CLOSE,
}

#[derive(Clone, Copy)]
pub struct HttpHeader {
	pub start_header_name: usize,
	pub end_header_name: usize,
	pub start_header_value: usize,
	pub end_header_value: usize,
}

pub struct HttpHeaders<'a> {
	pub(crate) termination_point: usize,
	pub(crate) start: usize,
	pub(crate) req: &'a Vec<u8>,
	pub(crate) start_uri: usize,
	pub(crate) end_uri: usize,
	pub(crate) http_request_type: HttpRequestType,
	pub(crate) headers: [HttpHeader; 100],
	pub(crate) header_count: usize,
	pub(crate) version: HttpVersion,
	pub(crate) host: String,
	pub(crate) connection: ConnectionType,
	pub(crate) range_start: usize,
	pub(crate) range_end: usize,
	pub(crate) if_none_match: String,
	pub(crate) if_modified_since: String,
	pub(crate) is_websocket_upgrade: bool,
	pub(crate) sec_websocket_key: String,
	pub(crate) sec_websocket_protocol: String,
	pub(crate) accept_gzip: bool,
}

#[derive(Debug)]
pub struct HttpConnectionData {
	pub(crate) last_active: u128,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) tid: usize,
	pub(crate) websocket_data: Option<WebSocketData>,
}

#[derive(Debug, Clone)]
pub struct WebSocketData {
	pub uri: String,
	pub query: String,
	pub negotiated_protocol: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum WebSocketMessageType {
	Text,
	Binary,
	Close,
	Ping,
	Pong,
	Open,
	Accept,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WebSocketMessage {
	pub mtype: WebSocketMessageType,
	pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct WebSocketHandle {
	pub(crate) write_handle: WriteHandle,
}

pub trait HttpServer {
	fn start(&mut self) -> Result<(), Error>;
	fn stop(&mut self) -> Result<(), Error>;
	fn stats(&self) -> Result<HttpStats, Error>;
}

pub struct HttpStats {}
pub struct HttpRequest {}

#[derive(Clone, Debug)]
pub struct PlainConfig {
	pub http_dir_map: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct TlsConfig {
	pub cert_file: String,
	pub privkey_file: String,
	pub http_dir_map: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub enum HttpInstanceType {
	Plain(PlainConfig),
	Tls(TlsConfig),
}

#[derive(Clone, Debug)]
pub struct HttpInstance {
	pub port: u16,
	pub addr: String,
	pub listen_queue_size: usize,
	pub instance_type: HttpInstanceType,
	pub default_file: Vec<String>,
	pub error_400file: String,
	pub error_403file: String,
	pub error_404file: String,
	pub callback: Option<HttpCallback>,
	pub callback_mappings: HashSet<String>,
	pub callback_extensions: HashSet<String>,
	pub websocket_mappings: HashMap<String, HashSet<String>>,
	pub websocket_handler: Option<WebsocketHandler>,
}

#[derive(Clone)]
pub struct HttpConfig {
	pub evh_config: EventHandlerConfig,
	pub instances: Vec<HttpInstance>,
	pub debug: bool,
	pub cache_slab_count: usize,
	pub idle_timeout: u128,
	pub server_name: String,
	pub server_version: String,
	pub mime_map: Vec<(String, String)>,
	pub bring_to_front_weight: f64,
	pub restat_file_frequency_in_millis: u64,
	pub request_callback: Option<RequestCallback>,
}

pub struct Builder {}

type HttpCallback =
	fn(&HttpHeaders, &HttpConfig, &HttpInstance, &mut WriteHandle) -> Result<(), Error>;

pub(crate) type RequestCallback = fn(&HttpRequest) -> Result<(), Error>;

type WebsocketHandler = fn(
	&WebSocketMessage,
	&HttpConfig,
	&HttpInstance,
	&mut WebSocketHandle,
	&WebSocketData,
) -> Result<(), Error>;

// Crate local types
pub(crate) struct HttpServerImpl {
	pub(crate) config: HttpConfig,
	pub(crate) cache: Box<dyn LockBox<Box<dyn HttpCache + Send + Sync>>>,
}

pub(crate) struct HttpCacheImpl {
	pub(crate) hashtable: Box<dyn Hashtable<String, usize> + Send + Sync>,
}

pub(crate) struct HttpContext {
	pub(crate) suffix_tree: Box<dyn SuffixTree + Send + Sync>,
	pub(crate) matches: [Match; 1_000],
	pub(crate) offset: usize,
	pub(crate) connections: HashMap<u128, HttpConnectionData>,
	pub(crate) mime_map: HashMap<String, String>,
	pub(crate) mime_lookup: HashMap<u32, String>,
	pub(crate) mime_rev_lookup: HashMap<String, u32>,
	pub(crate) now: u128,
}

#[derive(PartialEq, Debug)]
pub(crate) enum CacheStreamResult {
	Hit,
	Miss,
	Modified,
	NotModified,
}

#[derive(Debug, PartialEq)]
pub(crate) enum FrameType {
	Continuation,
	Text,
	Binary,
	Close,
	Ping,
	Pong,
}

#[derive(Debug, PartialEq)]
pub(crate) struct FrameHeader {
	pub(crate) ftype: FrameType,     // which type of frame is this?
	pub(crate) mask: bool,           // is this frame masked?
	pub(crate) fin: bool,            // is this the last piece of data in the frame?
	pub(crate) payload_len: usize,   // size of the payload
	pub(crate) masking_key: u32,     // masking key
	pub(crate) start_content: usize, // start of the content of the message
}

pub(crate) trait HttpCache {
	fn stream_file(
		&self,
		path: &String,
		conn_data: &mut ConnectionData,
		code: u16,
		message: &str,
		ctx: &HttpContext,
		config: &HttpConfig,
		headers: &HttpHeaders,
		gzip: bool,
	) -> Result<CacheStreamResult, Error>;
	fn write_metadata(
		&mut self,
		path: &String,
		len: usize,
		last_modified: u64,
		mime_type: u32,
		now: u64,
		gzip: bool,
	) -> Result<bool, Error>;
	fn write_block(
		&mut self,
		path: &String,
		offset: usize,
		data: &[u8; CACHE_BUFFER_SIZE],
		gzip: bool,
	) -> Result<(), Error>;
	fn bring_to_front(&mut self, path: &String, gzip: bool) -> Result<(), Error>;
	fn remove(&mut self, path: &String, gzip: bool) -> Result<(), Error>;
	fn update_last_checked_if_needed(
		&mut self,
		fpath: &String,
		ctx: &HttpContext,
		config: &HttpConfig,
		gzip: bool,
	) -> Result<(), Error>;
}
