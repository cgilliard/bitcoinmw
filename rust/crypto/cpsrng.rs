use core::ptr::write_volatile;
use crypto::aes::Aes256;
use prelude::*;
use std::ffi::rand_bytes;

pub struct Cpsrng(Aes256);

impl Cpsrng {
	pub fn new() -> Result<Self, Error> {
		Ok(Self(Self::reseed_impl()?))
	}

	pub fn reseed(&mut self) -> Result<(), Error> {
		self.0 = Self::reseed_impl()?;
		Ok(())
	}

	fn reseed_impl() -> Result<Aes256, Error> {
		let mut key = [0u8; 32];
		let mut iv = [0u8; 16];
		unsafe {
			if rand_bytes(key.as_mut_ptr(), 32) != 0 {
				return Err(Error::new(InsufficientEntropy));
			}
			if rand_bytes(iv.as_mut_ptr(), 16) != 0 {
				return Err(Error::new(InsufficientEntropy));
			}
		}
		let res = Aes256::new(key, iv)?;

		// zeroize key/iv
		for b in key.iter_mut() {
			unsafe {
				write_volatile(b, 0);
			}
		}
		for b in iv.iter_mut() {
			unsafe {
				write_volatile(b, 0);
			}
		}
		Ok(res)
	}

	pub fn gen(&self, bytes: &mut [u8]) {
		self.0.crypt(bytes);
	}

	#[cfg(test)]
	pub fn test_seed(key: [u8; 32], iv: [u8; 16]) -> Result<Self, Error> {
		Ok(Self(Aes256::new(key, iv)?))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_cpsrng_counter_advancement() -> Result<(), Error> {
		let rng = Cpsrng::new()?;

		let mut b1 = [0u8; 32];
		rng.gen(&mut b1);

		let mut b1a = [0u8; 32];
		rng.gen(&mut b1a);

		let mut b1b = [0u8; 32];
		rng.gen(&mut b1b);

		let mut b1c = [0u8; 32];
		rng.gen(&mut b1c);

		assert_ne!(b1, [0u8; 32]); // Non-zero output
		assert_ne!(b1, b1a);
		assert_ne!(b1a, b1b);
		assert_ne!(b1b, b1c);
		Ok(())
	}

	#[test]
	fn test_cpsrng_reseed() -> Result<(), Error> {
		let mut rng = Cpsrng::new()?;

		let mut b1 = [0u8; 32];
		rng.gen(&mut b1);

		rng.reseed()?;

		let mut b2 = [0u8; 32];
		rng.gen(&mut b2);

		assert_ne!(b1, b2); // Reseeding changes output
		Ok(())
	}

	#[test]
	fn test_cpsrng_instance_non_determinism() -> Result<(), Error> {
		let rng1 = Cpsrng::new()?;
		let rng2 = Cpsrng::new()?;

		let mut b1 = [0u8; 32];
		rng1.gen(&mut b1);

		let mut b2 = [0u8; 32];
		rng2.gen(&mut b2);

		assert_ne!(b1, b2); // Different instances
		Ok(())
	}

	#[test]
	fn test_cpsrng_test_seed() -> Result<(), Error> {
		let key = [0u8; 32];
		let iv = [0u8; 16];

		let rng_test1 = Cpsrng::test_seed(key, iv)?;
		let mut b3 = [0u8; 32];
		rng_test1.gen(&mut b3);

		let rng_test2 = Cpsrng::test_seed(key, iv)?;
		let mut b4 = [0u8; 32];
		rng_test2.gen(&mut b4);

		assert_eq!(b3, b4); // Same for identical seeds

		let mut b5 = [0u8; 32];
		rng_test1.gen(&mut b5);
		assert_ne!(b3, b5); // Different for subsequent calls
		Ok(())
	}

	#[test]
	fn test_cpsrng_empty_buffer() -> Result<(), Error> {
		let rng = Cpsrng::new()?;
		let mut bytes = [0u8; 0];
		rng.gen(&mut bytes); // Should not crash
		Ok(())
	}

	#[test]
	fn test_cpsrng_large_buffer() -> Result<(), Error> {
		let rng = Cpsrng::new()?;
		let mut bytes = [0u8; 1024 * 1024]; // 1 MiB
		rng.gen(&mut bytes);
		assert_ne!(bytes, [0u8; 1024 * 1024]); // Non-zero output
		Ok(())
	}

	#[test]
	fn test_cpsrng1() -> Result<(), Error> {
		let mut rng = Cpsrng::new()?;

		let mut b1 = [0u8; 32];
		rng.gen(&mut b1);

		let mut b1a = [0u8; 32];
		rng.gen(&mut b1a);

		let mut b1b = [0u8; 32];
		rng.gen(&mut b1b);

		let mut b1c = [0u8; 32];
		rng.gen(&mut b1c);

		rng.reseed()?;

		let mut b2 = [0u8; 32];
		rng.gen(&mut b2);

		let rng_test = Cpsrng::test_seed([0u8; 32], [0u8; 16])?;

		let mut b3 = [0u8; 32];
		rng_test.gen(&mut b3);

		let rng_test = Cpsrng::test_seed([0u8; 32], [0u8; 16])?;

		let mut b4 = [0u8; 32];
		rng_test.gen(&mut b4);

		assert_ne!(b1, b1a);
		assert_ne!(b1a, b1b);
		assert_ne!(b1b, b1c);
		assert_ne!(b1, [0u8; 32]);

		assert_ne!(b1, b2);
		assert_ne!(b2, b3);
		assert_eq!(b3, b4);

		Ok(())
	}
}
