#![allow(unused_variables)]

use core::ptr::copy_nonoverlapping;
use crypto::Sha3_256;
use lmdb::{Lmdb, LmdbTxn};
use prelude::*;
use std::misc::{from_le_bytes_u64, slice_copy, subslice, to_le_bytes_u64};
use std::strext::StrExt;
use store::constants::{ALL_ONES, ZERO_BYTES};

pub struct Pmmr {
	db: Lmdb,
	prefix: String,
}

pub struct PeakInfo {
	hash: [u8; 32],
	height: u8,
	pos: u64,
}

impl PeakInfo {
	fn new(hash: [u8; 32], pos: u64, height: u8) -> Self {
		Self { hash, pos, height }
	}

	pub fn hash(&self) -> [u8; 32] {
		self.hash
	}

	pub fn height(&self) -> u8 {
		self.height
	}

	pub fn pos(&self) -> u64 {
		self.pos
	}

	fn serialize(&self) -> Result<Vec<u8>, Error> {
		let mut ret = Vec::with_capacity(41)?;
		ret.extend_from_slice(self.hash().as_ref())?;
		let mut height_bytes = [0u8; 8];
		to_le_bytes_u64(self.pos(), &mut height_bytes)?;
		ret.extend_from_slice(&height_bytes)?;
		ret.push(self.height())?;

		Ok(ret)
	}

	fn deserialize(bytes: &[u8]) -> Result<Self, Error> {
		if bytes.len() != 41 {
			return Err(Error::new(IllegalArgument));
		} else {
			let mut hash = [0u8; 32];
			slice_copy(bytes, &mut hash, 32)?;
			let pos_slice = subslice(bytes, 32, 8)?;
			let pos = from_le_bytes_u64(pos_slice)?;
			let height = bytes[40];
			Ok(Self { pos, hash, height })
		}
	}
}

impl Pmmr {
	// create a PMMR instance with the specified LMDB
	pub fn new(db: Lmdb, prefix_str: &str) -> Result<Self, Error> {
		if prefix_str.len() == 0 {
			return Err(Error::new(IllegalArgument));
		}
		match prefix_str.findn(":", 0) {
			Some(_) => return Err(Error::new(IllegalArgument)),
			None => {}
		}
		let prefix = String::new(prefix_str)?;
		Ok(Self { prefix, db })
	}

	/*
	6 (height=2)
	/ \
	2   5 (height=1)                  9
	/ \ / \                         /  \
	0  1 3  4                       7  8
	   */

	pub fn append(&mut self, data: &[u8], txn: Option<LmdbTxn>) -> Result<(), Error> {
		if data.is_empty() {
			return Err(Error::new(IllegalArgument));
		}
		let (mut txn, commit) = self.get_write_txn(txn)?;
		let last_pos = self.last_pos(Some(txn.clone()))?;
		let leaf_key = format!("{}:leaf:{}", self.prefix, last_pos)?;
		let mut hash = self.hash_data(data);
		let data_key = format!("{}:data:{}", self.prefix, hash)?;
		let mut last_pos_bytes = ZERO_BYTES;
		to_le_bytes_u64(last_pos, &mut last_pos_bytes)?;
		txn.put(&data_key, &last_pos_bytes)?;
		txn.put(&leaf_key, &hash)?;
		let mut pos = last_pos;
		let mut last_pos_update = last_pos + 1;
		let mut height = 0;

		if Self::is_peak(pos, last_pos_update - 1) {
			let mut peaks = self.get_peaks(Some(txn.clone()))?;
			let info = PeakInfo::new(hash, pos, height);
			peaks.push(info)?;
			self.set_peaks(peaks, &mut txn)?;
		} else {
			while !Self::is_peak(pos, last_pos_update - 1) {
				let (parent_pos, sibling_pos) = Self::family(pos);
				let sibling_key = if height == 0 {
					format!("{}:leaf:{}", self.prefix, sibling_pos)
				} else {
					format!("{}:node:{}:{}", self.prefix, height, sibling_pos)
				}?;

				let sibling = match txn.get(&sibling_key)? {
					Some(s) => s,
					None => return Err(Error::new(IllegalState)), // Shouldn’t reach here
				};

				hash = if sibling_pos < pos {
					self.hash_children(sibling, &hash)
				} else {
					self.hash_children(&hash, sibling)
				};

				height += 1;
				pos = parent_pos;

				last_pos_update += 1;
				let node_key = format!("{}:node:{}:{}", self.prefix, height, pos)?;
				txn.put(&node_key, &hash)?;

				let mut peaks = self.get_peaks(Some(txn.clone()))?;
				let mut peaks_len = peaks.len();
				if peaks_len > 0 && peaks[peaks_len - 1].pos() == sibling_pos {
					// merge
					let info = PeakInfo::new(hash, pos, height);
					peaks[peaks_len - 1] = info;

					// now we need to continue up the tree to check if we need
					// to continue to merge

					while peaks_len > 1 {
						let (parent_pos1, sibling_pos1) = Self::family(peaks[peaks_len - 2].pos());
						let (parent_pos2, sibling_pos2) = Self::family(peaks[peaks_len - 1].pos());
						if parent_pos1 == parent_pos2 {
							let sibling1_key =
								format!("{}:node:{}:{}", self.prefix, height, sibling_pos1)?;
							let sibling2_key =
								format!("{}:node:{}:{}", self.prefix, height, sibling_pos2)?;
							let sibling_hash1 = match txn.get(&sibling1_key)? {
								Some(s) => s,
								None => return Err(Error::new(IllegalState)), // Shouldn’t reach here
							};
							let sibling_hash2 = match txn.get(&sibling2_key)? {
								Some(s) => s,
								None => return Err(Error::new(IllegalState)), // Shouldn’t reach here
							};

							height += 1;

							// reverse order because sibling2 is
							// actually the lower one even thought
							// comes from the later peak
							hash = self.hash_children(&sibling_hash2, &sibling_hash1);
							last_pos_update += 1;
							let node_key =
								format!("{}:node:{}:{}", self.prefix, height, parent_pos1)?;
							txn.put(&node_key, &hash)?;
							peaks.truncate(peaks_len - 1)?;
							peaks[peaks_len - 2] = PeakInfo::new(hash, parent_pos1, height);
							peaks_len -= 1;
						} else {
							break;
						}
					}

					self.set_peaks(peaks, &mut txn)?;
					break;
				}
			}
		}

		to_le_bytes_u64(last_pos_update, &mut last_pos_bytes)?;
		let size_key = format!("{}:meta:size", self.prefix)?;
		txn.put(&size_key, &last_pos_bytes)?;
		if commit {
			txn.commit()?;
		}
		Ok(())
	}

