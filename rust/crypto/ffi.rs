#![allow(dead_code)]
use crypto::types::{CpsrngContext, Sha3Context};

extern "C" {
	// sha3
	pub fn sha3_context_size() -> usize;
	pub fn sha3_Init(ctx: *const Sha3Context, bit_size: u32) -> i32;
	pub fn sha3_SetFlags(ctx: *const Sha3Context, flags: i32) -> i32;
	pub fn sha3_Update(ctx: *const Sha3Context, buf_in: *const u8, len: usize);
	pub fn sha3_Finalize(ctx: *const Sha3Context) -> *const u8;

	// cpsrng
	pub fn cpsrng_reseed();
	pub fn cpsrng_context_create() -> *mut CpsrngContext;
	pub fn cpsrng_context_destroy(ctx: *mut CpsrngContext);
	pub fn cpsrng_rand_bytes(ctx: *mut CpsrngContext, v: *mut u8, size: usize);

	// Only in tests
	pub fn cpsrng_test_seed(ctx: *mut CpsrngContext, iv16: *const u8, key32: *const u8);
}
