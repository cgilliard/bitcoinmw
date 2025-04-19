use lmdb::txn::LmdbTxn;
use prelude::*;
use std::misc::{from_le_bytes_u64, to_le_bytes_u64};
use store::constants::ZERO_BYTES;

pub struct PmmrIndex {
	db: Lmdb,
	prefix: String,
}

impl TryClone for PmmrIndex {
	fn try_clone(&self) -> Result<Self, Error>
	where
		Self: Sized,
	{
		Ok(Self {
			db: self.db.try_clone()?,
			prefix: self.prefix.clone(),
		})
	}
}

impl PmmrIndex {
	pub fn new(db: Lmdb, prefix: &str) -> Result<Self, Error> {
		if prefix.len() == 0 {
			return Err(Error::new(IllegalArgument));
		}
		Ok(Self {
			db,
			prefix: String::new(prefix)?,
		})
	}

	pub fn append(&mut self, data: &[u8], txn: Option<LmdbTxn>) -> Result<u64, Error> {
		if data.is_empty() {
			return Err(Error::new(IllegalArgument));
		}
		let (mut txn, commit) = match txn {
			Some(txn) => (txn, false),
			None => (self.db.write()?, true),
		};
		let size_key = format!("{}:meta:size", self.prefix)?;
		let size = txn.get(&size_key)?.unwrap_or(&ZERO_BYTES);
		let size = from_le_bytes_u64(size);
		let leaf_key = format!("{}:leaf:{}", self.prefix, size)?;
		txn.put(&leaf_key, data)?;
		let mut size_bytes = [0u8; 8];
		to_le_bytes_u64(size + 1, &mut size_bytes);
		txn.put(&size_key, &size_bytes)?;
		if commit {
			txn.commit()?;
		}
		Ok(size)
	}

	pub fn prune(&mut self, index: u64, txn: Option<LmdbTxn>) -> Result<(), Error> {
		let (mut txn, commit) = match txn {
			Some(txn) => (txn, false),
			None => (self.db.write()?, true),
		};

		let size_key = format!("{}:meta:size", self.prefix)?;
		let size = txn.get(&size_key)?.unwrap_or(&ZERO_BYTES);
		let size = from_le_bytes_u64(size);

		if size == 0 {
			return Err(Error::new(IllegalState));
		}

		let leaf_key = format!("{}:leaf:{}", self.prefix, index)?;
		if !txn.del(&leaf_key)? {
			return Err(Error::new(NotFound));
		}

		if commit {
			txn.commit()
		} else {
			Ok(())
		}
	}

	pub fn truncate(&mut self, pos: u64, txn: Option<LmdbTxn>) -> Result<(), Error> {
		let (mut txn, commit) = match txn {
			Some(txn) => (txn, false),
			None => (self.db.write()?, true),
		};

		let size_key = format!("{}:meta:size", self.prefix)?;
		let size = txn.get(&size_key)?.unwrap_or(&ZERO_BYTES);
		let size = from_le_bytes_u64(size);

		if pos + 1 > size {
			return Err(Error::new(IllegalArgument));
		}

		let key_prefix = format!("{}:leaf:", self.prefix)?;
		for i in pos + 1..size {
			let leaf_key = format!("{}{}", key_prefix, i)?;
			// delete if it's not pruned.
			txn.del(&leaf_key)?;
		}

		let mut size_bytes = [0u8; 8];
		to_le_bytes_u64(pos + 1, &mut size_bytes);
		txn.put(&size_key, &size_bytes)?;

		if commit {
			txn.commit()
		} else {
			Ok(())
		}
	}

	pub fn last_pos(&self, txn: Option<LmdbTxn>) -> Result<i64, Error> {
		let txn = match txn {
			Some(txn) => txn,
			None => self.db.read()?,
		};
		let size_key = format!("{}:meta:size", self.prefix)?;
		let size = txn.get(&size_key)?.unwrap_or(&ZERO_BYTES);
		let size = from_le_bytes_u64(size);
		Ok(size as i64 - 1)
	}

