use core::ptr::copy_nonoverlapping;
use crypto::Ctx;
use lmdb::txn::LmdbTxn;
use prelude::*;
use std::misc::{from_le_bytes_u64, to_le_bytes_u64};
use store::constants::ZERO_BYTES;
use store::pmmr_index::PmmrIndex;

pub struct Pmmr {
	pmmr_index: PmmrIndex,
	db: Lmdb,
	prefix: String,
	write: bool,
}

impl Pmmr {
	// create a new instance of this pmmr with the specified lmdb instance and prefix
	pub fn new(db: Lmdb, prefix_str: &str) -> Result<Self, Error> {
		let prefix = String::new(prefix_str)?;
		let pmmr_index = PmmrIndex::new(db.try_clone()?, prefix_str)?;
		let write = true;
		Ok(Self {
			prefix,
			db,
			pmmr_index,
			write,
		})
	}

	// rewind the pmmr - destructive rewind of the data in the pmmr to last_pos (reorgs)
	pub fn rewind(&mut self, last_pos: u64) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	pub fn size(&self, txn: Option<LmdbTxn>) -> Result<u64, Error> {
		let txn = self.get_read_txn(txn)?;
		let size_key = format!("{}:meta:size", self.prefix)?;
		let size = from_le_bytes_u64(self.get_key_with_default(&txn, &size_key, &ZERO_BYTES)?);
		Ok(size)
	}

	pub fn append(
		&mut self,
		ctx: &mut Ctx,
		data: &[u8],
		txn: Option<LmdbTxn>,
	) -> Result<(), Error> {
		if data.is_empty() {
			return Err(Error::new(IllegalArgument));
		}
		let (mut txn, commit) = self.get_write_txn(txn)?;
		let size = self.size(Some(txn.clone()))?;
		let leaf_key = format!("{}:leaf:{}", self.prefix, size)?;
		txn.put(&leaf_key, data)?;
		let mut hash = self.hash_data(ctx, data)?;
		let data_key = format!("{}:data:{}", self.prefix, hash)?;
		let mut size_bytes = [0u8; 8];
		to_le_bytes_u64(size, &mut size_bytes);
		txn.put(&data_key, &size_bytes)?;

		let mut pos = size;
		let mut height = 0;

		while Self::has_sibling(pos, height) {
			let sibling_pos = Self::get_sibling_pos(pos, height);
			// leaf nodes have height == 0, others are branch nodes
			let sibling_key = if height == 0 {
				format!("{}:leaf:{}", self.prefix, sibling_pos)
			} else {
				format!("{}:node:{}:{}", self.prefix, height, sibling_pos)
			}?;

			let sibling = match txn.get(&sibling_key)? {
				Some(s) => s,
				None => return Err(Error::new(InvalidData)),
			};
			hash = if pos & 1 == 0 {
				self.hash_children(ctx, sibling, &hash)
			} else {
				self.hash_children(ctx, &hash, sibling)
			}?;
			pos = Self::parent_pos(pos); // move up the tree
			height += 1;

			// store parent node
			let node_key = format!("{}:node:{}:{}", self.prefix, height, pos)?;
			txn.put(&node_key, &hash)?;
		}

		if !Self::has_sibling(pos, height) {
			let mut peaks = self.get_peaks(&txn)?;
			peaks.push(hash)?;

			// Compute expected number of peaks for size + 1
			let num_peaks = (size + 1).count_ones() as usize;

			// Merge peaks to form valid mountains
			while peaks.len() > num_peaks {
				let right_idx = peaks.len() - 1;
				let left_idx = peaks.len() - 2;

				// Merge the last two peaks
				let right = peaks[right_idx];
				let left = peaks[left_idx];
				peaks.truncate(peaks.len() - 2)?;

				let merged = self.hash_children(ctx, &left, &right)?;
				peaks.push(merged)?;
			}
			self.set_peaks(peaks, &mut txn)?;
		}

		let mut size_bytes = [0u8; 8];
		to_le_bytes_u64(size + 1, &mut size_bytes);
		let size_key = format!("{}:meta:size", self.prefix)?;
		txn.put(&size_key, &size_bytes)?;

		if commit {
			txn.commit()
		} else {
			Ok(())
		}
	}

	fn get_write_txn(&mut self, txn: Option<LmdbTxn>) -> Result<(LmdbTxn, bool), Error> {
		Ok(match txn {
			Some(txn) => (txn, false),
			None => (self.db.write()?, true),
		})
	}

	fn get_read_txn(&self, txn: Option<LmdbTxn>) -> Result<LmdbTxn, Error> {
		Ok(match txn {
			Some(txn) => txn,
			None => self.db.read()?,
		})
	}

	fn set_peaks(&self, peaks: Vec<[u8; 32]>, txn: &mut LmdbTxn) -> Result<(), Error> {
		let peaks_slice = peaks.slice(0, peaks.len());
		let serialized_peaks = Self::serialize_peaks(peaks_slice)?;
		let peaks_key = format!("{}:meta:peaks", self.prefix)?;
		txn.put(
			&peaks_key,
			serialized_peaks.slice(0, serialized_peaks.len()),
		)?;

		Ok(())
	}

	fn get_peaks(&self, txn: &LmdbTxn) -> Result<Vec<[u8; 32]>, Error> {
		let peaks_key = format!("{}:meta:peaks", self.prefix)?;
		let peaks: Vec<[u8; 32]> = txn
			.get(&peaks_key)?
			.map(|bytes| Self::deserialize_peaks(bytes))
			.unwrap_or(Ok(Vec::new()))?;
		Ok(peaks)
	}

