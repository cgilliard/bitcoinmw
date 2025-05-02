#![allow(dead_code)]
use net::multiplex::{Event, Multiplex};
use net::socket::Socket;

extern "C" {
	// Sockets
	pub fn socket_size() -> usize;
	pub fn socket_connect(socc: *const Socket, addr: *const u8, port: u16) -> i32;
	pub fn socket_listen(sock: *const Socket, addr: *const u8, port: u16, backlog: i32) -> i32;
	pub fn socket_accept(sock: *const Socket, accepted: *mut Socket) -> i32;
	pub fn socket_recv(sock: *const Socket, buf: *mut u8, capacity: usize) -> i32;
	pub fn socket_send(sock: *const Socket, buf: *const u8, len: usize) -> i32;
	pub fn socket_close(sock: *const Socket) -> i32;
	pub fn socket_shutdown(sock: *const Socket) -> i32;

	// Multiplex
	pub fn multiplex_size() -> usize;
	pub fn event_size() -> usize;
	pub fn multiplex_init(multiplex: *const Multiplex) -> i32;
	pub fn multiplex_register(
		multiplex: *const Multiplex,
		socket: *const Socket,
		flags: i32,
		opt_data: *const u8,
	) -> i32;
	pub fn multiplex_unregister_write(
		multiplex: *const Multiplex,
		socket: *const Socket,
		opt_data: *const u8,
	) -> i32;
	pub fn multiplex_wait(
		multiplex: *const Multiplex,
		events: *mut *mut Event,
		max_events: i32,
		timeout_millis: i64,
	) -> i32;
	pub fn multiplex_close(multiplex: *const Multiplex) -> i32;

	// Events
	pub fn event_handle(socket: *mut Socket, event: *const Event);
	pub fn event_is_read(event: *const Event) -> bool;
	pub fn event_is_write(event: *const Event) -> bool;
	pub fn event_ptr(event: *const Event) -> *mut u8;

	// base64 and sha1 (for websockets)
	pub fn sha1(data: *const u8, size: usize, hash: *mut u8);
	pub fn Base64decode(plain: *mut u8, bufcoded: *const u8) -> i32;
	pub fn Base64encode(encoded: *mut u8, plain: *const u8, len: i32) -> i32;

}
