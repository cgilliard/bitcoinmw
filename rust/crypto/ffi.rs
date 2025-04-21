use crypto::types::{AESContext, Sha3Context};

extern "C" {
	// sha3
	pub fn sha3_context_size() -> usize;
	pub fn sha3_init256(ctx: *const Sha3Context) -> i32;
	pub fn sha3_update(ctx: *const Sha3Context, buf_in: *const u8, len: usize);
	pub fn sha3_finalize(ctx: *const Sha3Context) -> *const u8;

	// heavyhash
	pub fn heavyhash(matrix: *const u16, pdata: *const u8, len: usize, out: *mut u8);
	pub fn generate_matrix(matrix: *mut u16, aes: *const AESContext);

	// aes 256
	pub fn aes_context_size() -> usize;
	pub fn aes_init(ctx: *const AESContext, key: *const u8, iv: *const u8);
	pub fn aes_set_iv(ctx: *const AESContext, iv: *const u8);
	pub fn aes_ctr_xcrypt_buffer(ctx: *const AESContext, buf: *mut u8, len: usize);
}
