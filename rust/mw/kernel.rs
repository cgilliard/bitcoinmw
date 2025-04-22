use crypto::{Commitment, Signature};
use prelude::*;

pub struct Kernel {
	excess: Commitment,
	signature: Signature,
	fee: u64,
	features: u8,
}

impl PartialOrd for Kernel {
	fn partial_cmp(&self, other: &Kernel) -> Option<Ordering> {
		match self.excess.cmp(&other.excess) {
			Ordering::Less => return Some(Ordering::Less),
			Ordering::Greater => return Some(Ordering::Greater),
			_ => {}
		}

		match self.signature.cmp(&other.signature) {
			Ordering::Less => return Some(Ordering::Less),
			Ordering::Greater => return Some(Ordering::Greater),
			_ => {}
		}

		match self.fee.cmp(&other.fee) {
			Ordering::Less => return Some(Ordering::Less),
			Ordering::Greater => return Some(Ordering::Greater),
			_ => {}
		}

		match self.features.cmp(&other.features) {
			Ordering::Less => return Some(Ordering::Less),
			Ordering::Greater => return Some(Ordering::Greater),
			_ => {}
		}

		Some(Ordering::Equal)
	}
}

impl Eq for Kernel {}

impl PartialEq for Kernel {
	fn eq(&self, other: &Self) -> bool {
		if self.excess != other.excess {
			false
		} else if self.signature != other.signature {
			false
		} else if self.fee != other.fee {
			false
		} else if self.features != other.features {
			false
		} else {
			true
		}
	}
}

impl Ord for Kernel {
	fn cmp(&self, other: &Self) -> Ordering {
		match self.excess.cmp(&other.excess) {
			Ordering::Less => return Ordering::Less,
			Ordering::Greater => return Ordering::Greater,
			_ => {}
		}

		match self.signature.cmp(&other.signature) {
			Ordering::Less => return Ordering::Less,
			Ordering::Greater => return Ordering::Greater,
			_ => {}
		}

		match self.fee.cmp(&other.fee) {
			Ordering::Less => return Ordering::Less,
			Ordering::Greater => return Ordering::Greater,
			_ => {}
		}

		match self.features.cmp(&other.features) {
			Ordering::Less => return Ordering::Less,
			Ordering::Greater => return Ordering::Greater,
			_ => {}
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
