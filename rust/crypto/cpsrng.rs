use core::ptr::null_mut;
use core::slice::from_raw_parts_mut;
use crypto::ffi::*;
use crypto::types::CpsrngContext;
use prelude::*;

#[derive(Clone)]
pub struct Cpsrng {
	ctx: *mut CpsrngContext,
}

impl Drop for Cpsrng {
	fn drop(&mut self) {
		if !self.ctx.is_null() {
			unsafe {
				cpsrng_context_destroy(self.ctx);
			}
			self.ctx = null_mut();
		}
	}
}

impl Cpsrng {
	pub fn new() -> Result<Self, Error> {
		let ctx = unsafe { cpsrng_context_create() };
		if ctx.is_null() {
			Err(Error::new(Alloc))
		} else {
			Ok(Self { ctx })
		}
	}

	pub fn gen(&self, v: &mut [u8]) {
		unsafe {
			cpsrng_rand_bytes(self.ctx, v.as_mut_ptr(), v.len());
		}
	}

	pub fn next_u64(&self) -> u64 {
		let mut v: u64 = 0;
		self.gen(unsafe { from_raw_parts_mut(&mut v as *mut u64 as *mut u8, 8) });
		v
	}
}

#[cfg(test)]
impl Cpsrng {
	pub fn new_s(seed: [u8; 32]) -> Result<Self, Error> {
		let ctx = unsafe { cpsrng_context_create() };
		if ctx.is_null() {
			Err(Error::new(Alloc))
		} else {
			let iv16 = [0u8; 16];
			unsafe {
				cpsrng_test_seed(ctx, iv16.as_ptr(), seed.as_ptr());
			}
			Ok(Self { ctx })
		}
	}

	pub fn reseed(&mut self, seed: [u8; 32]) {
		let iv16 = [0u8; 16];
		unsafe {
			cpsrng_test_seed(self.ctx, iv16.as_ptr(), seed.as_ptr());
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_seed() {
		let r = Cpsrng::new_s([8u8; 32]).unwrap();
		let v1 = r.next_u64();
		assert_eq!(v1, 3877200735786743394);
		let r2 = Cpsrng::new().unwrap();
		let r3 = Cpsrng::new().unwrap();
		assert!(r2.next_u64() != r3.next_u64());
	}
}
