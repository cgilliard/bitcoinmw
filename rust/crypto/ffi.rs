#![allow(dead_code)]
use crypto::types::Sha3Context;

extern "C" {
	// sha3
	pub fn sha3_context_size() -> usize;
	pub fn sha3_Init(ctx: *const Sha3Context, bit_size: u32) -> i32;
	pub fn sha3_SetFlags(ctx: *const Sha3Context, flags: i32) -> i32;
	pub fn sha3_Update(ctx: *const Sha3Context, buf_in: *const u8, len: usize);
	pub fn sha3_Finalize(ctx: *const Sha3Context) -> *const u8;
}