	// prune data in the PMMR.
	// note: pruning must not happen to the most recently inserted leaf node.
	// This can be prevented by appending new outputs from a block before pruning the inputs.
	// This ensures it's valid.
	pub fn prune(&mut self, data: &[u8], txn: Option<LmdbTxn>) -> Result<(), Error> {
		let last_pos = self.last_pos(txn.clone())?;
		let (mut txn, commit) = self.get_write_txn(txn)?;

		let hash = self.hash_data(data);
		let data_key = format!("{}:data:{}", self.prefix, hash)?;
		match txn.get(&data_key)? {
			Some(data_bytes) => {
				let pos = from_le_bytes_u64(&data_bytes)?;
				if pos + 1 == last_pos {
					// prevent deleting most recent addition to PMMR.
					// this would cause an invalid state.
					// to ensure this is never needed,
					// append all outputs from a block before pruning inputs.
					return Err(Error::new(IllegalState));
				}
				let leaf_key = format!("{}:leaf:{}", self.prefix, pos)?;
				txn.del(&data_key)?;
				txn.del(&leaf_key)?;
			}
			None => return Err(Error::new(NotFound)),
		}

		if commit {
			txn.commit()?;
		}
		Ok(())
	}

	// determine whether the PMMR contains a particular piece of data.
	pub fn contains(&self, data: &[u8], txn: Option<LmdbTxn>) -> Result<Option<u64>, Error> {
		let txn = self.get_read_txn(txn)?;
		let hash = self.hash_data(data);
		let data_key = format!("{}:data:{}", self.prefix, hash)?;
		match txn.get(&data_key)? {
			Some(key) => Ok(Some(from_le_bytes_u64(key)?)),
			None => Ok(None),
		}
	}

	// return the root hash of the PMMR.
	pub fn root_hash(&self, txn: Option<LmdbTxn>) -> Result<[u8; 32], Error> {
		Err(Error::new(Todo))
	}

	// return the last position in the pmmr.
	pub fn last_pos(&self, txn: Option<LmdbTxn>) -> Result<u64, Error> {
		let txn = self.get_read_txn(txn)?;
		let size_key = format!("{}:meta:size", self.prefix)?;
		from_le_bytes_u64(self.get_key_with_default(&txn, &size_key, &ZERO_BYTES)?)
	}

	// return the peak info for the specified position. This peak info includes the hash,
	// height and position of the 4 highest subpeaks of each peak in the MMR. These are used distributed sync.
	pub fn peaks(&self, position: u64, txn: Option<LmdbTxn>) -> Result<Vec<PeakInfo>, Error> {
		Err(Error::new(Todo))
	}

