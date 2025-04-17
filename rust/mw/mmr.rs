#![allow(dead_code)]

use core::ptr::copy_nonoverlapping;
use crypto::Ctx;
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

	/*

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
			println!("updating peaks");
			let peaks_key = format!("{}:meta:peaks", self.prefix)?;
			let mut peaks: Vec<[u8; 32]> = txn
				.get(&peaks_key)?
				.map(|bytes| Self::deserialize_peaks(bytes))
				.unwrap_or(Ok(Vec::new()))?;
			println!("x");
			peaks.push(hash)?;
			println!("Pushed peak: {}", hash);

			// Compute expected number of peaks for size + 1
			let num_peaks = (size + 1).count_ones() as usize;
			println!("Number of peaks for size {}: {}", size + 1, num_peaks);

			// Merge peaks to form valid mountains
			while peaks.len() > num_peaks {
				let right_idx = peaks.len() - 1;
				let left_idx = peaks.len() - 2;
				println!(
					"Merging peaks: right_idx={}, left_idx={}",
					right_idx, left_idx
				);

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
				println!("Merged peak: {}", merged);
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

	pub fn prune(&mut self, data: &[u8]) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	pub fn root_hash(&self, ctx: &mut Ctx) -> Result<[u8; 32], Error> {
		let size = self.size()?;
		let txn = self.db.read()?;

		let peaks_key = format!("{}:meta:peaks", self.prefix)?;

		let peaks: Vec<[u8; 32]> = match txn.get(&peaks_key)? {
			Some(bytes) => Self::deserialize_peaks(bytes)?,
			None => Vec::new(),
		};

		println!("peaks.len={}", peaks.len());

		// Validate peaks
		for i in 0..peaks.len() {
			println!("calling in root hash");
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

	pub fn peak_height(peak_index: usize, size: u64) -> Result<u32, Error> {
		println!(
			"calling peak_height with peak_index={}, size={}",
			peak_index, size
		);
		if size == 0 {
			return Err(Error::new(IllegalArgument)); // No peaks for empty MMR
		}
		/*
		if peak_index >= size.count_ones() as usize {
			println!(
				"Invalid peak_index={} >= num_peaks={}",
				peak_index,
				size.count_ones()
			);
			return Err(Error::new(IllegalArgument));
		}
				*/

		let mut remaining_peaks = peak_index as u32;
		let mut bit_position: u32 = 63; // Start at most significant bit (for u64)

		// Iterate through bits of size from MSB to LSB
		while bit_position != 0xFFFFFFFFu32 {
			if (size & (1u64 << bit_position)) != 0 {
				// Found a 1 bit (a peak)
				if remaining_peaks == 0 {
					println!("peak_height({}) -> {}", peak_index, bit_position);
					return Ok(bit_position); // Height is the bit position
				}
				remaining_peaks -= 1;
			}
			bit_position = bit_position.saturating_sub(1);
		}

		// Should never reach here due to early check
		println!(
			"!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!exceeded!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
		);
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

		*/
	fn deserialize_peaks(bytes: &[u8]) -> Result<Vec<[u8; 32]>, Error> {
		if bytes.len() % 32 != 0 {
			return Err(Error::new(InvalidData));
		}
		let mut peaks = Vec::new();
		for chunk in bytes.chunks(32) {
			if chunk.len() == 32 {
				let mut nval = [0u8; 32];
				unsafe {
					copy_nonoverlapping(chunk.as_ptr(), nval.as_mut_ptr(), 32);
				}
				peaks.push(nval)?;
			}
		}
		Ok(peaks)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::convert::Into;
	use crypto::SecretKey;

	/*
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
		println!("rh={}", mmr.root_hash(&mut ctx)?);
		let blind = SecretKey::gen(&mut ctx);
		let commitment = ctx.commit(0, &blind)?;
		mmr.append(&mut ctx, commitment.as_ref())?;
		println!("rh1={}", mmr.root_hash(&mut ctx)?);
		assert_eq!(mmr.size()?, 1);

		let blind = SecretKey::gen(&mut ctx);
		let commitment = ctx.commit(1, &blind)?;
		println!("==================start insert 2==========================");
		mmr.append(&mut ctx, commitment.as_ref())?;
		println!("rh2={}", mmr.root_hash(&mut ctx)?);
		assert_eq!(mmr.size()?, 2);

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}
		*/
}
