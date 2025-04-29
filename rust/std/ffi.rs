#![allow(unused)]

extern "C" {
	// memory allocation
	pub fn alloc(bytes: usize) -> *const u8;
	pub fn release(ptr: *const u8);
	pub fn resize(ptr: *const u8, bytes: usize) -> *const u8;

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

	// atomic
	pub fn atomic_store_u64(ptr: *mut u64, value: u64);
	pub fn atomic_load_u64(ptr: *const u64) -> u64;
	pub fn atomic_fetch_add_u64(ptr: *mut u64, value: u64) -> u64;
	pub fn atomic_fetch_sub_u64(ptr: *mut u64, value: u64) -> u64;
	pub fn cas_release(ptr: *mut u64, expect: *const u64, desired: u64) -> bool;
}
