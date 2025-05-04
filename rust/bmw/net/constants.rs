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

// linux is 12, macos is 32. We use 32 to be platform independent in rust code.
// It's ok because only 128 events in the array (additional 2.5 kb for linux).
pub const EVENT_SIZE: usize = 32;
pub const EVH_MAX_EVENTS: usize = 128;
pub const EVH_MAX_BYTES_PER_READ: usize = 16 * 1024;

pub const WEBSOCKET_MAGIC_STRING: &[u8; 36] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