	fn serialize_peaks(peaks: &[[u8; 32]]) -> Result<Vec<u8>, Error> {
		let mut result = Vec::with_capacity(peaks.len() * 32)?;
		for peak in peaks {
			for x in peak {
				result.push(*x)?;
			}
		}
		Ok(result)
	}

	fn deserialize_peaks(bytes: &[u8]) -> Result<Vec<[u8; 32]>, Error> {
		if bytes.len() % 32 != 0 {
			return Err(Error::new(InvalidData));
		}
		let mut peaks = Vec::new();
		let mut vec1 = Vec::new();
		for byte in bytes {
			vec1.push(*byte)?;
			if vec1.len() == 32 {
				let mut nval = [0u8; 32];
				unsafe {
					copy_nonoverlapping(vec1.as_ptr(), nval.as_mut_ptr(), 32);
				}
				peaks.push(nval)?;
				vec1 = Vec::new();
			}
		}
		Ok(peaks)
	}

	fn hash_children(&self, ctx: &mut Ctx, left: &[u8], right: &[u8]) -> Result<[u8; 32], Error> {
		let mut dual_hash = [0u8; 64];
		unsafe {
			copy_nonoverlapping(left.as_ptr(), dual_hash.as_mut_ptr(), 32);
			copy_nonoverlapping(right.as_ptr(), dual_hash.as_mut_ptr().offset(32), 32);
		}
		self.hash_data(ctx, &dual_hash)
	}

	// prune stale spent outputs from the pmmr
	pub fn prune(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	// retreive the last_pos of the pmmr
	pub fn last_pos(&self) -> Result<u64, Error> {
		Err(Error::new(Todo))
	}

	// return the position of data in the pmmr or error if not found
	pub fn contains(&self, ctx: &mut Ctx, data: &[u8]) -> Result<u64, Error> {
		Err(Error::new(Todo))
	}

	// return the root_hash of the pmmr
	pub fn root_hash(&self, ctx: &mut Ctx) -> Result<[u8; 32], Error> {
		Err(Error::new(Todo))
	}

	// return the last_pos of the oldest sync_peak data available in this pmmr
	pub fn oldest_sync_peak(&self) -> Result<u64, Error> {
		Err(Error::new(Todo))
	}

	// get the sync peaks (top 4 submountains of the largest mountain, top 2 submountains of
	// the second and third mountains it they exist, and the peak of the other mountains if
	// they exist).
	pub fn sync_peaks(&self, last_pos: u64) -> Result<Vec<[u8; 32]>, Error> {
		Err(Error::new(Todo))
	}

	// get the sync chunk associated with the sync_peaks for this pmmr instance.
	pub fn sync_chunk(&self, index: usize, last_pos: u64) -> Result<Vec<u8>, Error> {
		Err(Error::new(Todo))
	}

	// helper functions
	fn get_key_with_default<'a>(
		&self,
		txn: &'a LmdbTxn,
		key: &String,
		default_value: &'a [u8],
	) -> Result<&'a [u8], Error> {
		Ok(txn.get(key)?.unwrap_or(default_value))
	}

	fn hash_data(&self, ctx: &mut Ctx, data: &[u8]) -> Result<[u8; 32], Error> {
		let mut ret = [0u8; 32];
		let sha3 = ctx.sha3();
		sha3.reset();
		sha3.update(data);
		sha3.finalize(&mut ret)?;
		Ok(ret)
	}

	fn get_parent_info(&self, index: u64) -> Result<(u64, u32), Error> {
		let mut pos = index;
		let mut height = 0;
		while pos % 2 == 1 {
			pos /= 2;
			height += 1;
		}
		Ok((pos / 2, height))
	}

	fn has_sibling(pos: u64, height: u32) -> bool {
		let tree_size = 1u64 << height; // Size of a perfect tree at this height
		let next_pos = pos + 1; // Next position
		(next_pos & (tree_size - 1)) != 0
	}

	fn get_sibling_pos(pos: u64, height: u32) -> u64 {
		let tree_size = 1u64 << height;
		pos ^ tree_size // XOR to get the sibling position
	}

	fn parent_pos(pos: u64) -> u64 {
		(pos | 1) + 1 // Parent of pos in MMR
	}

	fn children_pos(parent: u64, height: u32) -> (u64, u64) {
		let tree_size = 1u64 << (height - 1); // Number of leaves in the parent's tree (2^(h-1))
		let left_child = parent - tree_size - 1; // Left child is offset by the right subtree's leaves
		let right_child = parent - 1; // Right child is the previous position
		(left_child, right_child)
	}

	fn peak_height(peak_index: usize, size: u64) -> Result<u32, Error> {
		if size == 0 {
			return Err(Error::new(IllegalArgument)); // No peaks for empty MMR
		}
		if peak_index >= size.count_ones() as usize {
			return Err(Error::new(IllegalArgument));
		}

		let mut remaining_peaks = peak_index as u32;
		let mut bit_position: u32 = 63; // Start at most significant bit (for u64)

		// Iterate through bits of size from MSB to LSB
		loop {
			if (size & (1u64 << bit_position)) != 0 {
				// Found a 1 bit (a peak)
				if remaining_peaks == 0 {
					return Ok(bit_position); // Height is the bit position
				}
				remaining_peaks -= 1;
			}
			bit_position = bit_position.saturating_sub(1);
		}
	}
}
