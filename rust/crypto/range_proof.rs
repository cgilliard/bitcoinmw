use crypto::constants::MAX_PROOF_SIZE;

#[derive(Clone)]
pub struct RangeProof {
	pub proof: [u8; MAX_PROOF_SIZE],
	pub plen: usize,
}

impl RangeProof {
	pub fn as_ref(&self) -> &[u8; MAX_PROOF_SIZE] {
		&self.proof
	}
}
