use crypto::{Commitment, Signature};
use prelude::*;

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
