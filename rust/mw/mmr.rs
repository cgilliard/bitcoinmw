#![allow(dead_code)]

use core::ptr::copy_nonoverlapping;
use crypto::Ctx;
use lmdb::txn::LmdbTxn;
use prelude::*;
use std::misc::{from_le_bytes_u64, to_le_bytes_u64};

pub struct MMR {
	db: Lmdb,
	prefix: String,
	capacity: u64,
}

impl MMR {
	pub fn new(db: Lmdb, prefix: &str, capacity: u64) -> Result<Self, Error> {
		if prefix.is_empty() {
			return Err(Error::new(IllegalArgument));
		}
		Ok(Self {
			db,
			prefix: String::new(prefix)?,
			capacity,
		})
	}

	pub fn append(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<(), Error> {
		if data.is_empty() {
			return Err(Error::new(IllegalArgument));
		}
		let mut txn = self.db.write()?;

		let size_key = format!("{}:meta:size", self.prefix)?;

		let size = txn.get(&size_key)?.unwrap_or(&[0, 0, 0, 0, 0, 0, 0, 0]);
		let size = from_le_bytes_u64(size);

		if size >= self.capacity {
			return Err(Error::new(CapacityExceeded));
		}

		let leaf_key = format!("{}:leaf:{}", self.prefix, size)?;

		txn.put(&leaf_key, data)?;

		ctx.sha3().reset();
		ctx.sha3().update(data);
		let mut hash = [0u8; 32];
		ctx.sha3().finalize(&mut hash)?;
		let data_key = format!("{}:data:{}", self.prefix, hash)?;
		let mut size_bytes = [0u8; 8];
		to_le_bytes_u64(size, &mut size_bytes);
		txn.put(&data_key, &size_bytes)?;

		let mut pos = size;
		let mut height = 0;

		while Self::has_sibling(pos, height) {
			let sibling_pos = Self::get_sibling_pos(pos, height);
			let sibling_key = if height == 0 {
				format!("{}:leaf:{}", self.prefix, sibling_pos)
			} else {
				format!("{}:node:{}:{}", self.prefix, height, sibling_pos)
			}?;

			let mut dual_hash = [0u8; 64];
			match txn.get(&sibling_key)? {
				Some(bytes) => unsafe {
					if bytes.len() != 32 {
						return Err(Error::new(InvalidData));
					}
					if pos & 1 == 0 {
						copy_nonoverlapping(bytes.as_ptr(), dual_hash.as_mut_ptr(), 32);
						copy_nonoverlapping(hash.as_ptr(), dual_hash.as_mut_ptr().offset(32), 32);
					} else {
						copy_nonoverlapping(bytes.as_ptr(), dual_hash.as_mut_ptr().offset(32), 32);
						copy_nonoverlapping(hash.as_ptr(), dual_hash.as_mut_ptr(), 32);
					}
				},
				None => return Err(Error::new(MissingNode)),
			}

			ctx.sha3().reset();
			ctx.sha3().update(&dual_hash);
			ctx.sha3().finalize(&mut hash)?;

			pos = Self::parent_pos(pos); // Move to parent position
			height += 1; // Increment height

			// Store parent node
			let node_key = format!("{}:node:{}:{}", self.prefix, height, pos)?;
			txn.put(&node_key, &hash)?;
		}

		if !Self::has_sibling(pos, height) {
			let peaks_key = format!("{}:meta:peaks", self.prefix)?;
			let mut peaks: Vec<[u8; 32]> = txn
				.get(&peaks_key)?
				.map(|bytes| Self::deserialize_peaks(bytes))
				.unwrap_or(Ok(Vec::new()))?;
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
				let mut merged = [0u8; 32];
				let mut dual_hash = [0u8; 64];
				unsafe {
					copy_nonoverlapping(left.as_ptr(), dual_hash.as_mut_ptr(), 32);
					copy_nonoverlapping(right.as_ptr(), dual_hash.as_mut_ptr().offset(32), 32);
				}
				ctx.sha3().reset();
				ctx.sha3().update(&dual_hash);
				ctx.sha3().finalize(&mut merged)?;
				peaks.push(merged)?;
			}

			let peaks_slice = peaks.slice(0, peaks.len());
			let serialized_peaks = Self::serialize_peaks(peaks_slice)?;
			txn.put(
				&peaks_key,
				serialized_peaks.slice(0, serialized_peaks.len()),
			)?;
		}

		let mut size_bytes = [0u8; 8];
		to_le_bytes_u64(size + 1, &mut size_bytes);
		txn.put(&size_key, &size_bytes)?;

		txn.commit()
	}

