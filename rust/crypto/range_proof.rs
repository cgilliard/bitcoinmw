use crypto::constants::MAX_PROOF_SIZE;
use prelude::*;
use std::misc::to_le_bytes_u64;

#[derive(Clone)]
pub struct RangeProof {
	pub proof: [u8; MAX_PROOF_SIZE],
	pub plen: usize,
}

impl RangeProof {
	pub fn as_ref(&self) -> &[u8; MAX_PROOF_SIZE] {
		&self.proof
	}

	pub fn sha3(&self, sha3: &mut Sha3) {
		sha3.update(&self.proof);
		let mut buf64 = [0u8; 8];
		to_le_bytes_u64(self.plen as u64, &mut buf64);
		sha3.update(&buf64);
	}
}
