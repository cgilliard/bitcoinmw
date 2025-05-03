use crypto::ctx::Ctx;
use crypto::ffi::secp256k1_schnorrsig_verify;
use crypto::pedersen::Commitment;
use crypto::sha3::Sha3_256;
use crypto::signature::{Message, Signature};
use mw::errors::*;
use prelude::*;
use std::misc::to_be_bytes_u64;

#[derive(Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct Kernel {
	excess: Commitment,
	signature: Signature,
	fee: u64,
	features: u8,
}

impl Kernel {
	pub fn new(excess: Commitment, signature: Signature, fee: u64, features: u8) -> Self {
		Self {
			excess,
			signature,
			fee,
			features,
		}
	}

	pub fn fee(&self) -> u64 {
		self.fee
	}

	pub fn signature(&self) -> &Signature {
		&self.signature
	}

	pub fn excess(&self) -> &Commitment {
		&self.excess
	}

	pub fn features(&self) -> u8 {
		self.features
	}

	pub fn validate(&self, ctx: &Ctx) -> Result<()> {
		let msg = self.message();
		let excess = self.excess.to_pubkey(ctx)?.decompress(ctx)?;

		let res = unsafe {
			secp256k1_schnorrsig_verify(
				ctx.as_ptr(),
				self.signature.as_ptr(),
				msg.as_ptr(),
				excess.as_ptr(),
			)
		};

		if res == 1 {
			Ok(())
		} else {
			err!(ValidationFailed)
		}
	}

	pub fn message(&self) -> Message {
		Self::message_for(self.excess(), self.fee(), self.features())
	}

	pub fn message_for(excess: &Commitment, fee: u64, features: u8) -> Message {
		let sha3 = Sha3_256::new();

		// exccess
		sha3.update(excess.as_ref());

		// fee
		let mut buf64 = [0u8; 8];
		// only error occurs when length is not equal to 8 bytes
		let _ = to_be_bytes_u64(fee, &mut buf64);
		sha3.update(&buf64);

		// features
		sha3.update(&[features]);

		// finalize
		Message::new(sha3.finalize())
	}

	pub fn sha3(&self, sha3: &Sha3_256) {
		sha3.update(self.message().as_ref());
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crypto::ctx::Ctx;
	use crypto::keys::{PublicKey, SecretKey};

	#[test]
	fn test_kernel1() -> Result<()> {
		let ctx = Ctx::new()?;
		let fee = 10;
		let features = 0;
		let blind = SecretKey::gen(&ctx);
		let excess = ctx.commit(0, &blind)?;
		let message = Kernel::message_for(&excess, fee, features);
		let secnonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &secnonce)?;
		let pubkey = excess.to_pubkey(&ctx)?;
		let s = ctx.sign(&message, &blind, &secnonce, &pubnonce, &pubkey)?;
		let kernel = Kernel::new(excess, s, fee, features);
		assert_eq!(kernel.fee(), 10);

		assert!(kernel.validate(&ctx).is_ok());
		Ok(())
	}
}
