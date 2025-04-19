extern "C" {
	// memory allocation
	pub fn alloc(bytes: usize) -> *const u8;
	pub fn release(ptr: *const u8);
	pub fn resize(ptr: *const u8, bytes: usize) -> *const u8;

	pub fn write(fd: i32, buf: *const u8, len: usize) -> i32;
	pub fn format_err(kind: *const u8, len: usize) -> *mut u8;
}
