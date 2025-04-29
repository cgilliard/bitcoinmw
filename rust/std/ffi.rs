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
	pub fn ptr_add(p: *mut u8, v: i64);

	// other functions
	pub fn write(fd: i32, buf: *const u8, len: usize) -> i32;
	pub fn exit(code: i32);
	pub fn f64_to_str(d: f64, buf: *mut u8, capacity: u64) -> i32;
	pub fn getmicros() -> u64;
	pub fn sleep_millis(ms: u64) -> i32;
	pub fn sched_yield() -> i32;
	pub fn rand_bytes(buf: *mut u8, len: usize) -> i32;

}
