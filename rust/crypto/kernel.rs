use crypto::constants::SCRATCH_SPACE_SIZE;
use crypto::ctx::Ctx;
use crypto::ffi::{
	secp256k1_schnorrsig_verify_batch, secp256k1_scratch_space_create,
	secp256k1_scratch_space_destroy,
};
use crypto::keys::{Message, Signature};
use crypto::pedersen::Commitment;
use prelude::*;

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

		let sigs = vec![self.signature.as_ptr()]?;
		let msgs = vec![msg.as_ptr()]?;
		let pubkeys = vec![excess.as_ptr()]?;

		unsafe {
			let scratch = secp256k1_scratch_space_create(ctx.secp, SCRATCH_SPACE_SIZE);
			if scratch.is_null() {
				return Err(Error::new(Alloc));
			}
			let res = secp256k1_schnorrsig_verify_batch(
				ctx.secp,
				scratch,
				sigs.as_ptr(),
				msgs.as_ptr(),
				pubkeys.as_ptr(),
				1,
			);

			secp256k1_scratch_space_destroy(scratch);

			if res == 1 {
				Ok(())
			} else {
				Err(Error::new(InvalidSignature))
			}
		}
	}
}