	pub fn prune(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<(), Error> {
		// Compute the hash of the input commitment
		ctx.sha3().reset();
		ctx.sha3().update(data);
		let mut hash = [0u8; 32];
		ctx.sha3().finalize(&mut hash)?;

		// Query the reverse index to get the MMR index
		let data_key = format!("{}:data:{}", self.prefix, hash)?;
		let mut txn = self.db.write()?;
		let index_bytes = txn.get(&data_key)?.ok_or(Error::new(MissingNode))?;
		let index = from_le_bytes_u64(index_bytes);

		// Validate index and check if already pruned
		let size = self.size()?;
		if index >= size {
			return Err(Error::new(IllegalArgument)); // Index out of bounds
		}
		let leaf_key = format!("{}:leaf:{}", self.prefix, index)?;
		if txn.get(&leaf_key)?.is_none() {
			return Err(Error::new(AlreadyPruned)); // Leaf already pruned
		}

		// Compute parent node’s index and height
		let (parent_index, height) = self.get_parent_info(index)?;
		let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };

		// Retrieve sibling’s data (if unpruned)
		let sibling_key = format!("{}:leaf:{}", self.prefix, sibling_index)?;
		let sibling_data = txn.get(&sibling_key)?;
		let mut sibling_hash = [0u8; 32];
		if let Some(data) = sibling_data {
			// Sibling is unpruned
			ctx.sha3().reset();
			ctx.sha3().update(data);
			ctx.sha3().finalize(&mut sibling_hash)?;
		} else {
			// Sibling is pruned; retrieve parent hash (future: from prefix:node:<height>:<index>)
			let sibling_parent_key =
				format!("{}:node:{}:{}", self.prefix, height + 1, parent_index)?;
			let parent_hash = txn
				.get(&sibling_parent_key)?
				.ok_or(Error::new(MissingNode))?;
			unsafe {
				copy_nonoverlapping(
					parent_hash.as_ptr(),
					sibling_hash.as_mut_ptr(),
					sibling_hash.len(),
				);
			}
		}

		// Retrieve leaf’s data before pruning
		let leaf_data = txn.get(&leaf_key)?.ok_or(Error::new(MissingNode))?;
		let mut leaf_hash = [0u8; 32];
		ctx.sha3().reset();
		ctx.sha3().update(leaf_data);
		ctx.sha3().finalize(&mut leaf_hash)?;

		// Compute parent hash
		let (left_hash, right_hash) = if index % 2 == 0 {
			(leaf_hash, sibling_hash)
		} else {
			(sibling_hash, leaf_hash)
		};
		let mut parent_hash = [0u8; 32];
		let mut dual_hash = [0u8; 64];
		unsafe {
			copy_nonoverlapping(left_hash.as_ptr(), dual_hash.as_mut_ptr(), 32);
			copy_nonoverlapping(right_hash.as_ptr(), dual_hash.as_mut_ptr().offset(32), 32);
		}
		ctx.sha3().reset();
		ctx.sha3().update(&dual_hash);
		ctx.sha3().finalize(&mut parent_hash)?;

		// Store parent node hash
		let parent_key = format!("{}:node:{}:{}", self.prefix, height + 1, parent_index)?;
		txn.put(&parent_key, &parent_hash)?;

		// Remove leaf data
		txn.del(&leaf_key)?;

		// Retain reverse index entry (Grin’s approach)
		// To prune reverse index, uncomment:
		txn.del(&data_key)?;

		// Update peaks (simplified: recompute for now)
		self.update_peaks_after_prune(ctx, &mut txn, index, size)?;

		txn.commit()?;
		Ok(())
	}

