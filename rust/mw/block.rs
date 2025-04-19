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
	// timestamp in seconds since epoch (ISO 8601) 00:00:00 Jan 1, 1970 (5 bytes allows for
	// 1099511627776 seconds (~34865 years).
	timestamp: [u8; 5],
	// the 16 lower bytes of the hash of prev block (sufficient to describe chain)
	prev_hash: [u8; 16],
	// The merkle root of all the kernels in this block. (using sha3-256).
	kernel_merkle_root: [u8; 32],
	// The hash of the output MMR at this block height.
	output_mmr_root_hash: [u8; 32],
	// The count of outputs in the output_mmr.
	output_mmr_size: u64,
	// Auxilary merkle root
	aux_merkle_root: [u8; 32],
	// nonce
	nonce: u32,
}

// Header = 130 bytes
// Minimal coinbase = 814 (1 kernel + 1 output + 1 range proof = 106 + 33 + 675) bytes

// implied values stored during block validation by miners but not broadcast
#[allow(dead_code)]
pub struct ShadowHeader {
	// hash of the current block Hash(header_version, timestamp, prev_hash, kernel_merkle_root,
	// aux_merkle_root, output_mmr_root_hash, nonce)
	hash: [u8; 32],
	// height of block
	height: u32,
	// maximum value allowed for the block hash
	target: [u8; 32],
	// block difficulty D = u64::MAX / u64_from_bytes(&target) (using most significant 8 bytes
	// of data from target)
	difficulty: u64,
	// cumulative difficulty for the entire blockchain
	// cumulative_difficulty(block) = difficulty(block) + cumulative_difficulty(parent)
	cumulative_difficulty: u64,
}

pub struct Block {
	// The block's header
	header: BlockHeader,
	// The block's transaction
	tx: Transaction,
}

impl BlockHeader {
	fn new(prev_hash: [u8; 16]) -> Self {
		let header_version = BLOCK_HEADER_VERSION;
		let timestamp = unsafe { getmicros() / 1_000_000u64 };
		let timestamp = Self::timestamp_to_bytes_le(timestamp);
		let aux_merkle_root = [0u8; 32];
		let output_mmr_root_hash = [0u8; 32];
		let output_mmr_size = 0;
		let kernel_merkle_root = [0u8; 32];

		let nonce = 0;
		Self {
			header_version,
			prev_hash,
			timestamp,
			kernel_merkle_root,
			aux_merkle_root,
			output_mmr_root_hash,
			output_mmr_size,
			nonce,
		}
	}

	fn timestamp_to_bytes_le(timestamp: u64) -> [u8; 5] {
		let mut bytes = [0u8; 8];
		to_le_bytes_u64(timestamp, &mut bytes);
		[bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]]
	}
}

impl Block {
	pub fn new(prev_hash: [u8; 16]) -> Self {
		let header = BlockHeader::new(prev_hash);
		let tx = Transaction::empty();
		Self { header, tx }
	}

	pub fn add_tx(&mut self, ctx: &Ctx, tx: Transaction) -> Result<(), Error> {
		self.tx.merge(ctx, tx)
	}

	#[inline]
	pub fn mine_block(
		&mut self,
		ctx: &mut Ctx,
		iterations: u32,
		target: [u8; 32],
	) -> Result<[u8; 32], Error> {
		for i in 0..iterations {
			self.header.nonce = self.header.nonce.wrapping_add(i);
			let hash = self.calculate_hash(ctx)?;
			if u256_less_than_or_equal(&target, &hash) {
				// difficulty met - block found
				return Ok(hash);
			}
		}
		Err(Error::new(NotFound))
	}

