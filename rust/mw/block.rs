use crypto::{Ctx, PublicKey, SecretKey};
use mw::constants::BLOCK_HEADER_VERSION;
use mw::kernel::Kernel;
use mw::transaction::Transaction;
use prelude::*;
use std::ffi::getmicros;
use std::misc::{to_le_bytes_u32, to_le_bytes_u64, u256_less_than_or_equal};

#[derive(Clone)]
pub struct BlockHeader {
	// Header Version. Currently = 0
	header_version: u8,
	// hash of the current block
	hash: [u8; 32],
	// hash of prev block
	prev_hash: [u8; 32],
	// height of block
	height: u32,
	// timestamp in microseconds since epoch (ISO 8601) 00:00:00 Jan 1, 1970
	timestamp: u64,
	// maximum value allowed for the block hash
	max_hash_value: [u8; 32],
	// nonce (for calculating pow)
	nonce: u64,
}

pub struct Block {
	// The block's header
	header: BlockHeader,
	// The block's transaction
	tx: Transaction,
}

impl BlockHeader {
	fn new(prev_hash: [u8; 32], height: u32, max_hash_value: [u8; 32]) -> Self {
		let header_version = BLOCK_HEADER_VERSION;
		let timestamp = unsafe { getmicros() };
		let hash = [0u8; 32];
		let nonce = 0;
		Self {
			header_version,
			hash,
			prev_hash,
			height,
			timestamp,
			max_hash_value,
			nonce,
		}
	}
}

impl Block {
	pub fn new(prev_hash: [u8; 32], height: u32, max_hash_value: [u8; 32]) -> Self {
		let header = BlockHeader::new(prev_hash, height, max_hash_value);
		let tx = Transaction::empty();
		Self { header, tx }
	}

	pub fn add_tx(&mut self, ctx: &Ctx, tx: Transaction) -> Result<(), Error> {
		self.tx.merge(ctx, tx)
	}

	#[inline]
	pub fn mine_block(&mut self, ctx: &mut Ctx, nonce: u64, iterations: u64) -> Result<(), Error> {
		self.header.nonce = nonce;
		for _i in 0..iterations {
			let hash = self.calculate_hash(ctx)?;

			if u256_less_than_or_equal(&self.header.max_hash_value, &hash) {
				// difficulty met - block found
				self.header.hash = hash;
				return Ok(());
			}
			self.header.nonce = self.header.nonce.wrapping_add(1);
		}
		Err(Error::new(BlockNotFound))
	}

	pub fn validate_hash(&self, ctx: &mut Ctx) -> Result<(), Error> {
		if !u256_less_than_or_equal(&self.header.max_hash_value, &self.header.hash) {
			Err(Error::new(InsufficientBlockHash))
		} else {
			let hash = self.calculate_hash(ctx)?;
			if hash != self.header.hash {
				Err(Error::new(InvalidBlockHash))
			} else {
				Ok(())
			}
		}
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

	#[inline]
	fn calculate_hash(&self, ctx: &mut Ctx) -> Result<[u8; 32], Error> {
		let sha3 = ctx.sha3();
		sha3.reset();
		let mut buf32 = [0u8; 4];
		let mut buf64 = [0u8; 8];

		// header_version
		sha3.update(&[self.header.header_version]);

		// prev_hash
		sha3.update(&self.header.prev_hash);

		// height
		to_le_bytes_u32(self.header.height, &mut buf32);
		sha3.update(&buf32[..]);

		// timestamp
		to_le_bytes_u64(self.header.timestamp, &mut buf64);
		sha3.update(&buf64[..]);

		// max_hash_value
		sha3.update(&self.header.max_hash_value);

		// nonce
		to_le_bytes_u64(self.header.nonce, &mut buf64);
		sha3.update(&buf64);

		// TODO: need to sort these

		// Block transaction
		for kernel in self.tx.kernels() {
			kernel.sha3(sha3);
		}
		for output_pair in self.tx.outputs() {
			let output = &output_pair.0;
			let range_proof = &output_pair.1;
			output.sha3(sha3);
			range_proof.sha3(sha3);
		}
		for input in self.tx.inputs() {
			input.sha3(sha3);
		}

		let mut ret = [0u8; 32];
		sha3.finalize(&mut ret)?;
		Ok(ret)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use mw::constants::{DIFFICULTY_8BIT_LEADING, DIFFICULTY_HARD, INITIAL_DIFFICULTY};

	#[test]
	fn test_block1() -> Result<(), Error> {
		// create a block (specify prev_hash)
		let prev_hash = [0u8; 32];
		let mut block = Block::new(prev_hash, 1, INITIAL_DIFFICULTY);
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
		let block = Block::new(prev_hash, 1, INITIAL_DIFFICULTY);
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

	#[test]
	fn test_mining() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let prev_hash = [0u8; 32];
		let block = Block::new(prev_hash, 1, DIFFICULTY_8BIT_LEADING);

		// mine an empty block (only coinbase)
		let miner_keychain = KeyChain::from_seed([4u8; 32])?;
		// set overage for this block height. We use 1000.
		let overage = 1000;
		// generate a blind from our keychain
		let coinbase_blind = miner_keychain.derive_key(&ctx, &[0, 0]);
		// complete the block with the coinbase added
		let mut complete = block.with_coinbase(&mut ctx, &coinbase_blind, overage)?;

		assert!(complete.header.hash == [0u8; 32]);
		assert!(complete.mine_block(&mut ctx, 0, 1024 * 1024).is_ok());
		assert!(complete.validate_hash(&mut ctx).is_ok());
		assert!(complete.header.hash != [0u8; 32]);
		assert!(complete.header.hash[0] == 0);

		// try something too difficult
		let block = Block::new(prev_hash, 1, DIFFICULTY_HARD);

		let mut complete = block.with_coinbase(&mut ctx, &coinbase_blind, overage)?;
		// verify coinbase
		assert_eq!(complete.tx.outputs().len(), 1);
		assert_eq!(complete.tx.kernels().len(), 1);
		assert_eq!(complete.tx.inputs().len(), 0);
		assert!(complete.tx.verify(&mut ctx, overage).is_ok());

		assert!(complete.header.hash == [0u8; 32]);
		// we'll error out after 1024 iterations
		assert!(complete.mine_block(&mut ctx, 0, 1024).is_err());
		assert!(complete.validate_hash(&mut ctx).is_err());

		Ok(())
	}
}
