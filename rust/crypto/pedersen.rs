use core::ptr::null;
use crypto::ctx::Ctx;
use crypto::ffi::*;
use crypto::keys::PublicKeyUncompressed;
use crypto::PublicKey;
use prelude::*;
use std::misc::bytes_to_hex_33;

#[repr(C)]
#[derive(Clone)]
pub struct Commitment([u8; 33]);
#[repr(C)]
pub struct CommitmentUncompressed([u8; 64]);

impl CommitmentUncompressed {
	pub fn new(v: [u8; 64]) -> Self {
		Self(v)
	}
	pub fn as_ptr(&self) -> *const CommitmentUncompressed {
		self.0.as_ptr() as *const CommitmentUncompressed
	}

	pub fn as_mut_ptr(&mut self) -> *mut CommitmentUncompressed {
		self.0.as_mut_ptr() as *mut CommitmentUncompressed
	}
}

impl Display for Commitment {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let b = bytes_to_hex_33(&self.0);
		for i in 0..66 {
			writeb!(f, "{}", b[i] as char)?;
		}
		Ok(())
	}
}

impl Ord for Commitment {
	fn cmp(&self, other: &Self) -> Ordering {
		let len = self.0.len();
		for i in 0..len {
			if self.0[i] < other.0[i] {
				return Ordering::Less;
			} else if self.0[i] > other.0[i] {
				return Ordering::Greater;
			}
		}
		Ordering::Equal
	}
}

impl Commitment {
	pub fn new(v: [u8; 33]) -> Self {
		Self(v)
	}

	pub fn sha3(&self, sha3: &mut Sha3) {
		sha3.update(&self.0);
	}

	pub fn decompress(&self, ctx: &Ctx) -> Result<CommitmentUncompressed, Error> {
		let mut out = CommitmentUncompressed([0u8; 64]);
		unsafe {
			if secp256k1_pedersen_commitment_parse(ctx.secp(), out.as_mut_ptr(), self.as_ptr()) != 1
			{
				Err(Error::new(InvalidCommitment))
			} else {
				Ok(out)
			}
		}
	}

	pub fn compress(ctx: &Ctx, key: CommitmentUncompressed) -> Result<Self, Error> {
		let mut v = Self([0u8; 33]);
		let serialize_result = unsafe {
			secp256k1_pedersen_commitment_serialize(ctx.secp(), v.as_mut_ptr(), key.as_ptr())
		};
		if serialize_result == 0 {
			Err(Error::new(Serialization))
		} else {
			Ok(v)
		}
	}

	pub fn to_pubkey(&self, ctx: &Ctx) -> Result<PublicKey, Error> {
		let mut pk = PublicKeyUncompressed::new([0u8; 64]);
		unsafe {
			if secp256k1_pedersen_commitment_to_pubkey(ctx.secp(), pk.as_mut_ptr(), self.as_ptr())
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
		let mut commit_out = CommitmentUncompressed([0u8; 64]);
		let mut pcommits = Vec::new();
		pcommits.push(self.decompress(ctx)?.as_ptr())?;
		pcommits.push(other.decompress(ctx)?.as_ptr())?;
		unsafe {
			if secp256k1_pedersen_commit_sum(
				ctx.secp(),
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

	pub fn as_mut_ptr(&mut self) -> *mut Commitment {
		self.0.as_mut_ptr() as *mut Commitment
	}

	pub fn as_ptr(&self) -> *const Commitment {
		self.0.as_ptr() as *const Commitment
	}

	pub fn as_ref(&self) -> &[u8; 33] {
		&self.0
	}
}