	pub fn root_hash(&self, ctx: &mut Ctx) -> Result<[u8; 32], Error> {
		let size = self.size()?;
		let txn = self.db.read()?;

		let peaks_key = format!("{}:meta:peaks", self.prefix)?;

		let peaks: Vec<[u8; 32]> = match txn.get(&peaks_key)? {
			Some(bytes) => Self::deserialize_peaks(bytes)?,
			None => Vec::new(),
		};

		// Validate peaks
		for i in 0..peaks.len() {
			match Self::peak_height(i, size) {
				Ok(_) => {}
				Err(_) => break,
			}
		}

		if peaks.len() == 0 {
			return Ok([0; 32]); // Empty MMR
		}

		// Bag the peaks (hash all peaks together, left to right)
		let mut hash_input = Vec::with_capacity(peaks.len() * 32)?;
		for peak in &peaks {
			for x in peak {
				hash_input.push(*x)?;
			}
		}
		ctx.sha3().reset();
		ctx.sha3().update(hash_input.slice(0, hash_input.len()));
		let mut hash = [0u8; 32];
		ctx.sha3().finalize(&mut hash)?;
		Ok(hash)
	}

	pub fn size(&self) -> Result<u64, Error> {
		let txn = self.db.read()?;
		let size_key = format!("{}:meta:size", self.prefix)?;
		let size = txn.get(&size_key)?.unwrap_or(&[0, 0, 0, 0, 0, 0, 0, 0]);
		if size.len() != 8 {
			return Err(Error::new(InvalidData));
		}
		Ok(from_le_bytes_u64(size))
	}

