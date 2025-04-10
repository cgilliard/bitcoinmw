use crypto::ctx::Ctx;
use crypto::ffi::secp256k1_schnorrsig_verify;
use crypto::keys::{Message, Signature};
use crypto::pedersen::Commitment;
use prelude::*;

#[derive(Clone)]
pub struct Kernel {
	excess: Commitment,
	signature: Signature,
	fee: u64,
}

impl Kernel {
	pub fn new(excess: Commitment, signature: Signature, fee: u64) -> Self {
		Self {
			excess,
			signature,
			fee,
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

	pub fn verify(&self, ctx: &mut Ctx, msg: &Message) -> Result<(), Error> {
		let excess = self.excess.to_pubkey(ctx)?.decompress(ctx)?;

		unsafe {
			let res = secp256k1_schnorrsig_verify(
				ctx.secp,
				self.signature.as_ptr(),
				msg.as_ptr(),
				excess.as_ptr(),
			);

			if res == 1 {
				Ok(())
			} else {
				Err(Error::new(InvalidSignature))
			}
		}
	}
}
