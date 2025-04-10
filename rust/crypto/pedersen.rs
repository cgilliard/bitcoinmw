use core::ptr::null;
use crypto::ctx::Ctx;
use crypto::ffi::*;
use crypto::keys::{PublicKey, PublicKeyUncompressed};
use prelude::*;

#[repr(C)]
#[derive(Clone)]
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

	pub fn add(&self, ctx: &mut Ctx, other: &Commitment) -> Result<Commitment, Error> {
		let mut commit_out = CommitmentUncompressed([0u8; 64]);
		let mut pcommits = Vec::new();
		pcommits.push(self.decompress(ctx)?.as_ptr())?;
		pcommits.push(other.decompress(ctx)?.as_ptr())?;
		unsafe {
			if secp256k1_pedersen_commit_sum(
				ctx.secp,
				commit_out.as_mut_ptr(),
				pcommits.as_ptr(),
				2,
				null(),
				0,
			) == 0
			{
				return Err(Error::new(InvalidCommitment));
			}
		}
		Self::compress(ctx, commit_out)
	}

	pub(crate) fn as_mut_ptr(&mut self) -> *mut Commitment {
		self.0.as_mut_ptr() as *mut Commitment
	}

	pub(crate) fn as_ptr(&self) -> *const Commitment {
		self.0.as_ptr() as *const Commitment
	}
}
