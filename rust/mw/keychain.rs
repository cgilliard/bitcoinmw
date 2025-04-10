use core::ptr::null;
use crypto::ctx::Ctx;
use crypto::ffi::{AES_CTR_xcrypt_buffer, AES_ctx_set_iv, AES_ctx_size, AES_init_ctx_iv};
use crypto::keys::SecretKey;
use prelude::*;
use std::ffi::{alloc, release};

pub struct KeyChain {
	aes256_ctx: *const u8,
}

impl Drop for KeyChain {
	fn drop(&mut self) {
		unsafe {
			if !self.aes256_ctx.is_null() {
				release(self.aes256_ctx);
				self.aes256_ctx = null();
			}
		}
	}
}

impl KeyChain {
	pub fn from_seed(seed: [u8; 32]) -> Result<Self, Error> {
		unsafe {
			let aes256_ctx = alloc(AES_ctx_size());
			if aes256_ctx.is_null() {
				Err(Error::new(Alloc))
			} else {
				AES_init_ctx_iv(aes256_ctx, seed.as_ptr(), [0u8; 16].as_ptr());
				Ok(Self { aes256_ctx })
			}
		}
	}

	pub fn derive_key(&self, ctx: &Ctx, path: &[u64; 2]) -> SecretKey {
		unsafe {
			AES_ctx_set_iv(self.aes256_ctx, path.as_ptr() as *const u8);
		}
		let mut skey = SecretKey([0u8; 32]);
		loop {
			unsafe {
				AES_CTR_xcrypt_buffer(self.aes256_ctx, skey.0.as_mut_ptr(), skey.0.len());
			}

			match ctx.verify_secret_key(&skey) {
				Ok(_) => break,
				Err(_) => {}
			}
		}
		skey
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_keychain() -> Result<(), Error> {
		let ctx = Ctx::new()?;
		let keychain = KeyChain::from_seed([0u8; 32])?;
		let sk1 = keychain.derive_key(&ctx, &[0, 0]);
		let sk2 = keychain.derive_key(&ctx, &[0, 1]);
		let sk3 = keychain.derive_key(&ctx, &[0, 0]);

		assert_eq!(sk1.0, sk3.0);
		assert!(sk1.0 != sk2.0);
		assert!(sk2.0 != sk3.0);

		assert_eq!(
			sk1.0,
			[
				220, 149, 192, 120, 162, 64, 137, 137, 173, 72, 162, 20, 146, 132, 32, 135, 83, 15,
				138, 251, 199, 69, 54, 185, 169, 99, 180, 241, 196, 203, 115, 139
			]
		);

		assert_eq!(
			sk2.0,
			[
				72, 22, 239, 227, 222, 179, 128, 86, 110, 186, 12, 23, 191, 88, 32, 144, 20, 206,
				210, 217, 87, 4, 25, 35, 117, 29, 227, 184, 47, 215, 121, 66
			]
		);

		assert_eq!(
			sk3.0,
			[
				220, 149, 192, 120, 162, 64, 137, 137, 173, 72, 162, 20, 146, 132, 32, 135, 83, 15,
				138, 251, 199, 69, 54, 185, 169, 99, 180, 241, 196, 203, 115, 139
			]
		);

		let keychain = KeyChain::from_seed([1u8; 32])?;
		let sk1 = keychain.derive_key(&ctx, &[0, 0]);
		let sk2 = keychain.derive_key(&ctx, &[0, 1]);
		let sk3 = keychain.derive_key(&ctx, &[0, 0]);

		assert_eq!(sk1.0, sk3.0);
		assert!(sk1.0 != sk2.0);
		assert!(sk2.0 != sk3.0);

		assert_eq!(
			sk1.0,
			[
				114, 152, 202, 165, 101, 3, 30, 173, 198, 206, 35, 210, 62, 166, 99, 120, 174, 168,
				174, 137, 68, 182, 57, 205, 8, 42, 222, 196, 254, 211, 34, 7
			]
		);

		assert_eq!(
			sk2.0,
			[
				20, 185, 184, 221, 204, 71, 105, 218, 0, 200, 249, 62, 90, 93, 140, 199, 184, 238,
				106, 47, 205, 118, 5, 47, 70, 31, 251, 72, 114, 67, 60, 93
			]
		);

		assert_eq!(
			sk3.0,
			[
				114, 152, 202, 165, 101, 3, 30, 173, 198, 206, 35, 210, 62, 166, 99, 120, 174, 168,
				174, 137, 68, 182, 57, 205, 8, 42, 222, 196, 254, 211, 34, 7
			]
		);

		let keychain = KeyChain::from_seed([0u8; 32])?;
		let sk1 = keychain.derive_key(&ctx, &[0, 0]);
		let sk2 = keychain.derive_key(&ctx, &[0, 1]);
		let sk3 = keychain.derive_key(&ctx, &[0, 0]);

		assert_eq!(sk1.0, sk3.0);
		assert!(sk1.0 != sk2.0);
		assert!(sk2.0 != sk3.0);

		assert_eq!(
			sk1.0,
			[
				220, 149, 192, 120, 162, 64, 137, 137, 173, 72, 162, 20, 146, 132, 32, 135, 83, 15,
				138, 251, 199, 69, 54, 185, 169, 99, 180, 241, 196, 203, 115, 139
			]
		);

		assert_eq!(
			sk2.0,
			[
				72, 22, 239, 227, 222, 179, 128, 86, 110, 186, 12, 23, 191, 88, 32, 144, 20, 206,
				210, 217, 87, 4, 25, 35, 117, 29, 227, 184, 47, 215, 121, 66
			]
		);

		assert_eq!(
			sk3.0,
			[
				220, 149, 192, 120, 162, 64, 137, 137, 173, 72, 162, 20, 146, 132, 32, 135, 83, 15,
				138, 251, 199, 69, 54, 185, 169, 99, 180, 241, 196, 203, 115, 139
			]
		);

		let keychain = KeyChain::from_seed([2u8; 32])?;
		let sk1 = keychain.derive_key(&ctx, &[1, 0]);
		let sk2 = keychain.derive_key(&ctx, &[0, 1]);
		let sk3 = keychain.derive_key(&ctx, &[1, 0]);

		assert_eq!(sk1.0, sk3.0);
		assert!(sk1.0 != sk2.0);
		assert!(sk2.0 != sk3.0);

		assert_eq!(
			sk1.0,
			[
				208, 82, 158, 124, 178, 155, 214, 161, 41, 255, 42, 34, 243, 72, 204, 25, 10, 197,
				194, 195, 190, 127, 102, 233, 138, 232, 250, 14, 12, 188, 12, 174
			]
		);

		assert_eq!(
			sk2.0,
			[
				54, 182, 151, 193, 14, 65, 206, 101, 69, 76, 10, 86, 152, 204, 149, 14, 183, 157,
				11, 88, 164, 234, 45, 211, 149, 249, 18, 13, 158, 4, 47, 22
			]
		);

		assert_eq!(
			sk3.0,
			[
				208, 82, 158, 124, 178, 155, 214, 161, 41, 255, 42, 34, 243, 72, 204, 25, 10, 197,
				194, 195, 190, 127, 102, 233, 138, 232, 250, 14, 12, 188, 12, 174
			]
		);

		let keychain = KeyChain::from_seed([3u8; 32])?;
		let sk1 = keychain.derive_key(&ctx, &[81, 80]);
		let sk2 = keychain.derive_key(&ctx, &[20, 11]);
		let sk3 = keychain.derive_key(&ctx, &[81, 80]);
		let sk4 = keychain.derive_key(&ctx, &[20, 11]);

		assert_eq!(sk1.0, sk3.0);
		assert!(sk1.0 != sk2.0);
		assert!(sk2.0 != sk3.0);
		assert_eq!(sk2.0, sk4.0);

		assert_eq!(
			sk1.0,
			[
				128, 201, 215, 8, 71, 167, 24, 94, 79, 170, 30, 250, 245, 55, 222, 255, 44, 58,
				109, 90, 151, 168, 38, 85, 118, 246, 10, 140, 234, 166, 139, 122
			]
		);

		assert_eq!(
			sk2.0,
			[
				29, 62, 167, 44, 19, 169, 171, 204, 52, 194, 30, 37, 224, 53, 85, 61, 38, 247, 3,
				232, 160, 148, 146, 166, 54, 81, 224, 18, 170, 169, 243, 41
			]
		);

		assert_eq!(
			sk3.0,
			[
				128, 201, 215, 8, 71, 167, 24, 94, 79, 170, 30, 250, 245, 55, 222, 255, 44, 58,
				109, 90, 151, 168, 38, 85, 118, 246, 10, 140, 234, 166, 139, 122
			]
		);

		assert_eq!(
			sk4.0,
			[
				29, 62, 167, 44, 19, 169, 171, 204, 52, 194, 30, 37, 224, 53, 85, 61, 38, 247, 3,
				232, 160, 148, 146, 166, 54, 81, 224, 18, 170, 169, 243, 41
			]
		);

		Ok(())
	}
}
