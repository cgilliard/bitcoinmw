use core::ptr::null_mut;
use crypto::constants::{SECP256K1_START_SIGN, SECP256K1_START_VERIFY};
use crypto::cpsrng::Cpsrng;
use crypto::ffi::secp256k1_context_create;
use crypto::types::Secp256k1Context;
use prelude::*;

pub struct Ctx {
	secp: *const Secp256k1Context,
	rng: Cpsrng,
}

impl Ctx {
	pub fn new() -> Result<Self, Error> {
		let rng = Cpsrng::new()?;
		let secp =
			unsafe { secp256k1_context_create(SECP256K1_START_SIGN | SECP256K1_START_VERIFY) };
		if secp == null_mut() {
			Err(Error::new(Alloc))
		} else {
			Ok(Self { secp, rng })
		}
	}

	pub fn gen(&self, b: &mut [u8]) {
		self.rng.gen(b);
	}

	pub fn as_ptr(&self) -> *const Secp256k1Context {
		self.secp
	}
}
