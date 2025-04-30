pub const MULTIPLEX_REGISTER_TYPE_FLAG_READ: i32 = 0x1;
pub const MULTIPLEX_REGISTER_TYPE_FLAG_WRITE: i32 = 0x1 << 1;

pub const ERROR_SOCKET: i32 = -1;
pub const ERROR_CONNECT: i32 = -2;
pub const ERROR_SETSOCKOPT: i32 = -3;
pub const ERROR_BIND: i32 = -4;
pub const ERROR_LISTEN: i32 = -5;
pub const ERROR_ACCEPT: i32 = -6;
pub const ERROR_FCNTL: i32 = -7;
pub const ERROR_GETSOCKNAME: i32 = -10;
pub const ERROR_EAGAIN: i32 = -11;

#[cfg(target_os = "linux")]
pub const EVENT_SIZE: usize = 12;
#[cfg(any(
	target_os = "macos",
	target_os = "freebsd",
	target_os = "openbsd",
	target_os = "netbsd"
))]
pub const EVENT_SIZE: usize = 32;

//pub const EVH_MAX_EVENTS: usize = 128;
//pub const EVH_MAX_BYTES_PER_READ: usize = 16 * 1024;
