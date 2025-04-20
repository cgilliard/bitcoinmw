use crypto::types::Sha3Context;

extern "C" {
	// sha3
	pub fn sha3_context_size() -> usize;
	pub fn sha3_init256(ctx: *const Sha3Context) -> i32;
	pub fn sha3_setflags(ctx: *const Sha3Context, flags: i32) -> i32;
	pub fn sha3_update(ctx: *const Sha3Context, buf_in: *const u8, len: usize);
	pub fn sha3_finalize(ctx: *const Sha3Context) -> *const u8;
}
