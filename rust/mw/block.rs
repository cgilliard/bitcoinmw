use crypto::{Ctx, PublicKey, SecretKey};
use mw::kernel::Kernel;
use mw::transaction::Transaction;
use prelude::*;
use std::ffi::getmicros;

#[derive(Clone)]
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
		ctx: &mut Ctx,
		output_blind: &SecretKey,
		overage: u64,
	) -> Result<Self, Error> {
		let (mut coinbase, excess_blind) = match self.tx.offset() {
			Some(offset) => {
				let mut noffset = offset.clone();
				noffset.negate(ctx)?;
				let excess_blind = ctx.blind_sum(&[], &[output_blind, &noffset])?;
				(Transaction::new(noffset), excess_blind)
			}
			None => {
				let excess_blind = ctx.blind_sum(&[], &[output_blind])?;
				(Transaction::empty(), excess_blind)
			}
		};
		let v = self.fees() + overage;
		let output = ctx.commit(v, output_blind)?;
		let range_proof = ctx.range_proof(v, output_blind)?;
		let excess = ctx.commit(0, &excess_blind)?;
		let msg = ctx.hash_kernel(&excess, 0, 0)?;
		let nonce = SecretKey::gen(ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign_single(&msg, &excess_blind, &nonce, &pubnonce, &pubkey)?;
		let kernel = Kernel::new(excess, sig, 0, 0);
		coinbase.add_kernel(kernel)?;
		coinbase.add_output(output, range_proof)?;
		coinbase.verify(ctx, v)?;
		let mut tx = self.tx.try_clone()?;
		tx.merge(ctx, coinbase)?;
		tx.set_offset_zero();
		tx.verify(ctx, overage)?;
		let header = self.header.clone();
		Ok(Block { header, tx })
	}

	pub fn fees(&self) -> u64 {
		let mut fees = 0;
		for k in self.tx.kernels() {
			fees += k.fee();
		}
		fees
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn test_block1() -> Result<(), Error> {
		// create a block (specify prev_hash)
		let prev_hash = [0u8; 32];
		let mut block = Block::new(prev_hash);
		// create a crypto ctx
		let mut ctx = Ctx::new()?;

		// create a couple of txns for our block

		// generate an offset for our slate
		let offset = SecretKey::gen(&ctx);
		// set fee to 10
		let fee = 10;
		// create slate
		let mut slate = Slate::new(fee, offset);

		// user 1's keychain from seed 0u8
		let kc1 = KeyChain::from_seed([0u8; 32])?;
		// create an input from user1's keychain
		let input = kc1.derive_key(&ctx, &[0, 0]);
		// create a change output
		let change_output = kc1.derive_key(&ctx, &[0, 1]);

		// commit user's values
		let user1_id = slate.commit(&mut ctx, &[(&input, 100)], &[(&change_output, 10)])?;

		// send to user2
		// user2's keychain from seed 1u8
		let kc2 = KeyChain::from_seed([1u8; 32])?;
		// create an output key
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		// commit to transaction receiving 80 coins.
		let user2_id = slate.commit(&mut ctx, &[], &[(&output, 80)])?;
		// user 2 signs, then sends back to user1 to sign and finalize the txn
		slate.sign(&mut ctx, user2_id, &[], &[&output])?;
		slate.sign(&mut ctx, user1_id, &[&input], &[&change_output])?;
		let tx = slate.finalize(&mut ctx)?;

		// verify the transaction is valid with overage = 0
		assert!(tx.verify(&mut ctx, 0).is_ok());

		// add tx to our block
		block.add_tx(&ctx, tx)?;

		// assert the block's txn is as expected
		assert_eq!(block.tx.outputs().len(), 2);
		assert_eq!(block.tx.inputs().len(), 1);
		assert_eq!(block.tx.kernels().len(), 1);

		// create a second transaction in similar fashion with slightly different
		// fees/amounts.
		let offset = SecretKey::gen(&ctx);
		let mut slate = Slate::new(20, offset);

		let kc1 = KeyChain::from_seed([2u8; 32])?;
		let input = kc1.derive_key(&ctx, &[0, 0]);
		let change_output = kc1.derive_key(&ctx, &[0, 1]);

		let user1_id = slate.commit(&mut ctx, &[(&input, 200)], &[(&change_output, 10)])?;

		let kc2 = KeyChain::from_seed([3u8; 32])?;
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		let user2_id = slate.commit(&mut ctx, &[], &[(&output, 170)])?;
		slate.sign(&mut ctx, user2_id, &[], &[&output])?;
		slate.sign(&mut ctx, user1_id, &[&input], &[&change_output])?;
		let tx2 = slate.finalize(&mut ctx)?;
		assert!(tx2.verify(&mut ctx, 0).is_ok());

		// add the second txn to our block
		block.add_tx(&ctx, tx2)?;

		// confirm outputs/inputs/kenrels
		assert_eq!(block.tx.outputs().len(), 4);
		assert_eq!(block.tx.inputs().len(), 2);
		assert_eq!(block.tx.kernels().len(), 2);

		// verify the block's transaction is valud with overage = 0
		assert!(block.tx.verify(&mut ctx, 0).is_ok());

		// create a keychain for our miner
		let miner_keychain = KeyChain::from_seed([4u8; 32])?;
		// set overage for this block height. We use 1000.
		let overage = 1000;
		// generate a blind from our keychain
		let coinbase_blind = miner_keychain.derive_key(&ctx, &[0, 0]);
		// complete the block with the coinbas added
		let complete = block.with_coinbase(&mut ctx, &coinbase_blind, overage)?;

		// verify the block's transaction
		assert!(complete.tx.verify(&mut ctx, overage).is_ok());
		assert!(complete.tx.offset().is_none());
		assert_eq!(complete.tx.outputs().len(), 5);
		assert_eq!(complete.tx.inputs().len(), 2);
		assert_eq!(complete.tx.kernels().len(), 3);

		Ok(())
	}

	#[test]
	fn mine_empty_block() -> Result<(), Error> {
		// create a block (specify prev_hash)
		let prev_hash = [0u8; 32];
		let block = Block::new(prev_hash);
		// create a crypto ctx
		let mut ctx = Ctx::new()?;

		// confirm outputs/inputs/kernels
		assert_eq!(block.tx.outputs().len(), 0);
		assert_eq!(block.tx.inputs().len(), 0);
		assert_eq!(block.tx.kernels().len(), 0);

		// mine an empty block (only coinbase)
		let miner_keychain = KeyChain::from_seed([4u8; 32])?;
		// set overage for this block height. We use 1000.
		let overage = 1000;
		// generate a blind from our keychain
		let coinbase_blind = miner_keychain.derive_key(&ctx, &[0, 0]);
		// complete the block with the coinbase added
		let complete = block.with_coinbase(&mut ctx, &coinbase_blind, overage)?;
		// verify coinbase
		assert_eq!(complete.tx.outputs().len(), 1);
		assert_eq!(complete.tx.kernels().len(), 1);
		assert_eq!(complete.tx.inputs().len(), 0);
		assert!(complete.tx.verify(&mut ctx, overage).is_ok());

		Ok(())
	}
}
