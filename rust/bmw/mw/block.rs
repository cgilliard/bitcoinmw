#![allow(dead_code)]

use bible::Bible;
use core::mem::size_of;
use core::slice::from_raw_parts;
use crypto::bip52::Bip52;
use crypto::ctx::Ctx;
use crypto::keys::{PublicKey, SecretKey};
use crypto::sha3::Sha3_256;
use mw::constants::*;
use mw::errors::*;
use mw::kernel::Kernel;
use mw::transaction::Transaction;
use prelude::*;
use ffi::getmicros;
use misc::{
	from_le_bytes_u32, from_le_bytes_u64, slice_copy, subslice_mut, to_le_bytes_u32,
	to_le_bytes_u64, u256_less_than_or_equal,
};

#[derive(Clone)]
#[repr(C)]
pub struct BlockHeader {
	// Header Version. Currently = 0
	header_version: u8,
	// timestamp in seconds since epoch (ISO 8601) 00:00:00 Jan 1, 1970. 5 bytes allows for
	// 1099511627776 seconds (~34,865 years).
	timestamp: [u8; 5],
	// nonce
	nonce: [u8; 4],
	// the previous block hash.
	prev_hash: [u8; 32],
	// hash of all relevant data commit to the state for sync peers.
	// Hash(kernel_merkle_root || output_pmmr_peak_data_hash || last_pos ||
	// bitmap_data_merkle_root || bitmap_rewind_data_merkle_root)
	// kernel_merkle_root - the merkle root of all the kernels in this block.
	// output_pmmr_peak_data_hash - peak_data_hash from the pmmr (encoding of pos, height, and
	// hash for each peak in the PMMR)
	// last_pos - last_pos in the pmmr
	// bitmap_data_merkle_root - the merkle root of the bitmap that indicates current state
	// of pruning of the outputs in the PMMR.
	// bitmap_rewind_data_merkle_root - The merkle root of (bit_pos: u64, is_output: bool)
	// pairs representing each input/output in the block. This data is used to rewind the
	// current bitmap back to a sync horizon.
	sync_state_hash: [u8; 32],
	// proposed Kp (PI controller) value. If 75% of blocks over the course of a 1440 block
	// epoch vote for the same Kp value, the value is adjusted for the next epoch.
	kp_proposed: [u8; 4],
	// proposed Ki (PI controller) value. If 75% of blocks over the course of a 1440 block
	// epoch vote for the same Ki value, the value is adjusted for the next epoch.
	ki_proposed: [u8; 4],
	// Auxilary data hash - used for non-consensus hased data a merkle root can be used to
	// commit to unlimited aux data in this block. Miners may also use this to increase the
	// nonce space, signal, or otherwise coordinate.
	aux_data_hash: [u8; 32],
}

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
	// Current Kp value
	kp: [u8; 4],
	// Current Ki value
	ki: [u8; 4],
}

pub struct Block {
	// The block's header
	header: BlockHeader,
	// The block's transaction
	tx: Transaction,
}

impl BlockHeader {
	fn new(
		prev_hash: [u8; 32],
		aux_data_hash: [u8; 32],
		ki_proposed: [u8; 4],
		kp_proposed: [u8; 4],
	) -> Self {
		let header_version = BLOCK_HEADER_VERSION;
		let timestamp = unsafe { getmicros() / 1_000_000u64 };
		let timestamp = Self::timestamp_to_bytes_le(timestamp);
		let sync_state_hash = [0u8; 32];
		let nonce = [0u8; 4];

		Self {
			header_version,
			prev_hash,
			timestamp,
			aux_data_hash,
			sync_state_hash,
			nonce,
			ki_proposed,
			kp_proposed,
		}
	}

	fn timestamp_to_bytes_le(seconds: u64) -> [u8; 5] {
		let mut bytes = [0u8; 8];
		let _ = to_le_bytes_u64(seconds, &mut bytes);
		[bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]]
	}
}