	pub fn validate_hash(
		&self,
		ctx: &mut Ctx,
		target: [u8; 32],
		hash: [u8; 32],
	) -> Result<(), Error> {
		if !u256_less_than_or_equal(&target, &hash) {
			Err(Error::new(InsufficientBlockHash))
		} else {
			let hash_calc = self.calculate_hash(ctx)?;
			if hash != hash_calc {
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
		let nonce = SecretKey::gen(&ctx);
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
		let mut header = self.header.clone();
		header.kernel_merkle_root = tx.kernel_merkle_root(ctx)?;
		// TODO: calculate header.output_mmr_root_hash and header.output_mmr_size

		Ok(Block { header, tx })
	}

	pub fn fees(&self) -> u64 {
		self.tx.fees()
	}

	#[inline]
	fn calculate_hash(&self, ctx: &mut Ctx) -> Result<[u8; 32], Error> {
		let sha3 = ctx.sha3();
		sha3.reset();
		let mut buf32 = [0u8; 4];
		let mut buf64 = [0u8; 8];

		// header_version
		sha3.update(&[self.header.header_version]);

		// timestamp
		sha3.update(&self.header.timestamp);

		// prev_hash
		sha3.update(&self.header.prev_hash);

		// kernel_merkle_root
		sha3.update(&self.header.kernel_merkle_root);

		// output_mmr_root_hash
		sha3.update(&self.header.output_mmr_root_hash);

		to_le_bytes_u64(self.header.output_mmr_size, &mut buf64);
		sha3.update(&buf64);

		// aux_merkle_root
		sha3.update(&self.header.aux_merkle_root);

		// nonce
		to_le_bytes_u32(self.header.nonce, &mut buf32);
		sha3.update(&buf32);

		let mut ret = [0u8; 32];
		sha3.finalize(&mut ret)?;
		Ok(ret)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::ptr::copy_nonoverlapping;
	use mw::constants::{DIFFICULTY_4BIT_LEADING, DIFFICULTY_HARD};

	#[test]
	fn test_block1() -> Result<(), Error> {
		// create a block (specify prev_hash)
		let prev_hash = [0u8; 16];
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

		let kernels = complete.tx.kernels();

		let mut leaves = Vec::with_capacity(kernels.len())?;

		// Copy kernel hashes from red-black tree to Vec
		for k in complete.tx.kernels().iter() {
			ctx.sha3().reset();
			let mut hash = [0u8; 32];
			k.sha3(ctx.sha3());
			ctx.sha3().finalize(&mut hash)?;
			leaves.push(hash)?;
		}

		let mut input1 = [0u8; 64];
		let mut input2 = [0u8; 64];
		let mut input_mid = [0u8; 64];
		unsafe {
			copy_nonoverlapping(leaves[0].as_ptr(), input1.as_mut_ptr(), 32);
			copy_nonoverlapping(leaves[1].as_ptr(), input1.as_mut_ptr().add(32), 32);
			copy_nonoverlapping(leaves[2].as_ptr(), input2.as_mut_ptr(), 32);
			copy_nonoverlapping(leaves[2].as_ptr(), input2.as_mut_ptr().add(32), 32);
			ctx.sha3().reset();
			ctx.sha3().update(&input1);

			ctx.sha3().finalize(&mut input_mid[0..32])?;
			ctx.sha3().reset();
			ctx.sha3().update(&input2);
			ctx.sha3().finalize(&mut input_mid[32..])?;
		}

		let mut hash = [0u8; 32];
		ctx.sha3().reset();
		ctx.sha3().update(&input_mid);
		ctx.sha3().finalize(&mut hash)?;
		assert_eq!(complete.tx.kernel_merkle_root(&mut ctx)?, hash);

		Ok(())
	}

	#[test]
	fn mine_empty_block() -> Result<(), Error> {
		// create a block (specify prev_hash)
		let prev_hash = [0u8; 16];
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

	#[test]
	fn test_mining() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let prev_hash = [0u8; 16];
		let block = Block::new(prev_hash);

		// mine an empty block (only coinbase)
		let miner_keychain = KeyChain::from_seed([5u8; 32])?;
		// set overage for this block height. We use 1000.
		let overage = 1000;
		// generate a blind from our keychain
		let coinbase_blind = miner_keychain.derive_key(&ctx, &[0, 0]);

		let mut complete = block.with_coinbase(&mut ctx, &coinbase_blind, overage)?;
		let hash = complete.mine_block(&mut ctx, 1024 * 1024, DIFFICULTY_4BIT_LEADING)?;

		assert!(hash != [0u8; 32]);
		assert!(hash[0] & 0xF0 == 0);

		assert!(complete
			.validate_hash(&mut ctx, DIFFICULTY_4BIT_LEADING, hash)
			.is_ok());

		// update the aux_merkle_root
		complete.header.aux_merkle_root = [1u8; 32];
		let hash2 = complete.mine_block(&mut ctx, 1024 * 1024, DIFFICULTY_4BIT_LEADING)?;

		assert!(hash2 != [0u8; 32]);
		assert!(hash2[0] & 0xF0 == 0);
		assert!(hash != hash2);

		assert!(complete
			.validate_hash(&mut ctx, DIFFICULTY_4BIT_LEADING, hash2)
			.is_ok());

		// try something too difficult
		let mut complete = block.with_coinbase(&mut ctx, &coinbase_blind, overage)?;

		assert!(complete
			.mine_block(&mut ctx, 1024, DIFFICULTY_HARD,)
			.is_err());

		Ok(())
	}
}
