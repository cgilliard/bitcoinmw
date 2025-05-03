use core::convert::AsMut;
use core::slice::from_raw_parts;
use crypto::aes::Aes256;
use crypto::ctx::Ctx;
use crypto::keys::SecretKey;
use prelude::*;
use std::misc::{slice_copy, subslice};

pub struct KeyChain {
	aes: Aes256,
}

impl KeyChain {
	pub fn from_seed(seed: [u8; 48]) -> Result<Self> {
		let mut key = [0u8; 32];
		let mut iv = [0u8; 16];
		let seed_key = subslice(&seed, 0, 32)?;
		let seed_iv = subslice(&seed, 32, 16)?;
		slice_copy(&seed_key, &mut key, 32)?;
		slice_copy(&seed_iv, &mut iv, 16)?;
		let aes = Aes256::new(key, iv);
		Ok(Self { aes })
	}

	pub fn derive_key(&self, ctx: &Ctx, path: &[u64; 2]) -> SecretKey {
		let mut skey = SecretKey::zero();
		loop {
			let mut iv = [0u8; 16];
			let path_as_u8: &[u8] = unsafe { from_raw_parts(path.as_ptr() as *const u8, 16) };
			match slice_copy(path_as_u8, &mut iv, 16) {
				Ok(_) => {
					self.aes.set_iv(iv);
					self.aes.crypt(skey.as_mut());
					match skey.validate(ctx) {
						Ok(_) => break,
						Err(_) => {}
					}
				}
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
	fn test_keychain() -> Result<()> {
		let ctx = Ctx::new()?;
		let keychain = KeyChain::from_seed([0u8; 48])?;
		let sk1 = keychain.derive_key(&ctx, &[0, 0]);
		let sk2 = keychain.derive_key(&ctx, &[0, 1]);
		let sk3 = keychain.derive_key(&ctx, &[0, 0]);

		assert_eq!(sk1.as_ref(), sk3.as_ref());
		assert!(sk1.as_ref() != sk2.as_ref());
		assert!(sk2.as_ref() != sk3.as_ref());

		let keychain2 = KeyChain::from_seed([1u8; 48])?;
		let sk21 = keychain2.derive_key(&ctx, &[0, 0]);
		let sk22 = keychain2.derive_key(&ctx, &[0, 1]);
		let sk23 = keychain2.derive_key(&ctx, &[0, 0]);

		assert_eq!(sk21.as_ref(), sk23.as_ref());
		assert!(sk21.as_ref() != sk22.as_ref());
		assert!(sk22.as_ref() != sk23.as_ref());

		assert_ne!(sk1.as_ref(), sk21.as_ref());
		assert_ne!(sk2.as_ref(), sk22.as_ref());
		assert_ne!(sk3.as_ref(), sk23.as_ref());

		let keychain3 = KeyChain::from_seed([0u8; 48])?;
		let sk31 = keychain3.derive_key(&ctx, &[0, 0]);
		let sk32 = keychain3.derive_key(&ctx, &[0, 1]);
		let sk33 = keychain3.derive_key(&ctx, &[0, 0]);

		assert_eq!(sk31, sk1);
		assert_eq!(sk32, sk2);
		assert_eq!(sk33, sk3);

		Ok(())
	}
}
