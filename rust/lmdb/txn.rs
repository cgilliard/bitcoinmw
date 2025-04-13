use core::convert::AsRef;
use core::mem::forget;
use core::ptr::null_mut;
use core::slice::from_raw_parts;
use lmdb::constants::{MDB_NOTFOUND, MDB_SUCCESS};
use lmdb::ffi::*;
use lmdb::types::{MDB_dbi, MDB_txn, MDB_val};
use prelude::*;

pub struct LmdbTxn {
	txn: *mut MDB_txn,
	dbi: MDB_dbi,
	write: bool,
}

impl LmdbTxn {
	pub fn new(txn: *mut MDB_txn, dbi: MDB_dbi, write: bool) -> Self {
		Self { txn, dbi, write }
	}

	pub fn get<K: AsRef<[u8]>>(&self, key: &K) -> Result<Option<&[u8]>, Error> {
		let mut key_val = MDB_val {
			mv_size: key.as_ref().len(),
			mv_data: key.as_ref().as_ptr() as *mut u8,
		};
		let mut data_val = MDB_val {
			mv_size: 0,
			mv_data: null_mut(),
		};
		unsafe {
			let rc = mdb_get(self.txn, self.dbi, &mut key_val, &mut data_val);
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

	pub fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&mut self, key: &K, value: &V) -> Result<(), Error> {
		if !self.write {
			return Err(Error::new(IllegalState));
		}
		let mut key_val = MDB_val {
			mv_size: key.as_ref().len(),
			mv_data: key.as_ref().as_ptr() as *mut u8,
		};
		let mut data_val = MDB_val {
			mv_size: value.as_ref().len(),
			mv_data: value.as_ref().as_ptr() as *mut u8,
		};
		unsafe {
			if mdb_put(self.txn, self.dbi, &mut key_val, &mut data_val, 0) != MDB_SUCCESS {
				return Err(Error::new(LmdbPut));
			}
		}
		Ok(())
	}

	pub fn del<K: AsRef<[u8]>>(&mut self, key: &K) -> Result<(), Error> {
		if !self.write {
			return Err(Error::new(IllegalState));
		}
		let mut key_val = MDB_val {
			mv_size: key.as_ref().len(),
			mv_data: key.as_ref().as_ptr() as *mut u8,
		};
		unsafe {
			if mdb_del(self.txn, self.dbi, &mut key_val, null_mut()) != MDB_SUCCESS {
				return Err(Error::new(LmdbDel));
			}
		}
		Ok(())
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
	use lmdb::db::Lmdb;

	#[test]
	fn test_lmdb1() -> Result<(), Error> {
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		let db_dir = "bin";
		let target = String::new("abc")?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;
		{
			let mut txn = db.write()?;
			let mut v = match txn.get(&target)? {
				Some(v) => v[0],
				None => 0,
			};

			//println!("v={}", v);
			v += 1;
			if v >= 10 {
				txn.del(&target)?;
			} else {
				let vs = String::newb(&[v])?;
				txn.put(&target, &vs)?;
			}

			let k1 = String::new("def")?;
			let v1 = Box::new(1)?;
			txn.put(&k1, &v1)?;

			txn.commit()?;
		}

		db.close()?;

		let db = Lmdb::new(db_dir, db_name, db_size)?;
		{
			let txn = db.read()?;
			let _v = match txn.get(&target)? {
				Some(v) => v[0],
				None => 0,
			};
			//println!("v2={}", _v);
		}

		Ok(())
	}

	#[test]
	fn test_lmdb2() -> Result<(), Error> {
		/*
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb2";
		let db_dir = "bin";
		let db = Lmdb::new(db_dir, db_name, db_size)?;
		{
			let txn = db.write()?;
			let txn2 = db.read()?;

			txn.commit()?;
		}

		let txn = db.write()?;
			*/
		Ok(())
	}
}
