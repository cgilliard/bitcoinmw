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

impl AsRaw<Self> for CommitmentUncompressed {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl CommitmentUncompressed {
	pub fn zero() -> Self {
		Self([0u8; 64])
	}
}

impl AsRaw<Self> for Commitment {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl AsRef<[u8]> for Commitment {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl Display for Commitment {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let b = bytes_to_hex_33(&self.0);
		for i in 0..66 {
			writef!(f, "{}", b[i] as char)?;
		}
		Ok(())
	}
}

impl PartialEq for Commitment {
	fn eq(&self, other: &Self) -> bool {
		for i in 0..self.0.len() {
			if self.0[i] != other.0[i] {
				return false;
			}
		}
		true
	}
}

impl PartialOrd for Commitment {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		let len = self.0.len();
		for i in 0..len {
			if self.0[i] < other.0[i] {
				return Some(Ordering::Less);
			} else if self.0[i] > other.0[i] {
				return Some(Ordering::Greater);
			}
		}
		Some(Ordering::Equal)
	}
}

impl Eq for Commitment {}

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

impl Hash for Commitment {
	fn hash<H>(&self, h: &mut H)
	where
		H: Hasher,
	{
		self.0.hash(h);
	}
}

impl Commitment {
	pub fn zero() -> Self {
		Self([0u8; 33])
	}

	pub fn decompress(&self, ctx: &Ctx) -> Result<CommitmentUncompressed, Error> {
		let mut out = CommitmentUncompressed([0u8; 64]);
		unsafe {
			if secp256k1_pedersen_commitment_parse(ctx.as_ptr(), out.as_mut_ptr(), self.as_ptr())
				!= 1
			{
				Err(Error::new(OperationFailed))
			} else {
				Ok(out)
			}
		}
	}

	pub fn compress(ctx: &Ctx, key: CommitmentUncompressed) -> Result<Self, Error> {
		let mut v = Self([0u8; 33]);
		let serialize_result = unsafe {
			secp256k1_pedersen_commitment_serialize(ctx.as_ptr(), v.as_mut_ptr(), key.as_ptr())
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
			if secp256k1_pedersen_commitment_to_pubkey(ctx.as_ptr(), pk.as_mut_ptr(), self.as_ptr())
				== 1
			{
				match PublicKey::compress(&ctx, pk) {
					Ok(pk) => Ok(pk),
					Err(e) => Err(e),
				}
			} else {
				Err(Error::new(OperationFailed))
			}
		}
	}

	pub fn combine(&self, ctx: &Ctx, other: &Commitment) -> Result<Commitment, Error> {
		let mut commit_out = CommitmentUncompressed([0u8; 64]);
		let mut pcommits = Vec::new();
		pcommits.push(self.decompress(ctx)?.as_ptr())?;
		pcommits.push(other.decompress(ctx)?.as_ptr())?;
		unsafe {
			if secp256k1_pedersen_commit_sum(
				ctx.as_ptr(),
				commit_out.as_mut_ptr(),
				pcommits.as_ptr(),
				2,
				null(),
				0,
			) == 0
			{
				return Err(Error::new(OperationFailed));
			}
		}
		Self::compress(ctx, commit_out)
	}
}
