use crypto::{Commitment, Ctx, RangeProof, SecretKey};
use mw::transaction::Transaction;
use prelude::*;
use std::ffi::getmicros;

pub struct BlockHeader {
	hash: [u8; 32],
	prev_hash: [u8; 32],
	timestamp: u64,
}
pub struct Block {
	header: BlockHeader,
	tx: Transaction,
}

impl BlockHeader {
	fn new(prev_hash: [u8; 32]) -> Self {
		let timestamp = unsafe { getmicros() };
		let hash = [0u8; 32];
		Self {
			hash,
			prev_hash,
			timestamp,
		}
	}
}

impl Block {
	pub fn new(prev_hash: [u8; 32]) -> Self {
		let header = BlockHeader::new(prev_hash);
		let tx = Transaction::empty();
		Self { header, tx }
	}

	pub fn add_tx(&mut self, ctx: &Ctx, tx: Transaction) -> Result<(), Error> {
		self.tx.merge(ctx, tx)
	}

	pub fn with_coinbase(
		&self,
		commit: Commitment,
		range_proof: RangeProof,
	) -> Result<Self, Error> {
		Err(Error::new(Todo))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn test_block1() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let block = Block::new([0u8; 32]);

		let mut slate = Slate::new(10, SecretKey::gen(&ctx));

		let kc1 = KeyChain::from_seed([0u8; 32])?;
		let input = kc1.derive_key(&ctx, &[0, 0]);
		let change_output = kc1.derive_key(&ctx, &[0, 1]);

		// commit to the slate
		let user1_id = slate.commit(&mut ctx, &[(&input, 100)], &[(&change_output, 10)])?;

		// now it's user2's turn
		let kc2 = KeyChain::from_seed([1u8; 32])?;
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		// commit here we receive 80 coins (10 coin fee, 100 coin input and 10 coin change)
		let user2_id = slate.commit(&mut ctx, &[], &[(&output, 80)])?;

		// now user2 signs the transaction
		slate.sign(&mut ctx, user2_id, &[], &[&output])?;

		// now it's user1's turn to sign and finalize
		slate.sign(&mut ctx, user1_id, &[&input], &[&change_output])?;
		// finalize the slate
		let tx = slate.finalize(&mut ctx)?;

		Ok(())
	}
}