	pub fn contains(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<bool, Error> {
		// Compute the hash of the input commitment
		ctx.sha3().reset();
		ctx.sha3().update(data);
		let mut hash = [0u8; 32];
		ctx.sha3().finalize(&mut hash)?;

		// Query the reverse index
		let data_key = format!("{}:data:{}", self.prefix, hash)?;
		let txn = self.db.read()?;
		Ok(txn.get(&data_key)?.is_some())
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

	fn update_peaks_after_prune(
		&mut self,
		ctx: &mut Ctx,
		txn: &mut LmdbTxn,
		pruned_index: u64,
		size: u64,
	) -> Result<(), Error> {
		let peaks_key = format!("{}:meta:peaks", self.prefix)?;
		let mut peaks: Vec<[u8; 32]> = txn
			.get(&peaks_key)?
			.map(|bytes| Self::deserialize_peaks(bytes))
			.unwrap_or(Ok(Vec::new()))?;

		// Validate pruned index
		if pruned_index >= size {
			return Err(Error::new(IllegalArgument));
		}

		// Find the peak containing the pruned index
		let mut peak_idx = 0;
		let mut current_pos = 0;
		let num_peaks = size.count_ones() as usize;
		while peak_idx < num_peaks && current_pos <= pruned_index {
			let height = Self::peak_height(peak_idx, size)?;
			let subtree_size = 1 << height;
			if current_pos <= pruned_index && pruned_index < current_pos + subtree_size {
				// Pruned index is in this peak’s subtree
				break;
			}
			current_pos += subtree_size;
			peak_idx += 1;
		}
		if peak_idx >= num_peaks {
			return Err(Error::new(InvalidData)); // Pruned index not in any peak
		}

		// Get the parent node’s hash for the pruned index
		let (parent_index, height) = self.get_parent_info(pruned_index)?;
		let parent_key = format!("{}:node:{}:{}", self.prefix, height + 1, parent_index)?;
		let parent_hash = txn.get(&parent_key)?.ok_or(Error::new(MissingNode))?;
		let mut parent_hash_array = [0u8; 32];
		unsafe {
			copy_nonoverlapping(
				parent_hash.as_ptr(),
				parent_hash_array.as_mut_ptr(),
				parent_hash.len(),
			);
		}

		// Update the affected peak
		if height + 1 == Self::peak_height(peak_idx, size)? {
			// The parent node replaces the peak directly
			peaks[peak_idx] = parent_hash_array;
		} else {
			// The pruned index is a leaf in a larger subtree; recompute the peak (simplified)
			let mut new_peaks = Vec::new();
			let mut i = current_pos;
			let subtree_end = current_pos + (1 << Self::peak_height(peak_idx, size)?);
			while i < subtree_end && i < size {
				let leaf_key = format!("{}:leaf:{}", self.prefix, i)?;
				if let Some(leaf_data) = txn.get(&leaf_key)? {
					// Unpruned leaf
					ctx.sha3().reset();
					ctx.sha3().update(leaf_data);
					let mut leaf_hash = [0u8; 32];
					ctx.sha3().finalize(&mut leaf_hash)?;
					new_peaks.push(leaf_hash)?;
					i += 1;
				} else {
					// Pruned leaf; check parent node
					let (p_index, h) = self.get_parent_info(i)?;
					let p_key = format!("{}:node:{}:{}", self.prefix, h + 1, p_index)?;
					if let Some(p_hash) = txn.get(&p_key)? {
						let mut p_hash_array = [0u8; 32];
						unsafe {
							copy_nonoverlapping(
								p_hash.as_ptr(),
								p_hash_array.as_mut_ptr(),
								p_hash.len(),
							);
						}
						new_peaks.push(p_hash_array)?;
						i += 1 << (h + 1); // Skip subtree
					} else {
						return Err(Error::new(MissingNode));
					}
				}
			}
			// Merge new_peaks to form the peak
			while new_peaks.len() > 1 {
				let mut merged_peaks = Vec::new();
				let mut i = 0;
				while i < new_peaks.len() {
					if i + 1 < new_peaks.len() {
						let mut dual_hash = [0u8; 64];
						unsafe {
							copy_nonoverlapping(new_peaks[i].as_ptr(), dual_hash.as_mut_ptr(), 32);
							copy_nonoverlapping(
								new_peaks[i + 1].as_ptr(),
								dual_hash.as_mut_ptr().offset(32),
								32,
							);
						}
						ctx.sha3().reset();
						ctx.sha3().update(&dual_hash);
						let mut merged_hash = [0u8; 32];
						ctx.sha3().finalize(&mut merged_hash)?;
						merged_peaks.push(merged_hash)?;
						i += 2;
					} else {
						merged_peaks.push(new_peaks[i])?;
						i += 1;
					}
				}
				new_peaks = merged_peaks;
			}
			if new_peaks.len() == 0 {
				return Err(Error::new(InvalidData));
			}
			peaks[peak_idx] = new_peaks[0];
		}

		// Ensure peaks match size.count_ones()
		let num_peaks = size.count_ones() as usize;
		if peaks.len() > num_peaks {
			let mut merged_peaks = Vec::new();
			let mut i = 0;
			while i < peaks.len() {
				if i + 1 < peaks.len()
					&& Self::peak_height(i, size)? == Self::peak_height(i + 1, size)?
				{
					let mut dual_hash = [0u8; 64];
					unsafe {
						copy_nonoverlapping(peaks[i].as_ptr(), dual_hash.as_mut_ptr(), 32);
						copy_nonoverlapping(
							peaks[i + 1].as_ptr(),
							dual_hash.as_mut_ptr().offset(32),
							32,
						);
					}
					ctx.sha3().reset();
					ctx.sha3().update(&dual_hash);
					let mut merged_hash = [0u8; 32];
					ctx.sha3().finalize(&mut merged_hash)?;
					merged_peaks.push(merged_hash)?;
					i += 2;
				} else {
					merged_peaks.push(peaks[i])?;
					i += 1;
				}
			}
			peaks = merged_peaks;
		}

		// Store updated peaks
		let serialized_peaks = Self::serialize_peaks(peaks.slice(0, peaks.len()))?;
		txn.put(
			&peaks_key,
			serialized_peaks.slice(0, serialized_peaks.len()),
		)?;

		Ok(())
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
		while bit_position != 0xFFFFFFFFu32 {
			if (size & (1u64 << bit_position)) != 0 {
				// Found a 1 bit (a peak)
				if remaining_peaks == 0 {
					return Ok(bit_position); // Height is the bit position
				}
				remaining_peaks -= 1;
			}
			bit_position = bit_position.saturating_sub(1);
		}

		// Should never reach here due to early check
		Err(Error::new(IllegalArgument))
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
}

#[cfg(test)]
mod test {
	use super::*;
	use core::convert::Into;
	use crypto::Commitment;

	#[test]
	fn test_mmr1() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;

		let db_size = 1024 * 1024 * 10;
		let db_name = "mydb";
		let db_dir = "bin/.mmr1";
		make_lmdb_test_dir(db_dir)?;
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		let mut mmr = MMR::new(db, "output_mmr", u32::MAX.into())?;

		assert_eq!(mmr.size()?, 0);
		let keychain = KeyChain::from_seed([3u8; 32])?;
		let blind = keychain.derive_key(&mut ctx, &[0, 0]);
		let commitment = ctx.commit(0, &blind)?;
		mmr.append(&mut ctx, commitment.as_ref())?;
		assert_eq!(mmr.size()?, 1);
		assert!(mmr.contains(&mut ctx, commitment.as_ref())?);

		let blind = keychain.derive_key(&mut ctx, &[0, 1]);
		let commitment2 = ctx.commit(1, &blind)?;
		mmr.append(&mut ctx, commitment2.as_ref())?;
		assert_eq!(mmr.size()?, 2);

		assert!(mmr.contains(&mut ctx, commitment2.as_ref())?);
		assert!(mmr.contains(&mut ctx, commitment.as_ref())?);

		let blind = keychain.derive_key(&mut ctx, &[0, 2]);
		let commitment3 = ctx.commit(1, &blind)?;
		assert!(!mmr.contains(&mut ctx, commitment3.as_ref())?);
		mmr.append(&mut ctx, commitment3.as_ref())?;
		assert_eq!(mmr.size()?, 3);

		assert!(mmr.contains(&mut ctx, commitment3.as_ref())?);
		assert!(mmr.contains(&mut ctx, commitment2.as_ref())?);
		assert!(mmr.contains(&mut ctx, commitment.as_ref())?);

		for i in 0..10 {
			let blind = keychain.derive_key(&mut ctx, &[0, 3 + i]);
			let commitment = ctx.commit(100, &blind)?;
			mmr.append(&mut ctx, commitment.as_ref())?;
		}

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}

	#[test]
	fn test_prune_and_peaks() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let db_size = 1024 * 1024 * 10;
		let db_name = "mydb";
		let db_dir = "bin/.mmr2";
		make_lmdb_test_dir(db_dir)?;
		let db = Lmdb::new(db_dir, db_name, db_size)?;
		let mut mmr = MMR::new(db, "output_mmr", u32::MAX.into())?;

		let mut commitments: Vec<Commitment> = Vec::new();
		let keychain = KeyChain::from_seed([3u8; 32])?;

		for i in 0..13 {
			let blind = keychain.derive_key(&mut ctx, &[0, 3 + i]);
			let commitment = ctx.commit(100, &blind)?;
			commitments.push(commitment)?;
		}

		// Insert 13 commitments
		for commitment in &commitments {
			mmr.append(&mut ctx, commitment.as_ref())?;
		}
		let original_root = mmr.root_hash(&mut ctx)?;

		// Prune the first commitment
		mmr.prune(&mut ctx, commitments[0].as_ref())?;

		// Verify contains
		assert!(
			!mmr.contains(&mut ctx, commitments[0].as_ref())?,
			"Pruned output should not be contained"
		);
		assert!(
			mmr.contains(&mut ctx, commitments[1].as_ref())?,
			"Unspent output should be contained"
		);

		// Verify leaf and reverse index
		{
			let txn = mmr.db.read()?;
			let leaf_key = format!("output_mmr:leaf:0")?;
			let mut hash = [0u8; 32];
			commitments[0].sha3(ctx.sha3());
			ctx.sha3().finalize(&mut hash)?;
			let data_key = format!("output_mmr:data:{}", hash)?;
			assert!(txn.get(&leaf_key)?.is_none(), "Leaf should be pruned");
			assert!(
				txn.get(&data_key)?.is_none(),
				"Reverse index should be pruned"
			);

			// Verify peaks and root hash
			let peaks_key = format!("output_mmr:meta:peaks")?;

			let peaks: Vec<[u8; 32]> = txn
				.get(&peaks_key)?
				.map(|bytes| MMR::deserialize_peaks(bytes))
				.unwrap_or(Ok(Vec::new()))?;

			assert_eq!(
				peaks.len(),
				13_i32.count_ones() as usize,
				"Number of peaks should match"
			);
		}
		let new_root = mmr.root_hash(&mut ctx)?;
		assert_ne!(
			new_root, original_root,
			"Root hash should change after pruning"
		);

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}
}
