use crypto::ctx::Ctx;
use crypto::ffi::secp256k1_ecdh;
use crypto::keys::{PublicKey, SecretKey};
use prelude::*;

#[repr(C)]
#[derive(PartialEq)]
pub struct SharedSecret([u8; 32]);

impl Debug for SharedSecret {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> FmtResult {
		#[cfg(test)]
		write!(_f, "SharedSecret(****)")?;
		Ok(())
	}
}

impl SharedSecret {
	pub fn new(ctx: &Ctx, exchanged: &PublicKey, local: &SecretKey) -> Result<SharedSecret> {
		let mut ss = Self([0u8; 32]);
		let pubkey = exchanged.decompress(ctx)?;

		unsafe {
			if secp256k1_ecdh(ctx.as_ptr(), &mut ss, pubkey.as_ptr(), local.as_ptr()) != 1 {
				err!(OperationFailed)
			} else {
				Ok(ss)
			}
		}
	}

	pub fn as_ptr(&self) -> *const Self {
		&self.0 as *const u8 as *const Self
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_ecdh1() -> Result<()> {
		let ctx = Ctx::new()?;
		let skey1 = SecretKey::gen(&ctx);
		let skey2 = SecretKey::gen(&ctx);
		let pubkey1 = PublicKey::from(&ctx, &skey1)?;
		let pubkey2 = PublicKey::from(&ctx, &skey2)?;

		// positive
		let ss1 = SharedSecret::new(&ctx, &pubkey1, &skey2)?;
		let ss2 = SharedSecret::new(&ctx, &pubkey2, &skey1)?;
		assert_eq!(ss1, ss2);

		// negative
		let skey3 = SecretKey::gen(&ctx);
		let ssbad = SharedSecret::new(&ctx, &pubkey2, &skey3)?;
		assert_ne!(ssbad, ss1);

		Ok(())
	}
}