	// rewind to specified position (reorgs - rewind to the position in the pmmr position in
	// the header)
	pub fn rewind(&mut self, last_pos: u64, txn: Option<LmdbTxn>) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	pub fn get_peaks(&self, txn: Option<LmdbTxn>) -> Result<Vec<PeakInfo>, Error> {
		let txn = self.get_read_txn(txn)?;
		let peaks_key = format!("{}:meta:peaks", self.prefix)?;
		let peaks: Vec<PeakInfo> = txn
			.get(&peaks_key)?
			.map(|bytes| Self::deserialize_peaks(bytes))
			.unwrap_or(Ok(Vec::new()))?;
		Ok(peaks)
	}

	// helpers
	fn set_peaks(&mut self, peaks: Vec<PeakInfo>, txn: &mut LmdbTxn) -> Result<(), Error> {
		let peaks_key = format!("{}:meta:peaks", self.prefix)?;
		let ser_peaks = Self::serialize_peaks(peaks)?;
		txn.put(&peaks_key, ser_peaks.as_ref())?;
		Ok(())
	}
	fn serialize_peaks(peaks: Vec<PeakInfo>) -> Result<Vec<u8>, Error> {
		let mut result = Vec::with_capacity(peaks.len() * 32)?;
		for peak in peaks {
			result.extend_from_slice(&peak.serialize()?)?;
		}
		Ok(result)
	}

	fn deserialize_peaks(bytes: &[u8]) -> Result<Vec<PeakInfo>, Error> {
		let bytes_len = bytes.len();
		if bytes_len % 41 != 0 {
			return Err(Error::new(IllegalArgument));
		}
		let mut ret = Vec::with_capacity(bytes_len / 41)?;
		let mut i = 0;
		while i + 41 <= bytes_len {
			let bytes_ref = subslice(bytes, i, 41)?;
			ret.push(PeakInfo::deserialize(bytes_ref)?)?;
			i += 41;
		}

		Ok(ret)
	}

	// Calculates the positions of the parent and sibling of the node at the
	// provided position.
	fn family(pos0: u64) -> (u64, u64) {
		let (peak_map, height) = Self::peak_map_height(pos0);
		let peak = 1 << height;
		if (peak_map & peak) != 0 {
			(pos0 + 1, pos0 + 1 - 2 * peak)
		} else {
			(pos0 + 2 * peak, pos0 + 2 * peak - 1)
		}
	}

	fn is_peak(pos: u64, last_pos: u64) -> bool {
		let (parent_pos, sibling_pos) = Self::family(pos);
		sibling_pos > last_pos
	}

