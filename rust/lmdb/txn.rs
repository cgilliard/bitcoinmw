use core::convert::AsRef;
use core::iter::Iterator;
use core::mem::forget;
use core::ptr::null_mut;
use core::slice::from_raw_parts;
use core::str::from_utf8;
use lmdb::constants::{MDB_GET_CURRENT, MDB_NEXT, MDB_NOTFOUND, MDB_SET_RANGE, MDB_SUCCESS};
use lmdb::ffi::*;
use lmdb::types::{MDB_cursor, MDB_dbi, MDB_txn, MDB_val};
use prelude::*;
use std::misc::CStr;

pub struct LmdbTxn {
	txn: *mut MDB_txn,
	dbi: MDB_dbi,
	write: bool,
}

pub struct LmdbCursor {
	cursor: *mut MDB_cursor,
	prefix: CStr,
	is_first: bool,
}

impl Iterator for LmdbCursor {
	type Item = CStr;

	fn next(&mut self) -> Option<Self::Item> {
		unsafe {
			let mut key_val = MDB_val {
				mv_size: self.prefix.len(),
				mv_data: self.prefix.as_mut_ptr(),
			};

			let mut data_val = MDB_val {
				mv_size: 0,
				mv_data: null_mut(),
			};

			let rc = if self.is_first {
				// First call: use current position from MDB_SET_RANGE
				self.is_first = false;
				mdb_cursor_get(self.cursor, &mut key_val, &mut data_val, MDB_GET_CURRENT)
			} else {
				// Subsequent calls: move to next key
				mdb_cursor_get(self.cursor, &mut key_val, &mut data_val, MDB_NEXT)
			};

			if rc == MDB_NOTFOUND {
				return None;
			}
			if rc != MDB_SUCCESS {
				return None; // Silently stop on error (like get returning None)
			}

			let prefix_string = match self.prefix.as_str() {
				Ok(ps) => ps,
				Err(_) => return None, // Silently stop unexpected state
			};

			// Convert key to CStr to check prefix
			let key_slice = from_raw_parts(key_val.mv_data, key_val.mv_size);
			match String::newb(key_slice) {
				Ok(s) => {
					if s.find(prefix_string.to_str()) != Some(0) {
						return None;
					}
					match CStr::new(s.to_str()) {
						Ok(_) => {}
						Err(_) => return None,
					}
				}
				Err(_) => return None,
			}

			let data_cstr = CStr::from_ptr(data_val.mv_data, true);
			Some(data_cstr)
		}
	}
}

impl Drop for LmdbCursor {
	fn drop(&mut self) {
		unsafe {
			if !self.cursor.is_null() {
				mdb_cursor_close(self.cursor);
			}
		}
	}
}

impl LmdbTxn {
	pub fn new(txn: *mut MDB_txn, dbi: MDB_dbi, write: bool) -> Self {
		Self { txn, dbi, write }
	}

