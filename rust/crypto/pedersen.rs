use crypto::ctx::Ctx;
use crypto::ffi::*;
use crypto::keys::{PublicKey, PublicKeyUncompressed};
use prelude::*;

#[repr(C)]
pub struct Commitment(pub(crate) [u8; 33]);
#[repr(C)]
pub struct CommitmentUncompressed(pub(crate) [u8; 64]);

impl CommitmentUncompressed {
	pub(crate) fn as_ptr(&self) -> *const CommitmentUncompressed {
		self.0.as_ptr() as *const CommitmentUncompressed
	}

	pub(crate) fn as_mut_ptr(&mut self) -> *mut CommitmentUncompressed {
		self.0.as_mut_ptr() as *mut CommitmentUncompressed
	}
}

impl Commitment {
	pub fn decompress(&self, ctx: &Ctx) -> Result<CommitmentUncompressed, Error> {
		let mut out = CommitmentUncompressed([0u8; 64]);
		unsafe {
			if secp256k1_pedersen_commitment_parse(ctx.secp, out.as_mut_ptr(), self.as_ptr()) != 1 {
				Err(Error::new(InvalidCommitment))
			} else {
				Ok(out)
			}
		}
	}

	pub fn compress(ctx: &Ctx, key: CommitmentUncompressed) -> Result<Self, Error> {
		let mut v = Self([0u8; 33]);
		let serialize_result = unsafe {
			secp256k1_pedersen_commitment_serialize(ctx.secp, v.as_mut_ptr(), key.as_ptr())
		};
		if serialize_result == 0 {
			Err(Error::new(Serialization))
		} else {
			Ok(v)
		}
	}

	pub fn to_pubkey(&self, ctx: &Ctx) -> Result<PublicKey, Error> {
		let mut pk = PublicKeyUncompressed([0u8; 64]);
		unsafe {
			if secp256k1_pedersen_commitment_to_pubkey(ctx.secp, pk.as_mut_ptr(), self.as_ptr())
				== 1
			{
				match PublicKey::compress(&ctx, pk) {
					Ok(pk) => Ok(pk),
					Err(e) => Err(e),
				}
			} else {
				Err(Error::new(InvalidPublicKey))
			}
		}
	}

	pub fn add(&self, ctx: &Ctx, other: &Commitment) -> Result<Commitment, Error> {
		let pk1 = match self.to_pubkey(ctx) {
			Ok(pk1) => pk1,
			Err(e) => return Err(e),
		};
		let pk2 = match other.to_pubkey(ctx) {
			Ok(pk2) => pk2,
			Err(e) => return Err(e),
		};
		match pk1.add(ctx, &pk2) {
			Ok(sum_pk) => Ok(Commitment(sum_pk.0)),
			Err(e) => return Err(e),
		}
	}

	pub(crate) fn as_mut_ptr(&mut self) -> *mut Commitment {
		self.0.as_mut_ptr() as *mut Commitment
	}

	pub(crate) fn as_ptr(&self) -> *const Commitment {
		self.0.as_ptr() as *const Commitment
	}
}