impl Block {
	pub fn new(
		prev_hash: [u8; 32],
		aux_data_hash: [u8; 32],
		ki_proposed: [u8; 4],
		kp_proposed: [u8; 4],
	) -> Self {
		let header = BlockHeader::new(prev_hash, aux_data_hash, ki_proposed, kp_proposed);
		let tx = Transaction::empty();
		Self { header, tx }
	}

	pub fn add_tx(&mut self, ctx: &Ctx, tx: Transaction) -> Result<()> {
		self.tx.merge(ctx, tx)
	}

	#[inline]
	pub fn mine_block(
		&mut self,
		bip52: &Bip52,
		iterations: u32,
		target: [u8; 32],
		bible: &Bible,
	) -> Result<[u8; 32]> {
		let mut nonce = from_le_bytes_u32(&self.header.nonce)?;

		for i in 0..iterations {
			nonce = nonce.wrapping_add(i);
			to_le_bytes_u32(nonce, &mut self.header.nonce)?;
			let hash = self.calculate_hash(bip52, bible);
			if u256_less_than_or_equal(&target, &hash) {
				// difficulty met - block found
				return Ok(hash);
			}
		}
		err!(NotFound)
	}

	pub fn validate_hash(
		&self,
		bip52: &Bip52,
		target: [u8; 32],
		hash: [u8; 32],
		bible: &Bible,
	) -> Result<()> {
		if !u256_less_than_or_equal(&target, &hash) {
			err!(ValidationFailed)
		} else {
			let hash_calc = self.calculate_hash(bip52, bible);
			if hash != hash_calc {
				err!(ValidationFailed)
			} else {
				Ok(())
			}
		}
	}

	pub fn finalize_header(&mut self) -> Result<()> {
		// update timestamp
		let timestamp = unsafe { getmicros() / 1_000_000u64 };
		let timestamp = BlockHeader::timestamp_to_bytes_le(timestamp);
		self.header.timestamp = timestamp;

		let sha3 = Sha3_256::new();
		// TODO: currently we just hash the kernel merkle root, but we need to add in the
		// other sync_state_hash.
		let kmr = self.tx.kernel_merkle_root()?;
		sha3.update(kmr.as_ref());
		self.header.sync_state_hash = sha3.finalize();

		Ok(())
	}

	pub fn with_coinbase(&self, ctx: &Ctx, output_blind: &SecretKey, overage: u64) -> Result<Self> {
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
		let msg = Kernel::message_for(&excess, 0, 0);
		let nonce = SecretKey::gen(&ctx);
		let pubnonce = PublicKey::from(&ctx, &nonce)?;
		let pubkey = PublicKey::from(&ctx, &excess_blind)?;
		let sig = ctx.sign(&msg, &excess_blind, &nonce, &pubnonce, &pubkey)?;
		let kernel = Kernel::new(excess, sig, 0, 0);
		coinbase.add_kernel(kernel)?;
		coinbase.add_output(output, range_proof)?;
		coinbase.validate(ctx, v)?;
		let mut tx = self.tx.try_clone()?;

		tx.merge(ctx, coinbase)?;
		tx.set_offset_zero();
		tx.validate(ctx, overage)?;

		// clone header
		let header = self.header.clone();
		Ok(Self { header, tx })
	}

	pub fn fees(&self) -> u64 {
		self.tx.fees()
	}

