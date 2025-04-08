use core::ptr::{copy_nonoverlapping, null};
use crypto::ffi::*;
use prelude::*;
use std::ffi::{alloc, release};

pub enum Sha3ByteSize {
	Sha3_256,
	Sha3_384,
	Sha3_512,
}
pub struct Sha3 {
	ctx: *const Sha3Context,
	byte_size: Sha3ByteSize,
}

impl Drop for Sha3 {
	fn drop(&mut self) {
		if !self.ctx.is_null() {
			unsafe {
				release(self.ctx as *const u8);
			}
			self.ctx = null();
		}
	}
}

impl Sha3 {
	pub fn new(byte_size: Sha3ByteSize) -> Result<Self, Error> {
		unsafe {
			// get size of context
			let size = sha3_context_size();
			// allocate memory
			let ctx = alloc(size) as *const Sha3Context;
			if ctx.is_null() {
				Err(Error::new(Alloc))
			} else {
				let res = match byte_size {
					Sha3ByteSize::Sha3_256 => sha3_Init(ctx, 256),
					Sha3ByteSize::Sha3_384 => sha3_Init(ctx, 384),
					Sha3ByteSize::Sha3_512 => sha3_Init(ctx, 512),
				};
				if res != 0 {
					release(ctx as *const u8);
					Err(Error::new(IllegalState))
				} else {
					// set flags to none (NIST sha3)
					sha3_SetFlags(ctx, 0);
					Ok(Self { ctx, byte_size })
				}
			}
		}
	}

	pub fn update(&mut self, b: &[u8]) {
		unsafe {
			sha3_Update(self.ctx, b.as_ptr(), b.len());
		}
	}

	pub fn finalize(&mut self, b: &mut [u8]) -> Result<(), Error> {
		match &self.byte_size {
			Sha3ByteSize::Sha3_256 => {
				if b.len() != 32 {
					Err(Error::new(IllegalArgument))
				} else {
					unsafe {
						let res = sha3_Finalize(self.ctx);
						copy_nonoverlapping(res, b.as_mut_ptr(), 32);
					}
					Ok(())
				}
			}
			Sha3ByteSize::Sha3_384 => {
				if b.len() != 48 {
					Err(Error::new(IllegalArgument))
				} else {
					unsafe {
						let res = sha3_Finalize(self.ctx);
						copy_nonoverlapping(res, b.as_mut_ptr(), 48);
					}
					Ok(())
				}
			}
			Sha3ByteSize::Sha3_512 => {
				if b.len() != 64 {
					Err(Error::new(IllegalArgument))
				} else {
					unsafe {
						let res = sha3_Finalize(self.ctx);
						copy_nonoverlapping(res, b.as_mut_ptr(), 64);
					}
					Ok(())
				}
			}
		}
	}
}