	// peak bitmap and height of next node in mmr of given size
	// Example: on size 4 returns (0b11, 0) as mmr tree of size 4 is
	//    2
	//   / \
	//  0   1   3
	// with 0b11 indicating the presence of peaks of height 0 and 1,
	// and 0 the height of the next node 4, which is a leaf
	// NOTE:
	// the peak map also encodes the path taken from the root to the added node
	// since the path turns left (resp. right) if-and-only-if
	// a peak at that height is absent (resp. present)
	fn peak_map_height(mut size: u64) -> (u64, u64) {
		if size == 0 {
			// rust can't shift right by 64
			return (0, 0);
		}
		let mut peak_size = ALL_ONES >> size.leading_zeros();
		let mut peak_map = 0;
		while peak_size != 0 {
			peak_map <<= 1;
			if size >= peak_size {
				size -= peak_size;
				peak_map |= 1;
			}
			peak_size >>= 1;
		}
		(peak_map, size)
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

	fn hash_data(&self, data: &[u8]) -> [u8; 32] {
		let sha3 = Sha3_256::new();
		sha3.update(data);
		sha3.finalize()
	}

	fn get_key_with_default<'a>(
		&self,
		txn: &'a LmdbTxn,
		key: &String,
		default_value: &'a [u8],
	) -> Result<&'a [u8], Error> {
		Ok(txn.get(key)?.unwrap_or(default_value))
	}

	fn hash_children(&self, left: &[u8], right: &[u8]) -> [u8; 32] {
		let mut dual_hash = [0u8; 64];
		unsafe {
			copy_nonoverlapping(left.as_ptr(), dual_hash.as_mut_ptr(), 32);
			copy_nonoverlapping(right.as_ptr(), dual_hash.as_mut_ptr().offset(32), 32);
		}
		self.hash_data(&dual_hash)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use lmdb::{make_lmdb_test_dir, remove_lmdb_test_dir};

	/*
	6 (height=2)
	/ \
	2   5 (height=1)                  9
	/ \ / \                         /  \
	0  1 3  4                       7  8
	   */

	#[test]
	fn test_pmmr1() -> Result<(), Error> {
		let db_dir = "bin/.pmmr1";
		let db_size = 100 * 1024 * 1024;
		let db_name = "mydb";
		make_lmdb_test_dir(db_dir)?;

		let db = Lmdb::new(db_dir, db_name, db_size)?;

		// create pmmr
		let mut pmmr = Pmmr::new(db, "pmmr1")?;

		// initially 0 peaks
		assert_eq!(pmmr.get_peaks(None)?.len(), 0);
		// assert that last_pos is 0
		assert_eq!(pmmr.last_pos(None)?, 0);

		// append [0u8; 32]
		pmmr.append(&[0u8; 32], None)?;

		// there should be 1 peak now
		let peaks = pmmr.get_peaks(None)?;
		assert_eq!(peaks.len(), 1);
		assert_eq!(peaks[0].hash, pmmr.hash_data(&[0u8; 32]));
		assert_eq!(peaks[0].height, 0);
		assert_eq!(peaks[0].pos, 0);

		// assert that last_pos is 1
		assert_eq!(pmmr.last_pos(None)?, 1);

		// append [1u8; 32]
		pmmr.append(&[1u8; 32], None)?;

		// assert that last_pos is 3 (two leaves + 1 branch)
		assert_eq!(pmmr.last_pos(None)?, 3);

		// there's still 1 peak (parent of both children)
		let peaks = pmmr.get_peaks(None)?;
		assert_eq!(peaks.len(), 1);

		assert_eq!(peaks[0].height, 1);
		assert_eq!(peaks[0].pos, 2);
		// the peak hash is the hash of the hashes of the children
		let peak_hash =
			pmmr.hash_children(&pmmr.hash_data(&[0u8; 32]), &pmmr.hash_data(&[1u8; 32]));
		assert_eq!(peaks[0].hash, peak_hash);

		// append [2u8; 32]
		pmmr.append(&[2u8; 32], None)?;

		// there should now be 2 peaks - 1 has two children leaves the other is a leaf only
		let peaks = pmmr.get_peaks(None)?;
		assert_eq!(peaks.len(), 2);

		// the first peak remains
		assert_eq!(peaks[0].hash, peak_hash);
		assert_eq!(peaks[0].height, 1);
		assert_eq!(peaks[0].pos, 2);
		// second peak is the hash of the last child
		assert_eq!(peaks[1].hash, pmmr.hash_data(&[2; 32]));
		assert_eq!(peaks[1].height, 0);
		assert_eq!(peaks[1].pos, 3);

		assert_eq!(pmmr.last_pos(None)?, 4);

		// append [3u8; 32]
		pmmr.append(&[3u8; 32], None)?;

		assert_eq!(pmmr.last_pos(None)?, 7);
		// there should now be 1 peak a perfect binary tree
		let peaks = pmmr.get_peaks(None)?;
		assert_eq!(peaks.len(), 1);
		assert_eq!(peaks[0].height, 2);
		assert_eq!(peaks[0].pos, 6);
		let subpeak_hash =
			pmmr.hash_children(&pmmr.hash_data(&[2u8; 32]), &pmmr.hash_data(&[3u8; 32]));

		// previous peak hash hashed with our new subpeak should be the final mountain hash
		let hash_pos_6 = pmmr.hash_children(&peak_hash, &subpeak_hash);
		assert_eq!(peaks[0].hash, hash_pos_6);

		pmmr.append(&[4u8; 32], None)?;
		let peaks = pmmr.get_peaks(None)?;
		assert_eq!(peaks.len(), 2); // Peaks [6, 7]
		assert_eq!(pmmr.last_pos(None)?, 8);
		assert_eq!(peaks[0].hash, hash_pos_6);
		assert_eq!(peaks[1].hash, pmmr.hash_data(&[4u8; 32]));

		pmmr.append(&[5u8; 32], None)?;
		let peaks = pmmr.get_peaks(None)?;
		assert_eq!(peaks.len(), 2); // Peaks [6, 9]
		assert_eq!(pmmr.last_pos(None)?, 10);
		assert_eq!(peaks[0].hash, hash_pos_6);
		assert_eq!(peaks[1].pos, 9);
		assert_eq!(peaks[1].height, 1);
		assert_eq!(
			peaks[1].hash,
			pmmr.hash_children(&pmmr.hash_data(&[4u8; 32]), &pmmr.hash_data(&[5u8; 32]))
		);

		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}
}
