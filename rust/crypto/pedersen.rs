use core::ptr::null;
use crypto::ctx::Ctx;
use crypto::errors::*;
use crypto::ffi::*;
use crypto::keys::{PublicKey, PublicKeyUncompressed};
use prelude::*;
use std::misc::bytes_to_hex_33;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Commitment([u8; 33]);
#[repr(C)]
pub struct CommitmentUncompressed([u8; 64]);

impl AsRaw<Self> for CommitmentUncompressed {
	fn as_ptr(&self) -> Ptr<Self> {
		Ptr::new(self.0.as_ptr() as *const Self)
	}
}

impl CommitmentUncompressed {
	pub fn zero() -> Self {
		Self([0u8; 64])
	}
}

impl AsRef<[u8]> for Commitment {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl Display for Commitment {
	fn format(&self, f: &mut Formatter) -> Result<()> {
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

	pub fn as_raw(&self) -> *mut Self {
		self.0.as_ptr() as *mut Self
	}

	pub fn decompress(&self, ctx: &Ctx) -> Result<CommitmentUncompressed> {
		let out = CommitmentUncompressed([0u8; 64]);
		unsafe {
			if secp256k1_pedersen_commitment_parse(
				ctx.as_ptr().raw(),
				out.as_ptr().raw(),
				self.as_raw(),
			) != 1
			{
				err!(OperationFailed)
			} else {
				Ok(out)
			}
		}
	}

	pub fn compress(ctx: &Ctx, key: CommitmentUncompressed) -> Result<Self> {
		let v = Self([0u8; 33]);
		let serialize_result = unsafe {
			secp256k1_pedersen_commitment_serialize(
				ctx.as_ptr().raw(),
				v.as_raw(),
				key.as_ptr().raw(),
			)
		};
		if serialize_result == 0 {
			err!(Serialization)
		} else {
			Ok(v)
		}
	}

	pub fn to_pubkey(&self, ctx: &Ctx) -> Result<PublicKey> {
		let pk = PublicKeyUncompressed::new([0u8; 64]);
		unsafe {
			if secp256k1_pedersen_commitment_to_pubkey(
				ctx.as_ptr().raw(),
				pk.as_ptr().raw(),
				self.as_raw(),
			) == 1
			{
				match PublicKey::compress(&ctx, pk) {
					Ok(pk) => Ok(pk),
					Err(e) => Err(e),
				}
			} else {
				err!(OperationFailed)
			}
		}
	}

	pub fn combine(&self, ctx: &Ctx, other: &Commitment) -> Result<Commitment> {
		let commit_out = CommitmentUncompressed([0u8; 64]);
		let mut pcommits = Vec::new();
		pcommits.push(self.decompress(ctx)?.as_ptr().raw() as *const CommitmentUncompressed)?;
		pcommits.push(other.decompress(ctx)?.as_ptr().raw() as *const CommitmentUncompressed)?;
		unsafe {
			if secp256k1_pedersen_commit_sum(
				ctx.as_ptr().raw(),
				commit_out.as_ptr().raw(),
				pcommits.as_ptr(),
				2,
				null(),
				0,
			) == 0
			{
				return err!(OperationFailed);
			}
		}
		Self::compress(ctx, commit_out)
	}
}