	pub fn iter<K: AsRef<[u8]>>(&self, key: &K) -> Result<LmdbCursor, Error> {
		unsafe {
			let mut cursor: *mut MDB_cursor = null_mut();
			let rc = mdb_cursor_open(self.txn, self.dbi, &mut cursor);
			if rc != MDB_SUCCESS {
				return Err(Error::new(IllegalState));
			}

			// Position cursor at key or next
			let key_bytes = key.as_ref();
			let prefix_cstr = match from_utf8(key_bytes) {
				Ok(s) => CStr::new(s)?,
				Err(_) => return Err(Error::new(IllegalState)),
			};
			let mut key_val = MDB_val {
				mv_size: key_bytes.len(),
				mv_data: key_bytes.as_ptr() as *mut u8,
			};
			let mut data_val = MDB_val {
				mv_size: 0,
				mv_data: null_mut(),
			};
			let rc = mdb_cursor_get(cursor, &mut key_val, &mut data_val, MDB_SET_RANGE);
			if rc != MDB_SUCCESS && rc != MDB_NOTFOUND {
				mdb_cursor_close(cursor);
				return Err(Error::new(IllegalState));
			}

			Ok(LmdbCursor {
				//txn: self.txn,
				//dbi: self.dbi,
				cursor,
				prefix: prefix_cstr,
				is_first: true,
			})
		}
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
pub mod test {
	use super::*;
	use lmdb::db::Lmdb;
	use std::ffi::{mkdir, rmdir, unlink};
	use std::misc::CStr;

	pub fn make_lmdb_test_dir(s: &str) -> Result<(), Error> {
		remove_lmdb_test_dir(s)?;
		let cstr = CStr::new(s)?;
		unsafe {
			mkdir(cstr.as_ptr(), 0o700);
		}
		Ok(())
	}

	pub fn remove_lmdb_test_dir(s: &str) -> Result<(), Error> {
		let data_file = format!("{}/data.mdb", s)?;
		let lock_file = format!("{}/lock.mdb", s)?;
		let dir = CStr::new(s)?;
		let data = CStr::new(data_file.to_str())?;
		let lock = CStr::new(lock_file.to_str())?;

		unsafe {
			unlink(data.as_ptr());
			unlink(lock.as_ptr());
			rmdir(dir.as_ptr());
		}
		Ok(())
	}

	#[test]
	fn test_lmdb1() -> Result<(), Error> {
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		let db_dir = "bin/.lmdb";
		make_lmdb_test_dir(db_dir)?;
		let target = String::new("abc")?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;
		{
			let mut txn = db.write()?;
			let mut v = match txn.get(&target)? {
				Some(v) => v[0],
				None => 0,
			};

			assert_eq!(v, 0);

			v += 1;
			if v >= 10 {
				txn.del(&target)?;
			} else {
				let vs = String::newb(&[v])?;
				txn.put(&target, &vs)?;
			}

			txn.commit()?;
		}

		db.close()?;

		let db = Lmdb::new(db_dir, db_name, db_size)?;
		{
			let txn = db.read()?;
			let v = match txn.get(&target)? {
				Some(v) => v[0],
				None => 0,
			};
			assert_eq!(v, 1);
		}

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}

	#[test]
	fn test_lmdb2() -> Result<(), Error> {
		let db_dir = "bin/.lmdb2";
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		let target = String::new("abc")?;

		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;
			let mut v = match txn.get(&target)? {
				Some(v) => v[0],
				None => 0,
			};

			assert_eq!(v, 0);

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
			let v = match txn.get(&target)? {
				Some(v) => v[0],
				None => 0,
			};

			assert_eq!(v, 1);
		}

		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}

	#[test]
	fn test_lmdb_cursor() -> Result<(), Error> {
		let db_dir = "bin/.lmdb_cursor";
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		make_lmdb_test_dir(db_dir)?;
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;

			let target = String::new("test3")?;
			let v = ['x' as u8];
			txn.put(&target, &v)?;

			let target = String::new("test2")?;
			let v = ['y' as u8];
			txn.put(&target, &v)?;

			let target = String::new("test1")?;
			let v = ['z' as u8];
			txn.put(&target, &v)?;

			let target = String::new("test1")?;
			let out = txn.get(&target)?;
			assert_eq!(unsafe { *out.unwrap().as_ptr() }, 'z' as u8);

			let target = String::new("test2")?;
			let out = txn.get(&target)?;
			assert_eq!(unsafe { *out.unwrap().as_ptr() }, 'y' as u8);

			let target = String::new("test3")?;
			let out = txn.get(&target)?;
			assert_eq!(unsafe { *out.unwrap().as_ptr() }, 'x' as u8);

			let mut i = 0;
			for v in txn.iter(&String::new("test")?)? {
				if i == 0 {
					assert_eq!(unsafe { *v.as_ptr() }, 'z' as u8);
					assert_eq!(v.as_bytes()?[0], 'z' as u8);
				} else if i == 1 {
					assert_eq!(unsafe { *v.as_ptr() }, 'y' as u8);
					assert_eq!(v.as_bytes()?[0], 'y' as u8);
				} else if i == 2 {
					assert_eq!(unsafe { *v.as_ptr() }, 'x' as u8);
					assert_eq!(v.as_bytes()?[0], 'x' as u8);
				}
				i += 1;
			}
			assert_eq!(i, 3);
			txn.commit()?;
		}

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}
}
