#![allow(unused)]

extern "C" {
	// memory allocation
	pub fn alloc(bytes: usize) -> *const u8;
	pub fn release(ptr: *const u8);
	pub fn resize(ptr: *const u8, bytes: usize) -> *const u8;

	// basic file ops for directory management
	pub fn mkdir(path: *const u8, mode: i32) -> i32;
	pub fn rmdir(path: *const u8) -> i32;
	pub fn unlink(path: *const u8) -> i32;

	// formatting errors
	pub fn format_err(kind: *const u8, len: usize) -> *mut u8;

	// other functions
	pub fn write(fd: i32, buf: *const u8, len: usize) -> i32;
	pub fn ptr_add(p: *mut u8, v: i64);
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

	// thread
	pub fn thread_create(start_routine: extern "C" fn(*mut u8), arg: *mut u8) -> i32;
	pub fn thread_create_joinable(
		handle: *const u8,
		start_routine: extern "C" fn(*mut u8),
		arg: *mut u8,
	) -> i32;
	pub fn thread_join(handle: *const u8) -> i32;
	pub fn thread_detach(handle: *const u8) -> i32;
	pub fn thread_handle_size() -> usize;

	// channels
	pub fn channel_init(channel: *const u8) -> i32;
	pub fn channel_send(channel: *const u8, ptr: *const u8) -> i32;
	pub fn channel_recv(channel: *const u8) -> *mut u8;
	pub fn channel_handle_size() -> usize;
	pub fn channel_destroy(channel: *const u8) -> i32;
	pub fn channel_pending(channel: *const u8) -> i32;

	// backtrace
	pub fn gen_backtrace() -> *const u8;
}

pub unsafe fn getalloccount() -> usize {
	0
}
