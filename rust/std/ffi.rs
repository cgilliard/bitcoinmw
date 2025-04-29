#![allow(unused)]

extern "C" {
	// memory allocation
	pub fn alloc(bytes: usize) -> *const ();
	pub fn release(ptr: *const ());
	pub fn resize(ptr: *const (), bytes: usize) -> *const ();

	// backtrace
	pub fn gen_backtrace() -> *const u8;

	// util
	pub fn cstring_len(ptr: *const u8) -> i32;
}
