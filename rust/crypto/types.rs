use core::ops::Drop;
use ffi::*;

pub struct Secp {
	ctx: *mut Secp256k1Context,
	rand: *mut CsprngCtx,
}

impl Secp {
	pub fn new() -> Self {
		let ctx =
			unsafe { secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY) };
		let rand = unsafe { cpsrng_context_create() };
		Self { ctx, rand }
	}
}

impl Drop for Secp {
	fn drop(&mut self) {
		unsafe {
			secp256k1_context_destroy(self.ctx);
			cpsrng_context_destroy(self.rand);
		}
	}
}

pub struct PublicKey([u8; 33]);

pub struct Commitment([u8; 33]);

impl Commitment {
	pub fn new() {}
}
