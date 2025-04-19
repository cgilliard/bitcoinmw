#![allow(unused)]

extern "C" {
	// memory allocation
	pub fn alloc(bytes: usize) -> *const u8;
	pub fn release(ptr: *const u8);
	pub fn resize(ptr: *const u8, bytes: usize) -> *const u8;

	// formatting errors
	pub fn format_err(kind: *const u8, len: usize) -> *mut u8;

	// get number of allocations
	pub fn getalloccount() -> usize;

	// other functions
	pub fn write(fd: i32, buf: *const u8, len: usize) -> i32;
	pub fn ptr_add(p: *mut u8, v: i64);
	pub fn exit(code: i32);
	pub fn f64_to_str(d: f64, buf: *mut u8, capacity: u64) -> i32;
	pub fn getmicros() -> u64;
}
