use crypto::{Commitment, Signature};
use prelude::*;

#[derive(Ord, PartialOrd, PartialEq, Eq)]
pub struct Kernel {
	excess: Commitment,
	signature: Signature,
	fee: u64,
	features: u8,
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
}
