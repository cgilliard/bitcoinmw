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

mod builder;
mod cache;
mod client;
mod constants;
mod http;
mod macros;
mod test;
mod types;
mod ws;

pub use crate::types::{
	Builder, ConfigOption, ConnectionType, HttpClient, HttpClientConfig, HttpClientContainer,
	HttpConfig, HttpConnection, HttpConnectionConfig, HttpConnectionData, HttpContentReader,
	HttpHandler, HttpHeader, HttpHeaders, HttpInstance, HttpInstanceType, HttpMethod, HttpRequest,
	HttpRequestConfig, HttpResponse, HttpServer, HttpStats, HttpVersion, PlainConfig, TlsConfig,
	TlsConfigFiles, WebSocketClient, WebSocketClientConfig, WebSocketClientContainer,
	WebSocketConnection, WebSocketConnectionConfig, WebSocketData, WebSocketHandle,
	WebSocketHandler, WebSocketMessage, WebSocketMessageType,
};

pub use crate::client::{HTTP_CLIENT_CONTAINER, HTTP_CLIENT_CONTEXT};
pub use crate::ws::{WEBSOCKET_CLIENT_CONTAINER, WEBSOCKET_CLIENT_CONTEXT};

pub use crate::types::ConfigOption::*;
