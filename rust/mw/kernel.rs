use crypto::ffi::secp256k1_schnorrsig_verify;
use crypto::{Commitment, Ctx, Message, Signature};
use prelude::*;
use std::misc::to_le_bytes_u64;

#[derive(Clone)]
pub struct Kernel {
	excess: Commitment,
	signature: Signature,
	fee: u64,
	features: u8,
}

impl Ord for Kernel {
	fn cmp(&self, other: &Self) -> Ordering {
		let c = self.excess.cmp(&other.excess);
		if c != Ordering::Equal {
			return c;
		}
		let c = self.signature.cmp(&other.signature);
		if c != Ordering::Equal {
			return c;
		}
		if self.fee < other.fee {
			return Ordering::Less;
		} else if self.fee > other.fee {
			return Ordering::Greater;
		}

		if self.features < other.features {
			return Ordering::Less;
		} else if self.features > other.features {
			return Ordering::Greater;
		}

		Ordering::Equal
	}
}

impl Display for Kernel {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		writeb!(f, "{}", self.excess)
	}
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

	pub fn sha3(&self, sha3: &mut Sha3) {
		// excess
		self.excess.sha3(sha3);

		// signature
		sha3.update(self.signature.as_ref());

		// fee
		let mut buf64 = [0u8; 8];
		to_le_bytes_u64(self.fee as u64, &mut buf64);
		sha3.update(&buf64);

		// features
		sha3.update(&[self.features]);
	}

	pub fn merge(&mut self, ctx: &Ctx, k: Kernel) -> Result<(), Error> {
		let commit = self.excess.clone();
		commit.add(ctx, &k.excess)?;
		self.excess = commit;
		Ok(())
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

	pub fn verify(&self, ctx: &Ctx, msg: &Message) -> Result<(), Error> {
		let excess = self.excess.to_pubkey(ctx)?.decompress(ctx)?;

		unsafe {
			let res = secp256k1_schnorrsig_verify(
				ctx.secp(),
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

#[cfg(test)]
mod test {
	use super::*;
	use crypto::SecretKey;

	#[test]
	fn test_kernel1() -> Result<(), Error> {
		let ctx = Ctx::new()?;
		let blind = SecretKey::gen(&ctx);
		let x = ctx.commit(0, &blind)?;
		let s = Signature::new();
		let kernel = Kernel::new(x, s, 10, 0);
		assert_eq!(kernel.fee(), 10);
		Ok(())
	}
}