	#[inline]
	fn calculate_hash(&self, bip52: &Bip52, bible: &Bible) -> [u8; 32] {
		// largest verse is 533 bytes and header 114 so 1024 is enough
		let mut buffer = [0u8; 1024];
		let header_ptr = &self.header as *const BlockHeader as *const u8;
		let header_slice = unsafe { from_raw_parts(header_ptr, size_of::<BlockHeader>()) };
		let end_prev_hash = match self.header.prev_hash.subslice(24, 8) {
			Ok(eph) => eph,
			Err(e) => exit!("unexpected error returned from subslice: {}", e),
		};
		let val = match from_le_bytes_u64(end_prev_hash) {
			Ok(val) => val,
			Err(e) => exit!("unexpected error returned from from_le_bytes_u64: {}", e),
		};
		let verse = bible.find_mod(val as usize);
		let verse_ref = verse.as_ref();

		// we know this is not out of bounds
		let _ = slice_copy(header_slice, &mut buffer, header_slice.len());
		let header_sub_slice =
			match subslice_mut(&mut buffer, header_slice.len(), 1024 - header_slice.len()) {
				Ok(s) => s,
				Err(e) => exit!("unexpected error returned by sub_slice: {}", e),
			};
		let _ = slice_copy(verse_ref, header_sub_slice, verse.len());

		bip52.hash(&buffer)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::mem::size_of;
	use mw::keychain::KeyChain;
	use mw::slate::Slate;

	#[test]
	fn test_blocks1() -> Result<()> {
		assert_eq!(size_of::<BlockHeader>(), 114);
		Ok(())
	}

	#[test]
	fn test_block1() -> Result<()> {
		// create a block (specify prev_hash)
		let prev_hash = [0u8; 32];
		let mut block = Block::new(prev_hash, [0u8; 32], [0u8; 4], [0u8; 4]);
		// create a crypto ctx
		let ctx = Ctx::new()?;

		// create a couple of txns for our block

		// generate an offset for our slate
		let offset = SecretKey::gen(&ctx);
		// set fee to 10
		let fee = 10;
		// create slate
		let mut slate = Slate::new(fee, offset);

		// user 1's keychain from seed 0u8
		let kc1 = KeyChain::from_seed([0u8; 48])?;
		// create an input from user1's keychain
		let input = kc1.derive_key(&ctx, &[0, 0]);
		// create a change output
		let change_output = kc1.derive_key(&ctx, &[0, 1]);

		// commit user's values
		let user1_id = slate.commit(&ctx, &[(&input, 100)], &[(&change_output, 10)])?;

		// send to user2
		// user2's keychain from seed 1u8
		let kc2 = KeyChain::from_seed([1u8; 48])?;
		// create an output key
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		// commit to transaction receiving 80 coins.
		let user2_id = slate.commit(&ctx, &[], &[(&output, 80)])?;
		// user 2 signs, then sends back to user1 to sign and finalize the txn
		slate.sign(&ctx, user2_id, &[], &[&output])?;
		slate.sign(&ctx, user1_id, &[&input], &[&change_output])?;
		let tx = slate.finalize(&ctx)?;

		// verify the transaction is valid with overage = 0
		assert!(tx.validate(&ctx, 0).is_ok());

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

		let kc1 = KeyChain::from_seed([2u8; 48])?;
		let input = kc1.derive_key(&ctx, &[0, 0]);
		let change_output = kc1.derive_key(&ctx, &[0, 1]);

		let user1_id = slate.commit(&ctx, &[(&input, 200)], &[(&change_output, 10)])?;

		let kc2 = KeyChain::from_seed([3u8; 48])?;
		let output = kc2.derive_key(&ctx, &[0, 0]); // choose an output

		let user2_id = slate.commit(&ctx, &[], &[(&output, 170)])?;
		slate.sign(&ctx, user2_id, &[], &[&output])?;
		slate.sign(&ctx, user1_id, &[&input], &[&change_output])?;
		let tx2 = slate.finalize(&ctx)?;
		assert!(tx2.validate(&ctx, 0).is_ok());

		// add the second txn to our block
		block.add_tx(&ctx, tx2)?;

		// confirm outputs/inputs/kenrels
		assert_eq!(block.tx.outputs().len(), 4);
		assert_eq!(block.tx.inputs().len(), 2);
		assert_eq!(block.tx.kernels().len(), 2);

		// verify the block's transaction is valud with overage = 0
		assert!(block.tx.validate(&ctx, 0).is_ok());

		// create a keychain for our miner
		let miner_keychain = KeyChain::from_seed([4u8; 48])?;
		// set overage for this block height. We use 1000.
		let overage = 1000;
		// generate a blind from our keychain
		let coinbase_blind = miner_keychain.derive_key(&ctx, &[0, 0]);
		// complete the block with the coinbas added
		let complete = block.with_coinbase(&ctx, &coinbase_blind, overage)?;

		// verify the block's transaction
		assert!(complete.tx.validate(&ctx, overage).is_ok());
		assert!(complete.tx.offset().is_none());

		assert_eq!(complete.tx.outputs().len(), 5);
		assert_eq!(complete.tx.inputs().len(), 2);
		assert_eq!(complete.tx.kernels().len(), 3);

		Ok(())
	}

	#[test]
	fn mine_empty_block() -> Result<()> {
		// create a block (specify prev_hash)
		let prev_hash = [0u8; 32];
		let block = Block::new(prev_hash, [0u8; 32], [0u8; 4], [0u8; 4]);
		// create a crypto ctx
		let ctx = Ctx::new()?;

		// confirm outputs/inputs/kernels
		assert_eq!(block.tx.outputs().len(), 0);
		assert_eq!(block.tx.inputs().len(), 0);
		assert_eq!(block.tx.kernels().len(), 0);

		// mine an empty block (only coinbase)
		let miner_keychain = KeyChain::from_seed([4u8; 48])?;
		// set overage for this block height. We use 1000.
		let overage = 1000;
		// generate a blind from our keychain
		let coinbase_blind = miner_keychain.derive_key(&ctx, &[0, 0]);
		// complete the block with the coinbase added
		let complete = block.with_coinbase(&ctx, &coinbase_blind, overage)?;

		// verify coinbase
		assert_eq!(complete.tx.outputs().len(), 1);
		assert_eq!(complete.tx.kernels().len(), 1);
		assert_eq!(complete.tx.inputs().len(), 0);
		assert!(complete.tx.validate(&ctx, overage).is_ok());

		Ok(())
	}

	#[test]
	fn test_mining() -> Result<()> {
		let bible = Bible::new();
		let ctx = Ctx::new()?;
		let prev_hash = [77u8; 32];
		let block = Block::new(prev_hash, [3u8; 32], [9u8; 4], [81u8; 4]);

		// mine an empty block (only coinbase)
		let miner_keychain = KeyChain::from_seed([5u8; 48])?;
		// set overage for this block height. We use 1000.
		let overage = 1000;
		// generate a blind from our keychain
		let coinbase_blind = miner_keychain.derive_key(&ctx, &[0, 0]);
		// generate a bip52 hasher with key (network param here we use [1u8; 32]) and
		// prev_hash
		let bip52 = Bip52::new([1u8; 32], prev_hash);

		let mut complete = block.with_coinbase(&ctx, &coinbase_blind, overage)?;
		complete.finalize_header()?;
		let hash = complete.mine_block(&bip52, 1024 * 1024, DIFFICULTY_4BIT_LEADING, &bible)?;

		assert!(hash != [0u8; 32]);
		assert!(hash[0] & 0xF0 == 0);

		assert!(complete
			.validate_hash(&bip52, DIFFICULTY_4BIT_LEADING, hash, &bible)
			.is_ok());

		// update the aux_hash_data
		complete.header.aux_data_hash = [1u8; 32];
		// we attempt up to 1 million iterations at this low difficulty (on avg only takes
		// a few tries)
		complete.finalize_header()?;
		let hash2 = complete.mine_block(&bip52, 1024 * 1024, DIFFICULTY_4BIT_LEADING, &bible)?;

		assert!(hash2 != [0u8; 32]);
		assert!(hash2[0] & 0xF0 == 0);
		assert!(hash != hash2);

		assert!(complete
			.validate_hash(&bip52, DIFFICULTY_4BIT_LEADING, hash2, &bible)
			.is_ok());

		// try something too difficult
		let mut complete = block.with_coinbase(&ctx, &coinbase_blind, overage)?;

		complete.finalize_header()?;
		assert!(complete
			.mine_block(&bip52, 1024, DIFFICULTY_HARD, &bible)
			.is_err());

		Ok(())
	}
}
