use crypto::aes::Aes256;
use crypto::ffi::{generate_matrix, heavyhash};
use prelude::*;

pub struct Bip52 {
	matrix: [u16; 4096],
	aes: Aes256,
}

impl Bip52 {
	pub fn new(key: [u8; 32], prev_hash: [u8; 32]) -> Self {
		// init with 0s for iv, we update in ret.reset below
		let aes = Aes256::new(key, [0u8; 16]);
		let matrix = [0u16; 4096];
		let mut ret = Self { matrix, aes };
		ret.reset(prev_hash);
		ret
	}

	pub fn reset(&mut self, prev_hash: [u8; 32]) {
		let mut iv = [0u8; 16];
		match prev_hash.subslice(16, 16) {
			Ok(ivv) => {
				let _ = iv.slice_copy(ivv);
			}
			Err(e) => exit!("unexpected error copying slice: {}", e),
		}
		self.aes.set_iv(iv);
		unsafe {
			generate_matrix(self.matrix.as_mut_ptr(), self.aes.as_ptr());
		}
	}

	#[inline]
	pub fn hash(&self, b: &[u8]) -> [u8; 32] {
		let mut ret = [0u8; 32];
		unsafe {
			heavyhash(self.matrix.as_ptr(), b.as_ptr(), b.len(), ret.as_mut_ptr());
		}
		ret
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::aes::Aes256;
	use crypto::cpsrng::Cpsrng;
	use crypto::ffi::{generate_matrix, heavyhash};

	#[test]
	fn test_bip52_struct() -> Result<()> {
		let bip52_key = [9u8; 32];
		//println!("");
		let mut bip52 = Bip52::new(bip52_key, [0u8; 32]); // generate matrix with previous block hash
		let result1 = bip52.hash("hello".as_bytes());
		//println!("heavyhash(hello, 0u8)={}", result1);
		let result1a = bip52.hash("hello".as_bytes());
		//println!("heavyhash(hello, 0u8)={}", result1a);
		assert_eq!(result1, result1a);
		let result1b = bip52.hash("hello2".as_bytes());
		assert_ne!(result1a, result1b);
		//println!("heavyhash(hello2, 0u8)={}", result1b);
		bip52.reset([1u8; 32]); // generate new matrix based on previous block hash (new
						  // block arrived)
		let result2 = bip52.hash("hello".as_bytes());
		//println!("heavyhash(hello, 1u8)={}", result2);
		assert_ne!(result1, result2);
		let result2a = bip52.hash("hello2".as_bytes());
		assert_ne!(result2a, result2);
		//println!("heavyhash(hello2, 1u8)={}", result2a);

		Ok(())
	}

	#[test]
	fn test_keccak_bip52() -> Result<()> {
		let rng = Cpsrng::new()?;
		for _ in 0..255 {
			let mut key = [0u8; 32];
			let mut iv = [0u8; 16];
			rng.gen(&mut key);
			rng.gen(&mut iv);
			let aes = Aes256::new(key, iv);
			let mut matrix = [0u16; 4096];
			let mut hash_out = [0u8; 32];
			let pdata = [1u8; 32];
			unsafe {
				generate_matrix(matrix.as_mut_ptr(), aes.as_ptr());
				heavyhash(matrix.as_ptr(), pdata.as_ptr(), 32, hash_out.as_mut_ptr());
			}
			//println!("hash_out={}", hash_out);
		}

		Ok(())
	}
}
