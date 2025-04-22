use crypto::constants::MAX_PROOF_SIZE;
use prelude::*;

#[derive(Clone, Copy)]
pub struct RangeProof {
	pub proof: [u8; MAX_PROOF_SIZE],
	pub plen: usize,
}

impl Hash for RangeProof {
	fn hash<H>(&self, h: &mut H)
	where
		H: Hasher,
	{
		self.proof.hash(h);
		self.plen.hash(h);
	}
}

impl AsRef<[u8]> for RangeProof {
	fn as_ref(&self) -> &[u8] {
		&self.proof
	}
}
