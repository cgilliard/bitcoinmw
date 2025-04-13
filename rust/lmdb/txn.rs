use core::mem::forget;
use core::ptr::{null, null_mut};
use core::slice::from_raw_parts;
use lmdb::constants::{MDB_NOOVERWRITE, MDB_NOTFOUND, MDB_SUCCESS};
use lmdb::env::LmdbDb;
use lmdb::ffi::*;
use lmdb::types::{MDB_txn, MDB_val};
use prelude::*;

pub struct LmdbTxn {
	txn: *mut MDB_txn,
	write: bool,
}

impl LmdbTxn {
	pub fn new(txn: *mut MDB_txn, write: bool) -> Self {
		Self { txn, write }
	}
	pub fn txn(&self) -> *mut MDB_txn {
		self.txn
	}

	pub fn commit(self) -> Result<(), Error> {
		unsafe {
			if self.write {
				if mdb_txn_commit(self.txn) != MDB_SUCCESS {
					return Err(Error::new(LmdbCommit));
				}
			} else {
				mdb_txn_abort(self.txn);
			}
		}
		forget(self);
		Ok(())
	}

	pub fn put(
		&mut self,
		db: &LmdbDb,
		key: &[u8],
		value: &[u8],
		overwrite: bool,
	) -> Result<(), Error> {
		if !self.write {
			return Err(Error::new(IllegalState));
		}
		let mut key_val = MDB_val {
			mv_size: key.len(),
			mv_data: key.as_ptr() as *mut u8,
		};
		let mut data_val = MDB_val {
			mv_size: value.len(),
			mv_data: value.as_ptr() as *mut u8,
		};
		let flags = if overwrite { 0 } else { MDB_NOOVERWRITE };
		unsafe {
			if mdb_put(self.txn, db.dbi(), &mut key_val, &mut data_val, flags) != MDB_SUCCESS {
				return Err(Error::new(LmdbPut));
			}
		}
		Ok(())
	}

	pub fn get(&self, db: &LmdbDb, key: &[u8]) -> Result<Option<&[u8]>, Error> {
		let mut key_val = MDB_val {
			mv_size: key.len(),
			mv_data: key.as_ptr() as *mut u8,
		};
		let mut data_val = MDB_val {
			mv_size: 0,
			mv_data: null_mut(),
		};
		unsafe {
			let rc = mdb_get(self.txn, db.dbi(), &mut key_val, &mut data_val);
			if rc == MDB_NOTFOUND {
				return Ok(None);
			}
			if rc != MDB_SUCCESS {
				return Err(Error::new(LmdbGet));
			}
			let slice = from_raw_parts(data_val.mv_data, data_val.mv_size);
			Ok(Some(slice))
		}
	}

	pub fn del(&mut self, db: &LmdbDb, key: &[u8]) -> Result<(), Error> {
		if !self.write {
			return Err(Error::new(IllegalState));
		}
		let mut key_val = MDB_val {
			mv_size: key.len(),
			mv_data: key.as_ptr() as *mut u8,
		};
		unsafe {
			if mdb_del(self.txn, db.dbi(), &mut key_val, null_mut()) != MDB_SUCCESS {
				return Err(Error::new(LmdbDel));
			}
		}
		Ok(())
	}
}

impl Drop for LmdbTxn {
	fn drop(&mut self) {
		unsafe {
			if !self.txn.is_null() {
				mdb_txn_abort(self.txn);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use lmdb::env::LmdbEnv;

	#[test]
	fn test_lmdb1() -> Result<(), Error> {
		let env = LmdbEnv::new("bin", 1024 * 1024 * 100, 10)?;
		let db = env.open_db("mydb")?;
		let mut txn = db.write()?;
		let mut v = match txn.get(&db, &[0, 0, 0, 0])? {
			Some(v) => v[0],
			None => 0,
		};

		println!("v={}", v);
		v += 1;
		txn.put(&db, &[0, 0, 0, 0], &[v], true)?;

		txn.commit()?;

		Ok(())
	}
}