	pub fn find(&self, index: u64, txn: Option<LmdbTxn>) -> Result<Vec<u8>, Error> {
		let txn = match txn {
			Some(txn) => txn,
			None => self.db.read()?,
		};
		let leaf_key = format!("{}:leaf:{}", self.prefix, index)?;
		match txn.get(&leaf_key)? {
			Some(v) => {
				let mut ret = Vec::with_capacity(v.len())?;
				for x in v {
					let _ = ret.push(*x);
				}
				Ok(ret)
			}
			None => Err(Error::new(NotFound)),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_pmmr_index() -> Result<(), Error> {
		let db_size = 1024 * 1024 * 10;
		let db_name = "pmmr";
		let db_dir = "bin/.pmmr_index";
		make_lmdb_test_dir(db_dir)?;

		let db = Lmdb::new(db_dir, db_name, db_size)?;
		let db2 = db.try_clone()?;
		let mut pmmr_index = PmmrIndex::new(db, "output_mmr")?;
		assert_eq!(pmmr_index.last_pos(None)?, -1); // -1 indicates empty

		let index1 = pmmr_index.append(&[0u8; 32], None)?;
		assert_eq!(pmmr_index.last_pos(None)?, 0);
		let index2 = pmmr_index.append(&[1u8; 32], None)?;
		let index3 = pmmr_index.append(&[2u8; 32], None)?;
		assert_eq!(pmmr_index.last_pos(None)?, 2);

		assert_eq!(index1, 0);
		assert_eq!(index2, 1);
		assert_eq!(index3, 2);

		assert_eq!(pmmr_index.find(0, None)?.slice(0, 32), [0u8; 32]);
		assert_eq!(pmmr_index.find(1, None)?.slice(0, 32), [1u8; 32]);
		assert_eq!(pmmr_index.find(2, None)?.slice(0, 32), [2u8; 32]);

		assert!(pmmr_index.prune(index2, None).is_ok());
		assert!(pmmr_index.prune(index2, None).is_err());
		assert_eq!(pmmr_index.last_pos(None)?, 2);

		let index4 = pmmr_index.append(&[3u8; 32], None)?;
		assert_eq!(index4, 3);

		let txn = Some(db2.read()?);
		assert_eq!(pmmr_index.last_pos(txn.clone())?, 3);
		assert_eq!(pmmr_index.last_pos(txn)?, 3);

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}

	#[test]
	fn test_pmmr_truncate() -> Result<(), Error> {
		let db_size = 1024 * 1024 * 10;
		let db_name = "pmmr";
		let db_dir = "bin/.pmmr_index_truncate";
		make_lmdb_test_dir(db_dir)?;

		let db = Lmdb::new(db_dir, db_name, db_size)?;
		let db2 = db.try_clone()?;
		let mut pmmr_index = PmmrIndex::new(db2, "output_mmr")?;

		{
			let w = db.write()?;
			let txn = Some(w);

			for i in 0..100 {
				pmmr_index.append(&[i as u8; 32], txn.clone())?;
			}

			txn.unwrap().commit()?;
		}

		assert_eq!(pmmr_index.find(75, None)?.slice(0, 32), [75u8; 32]);

		// last_pos is one less than number of appends because we start at 0.
		assert_eq!(pmmr_index.last_pos(None)?, 99);
		assert!(pmmr_index.prune(75, None).is_ok());
		assert_eq!(pmmr_index.find(75, None), Err(Error::new(NotFound)));
		assert_eq!(pmmr_index.find(76, None)?.slice(0, 32), [76u8; 32]);
		pmmr_index.truncate(50, None)?;
		assert_eq!(pmmr_index.find(76, None), Err(Error::new(NotFound)));
		assert_eq!(pmmr_index.last_pos(None)?, 50);

		assert_eq!(pmmr_index.append(&[100 as u8; 32], None)?, 51);
		assert_eq!(pmmr_index.last_pos(None)?, 51);

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}
}
