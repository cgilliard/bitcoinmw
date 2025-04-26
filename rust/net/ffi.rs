use net::socket::Socket;

extern "C" {
	pub fn socket_size() -> usize;
	pub fn socket_connect(socc: *mut Socket, addr: *const u8, port: u16) -> i32;
	pub fn socket_listen(sock: *mut Socket, addr: *const u8, port: u16, backlog: i32) -> i32;
	pub fn socket_accept(sock: *mut Socket, accepted: *mut Socket) -> i32;
	pub fn socket_recv(sock: *const Socket, buf: *mut u8, capacity: usize) -> i32;
	pub fn socket_send(sock: *const Socket, buf: *const u8, len: usize) -> i32;
}
