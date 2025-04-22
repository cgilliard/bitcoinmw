use core::ptr::null_mut;
use lmdb::constants::{FILE_MODE, MDB_CREATE, MDB_MAX_DBS, MDB_READONLY, MDB_SUCCESS};
use lmdb::ffi::*;
use lmdb::txn::LmdbTxn;
use lmdb::types::{MDB_dbi, MDB_env, MDB_txn};
use prelude::*;
use std::CStr;

struct LmdbEnv {
	env: *mut MDB_env,
}

impl Drop for LmdbEnv {
	fn drop(&mut self) {
		unsafe {
			if !self.env.is_null() {
				mdb_env_close(self.env);
				self.env = null_mut();
			}
		}
	}
}

impl TryClone for Lmdb {
	fn try_clone(&self) -> Result<Self, Error>
	where
		Self: Sized,
	{
		let env = self.env.clone();
		let c_path = CStr::new(self.c_path.as_str()?.to_str())?;
		let c_name = CStr::new(self.c_name.as_str()?.to_str())?;
		Ok(Self {
			env,
			dbi: self.dbi,
			map_size: self.map_size,
			c_path,
			c_name,
		})
	}
}

pub struct Lmdb {
	env: Rc<LmdbEnv>,
	dbi: MDB_dbi,
	map_size: usize,
	c_path: CStr,
	c_name: CStr,
}

impl Lmdb {
	pub fn new(path: &str, name: &str, map_size: usize) -> Result<Self, Error> {
		let env = Rc::new(LmdbEnv { env: null_mut() })?;
		let c_path = CStr::new(path)?;
		let c_name = CStr::new(name)?;
		let dbi = MDB_dbi(0);
		let mut lmdb = Lmdb {
			env,
			dbi,
			map_size,
			c_path,
			c_name,
		};
		lmdb.init()?;
		Ok(lmdb)
	}

	pub fn write(&self) -> Result<LmdbTxn, Error> {
		let mut txn: *mut MDB_txn = null_mut();
		if unsafe { mdb_txn_begin(self.env.env, null_mut(), 0, &mut txn) != MDB_SUCCESS } {
			return Err(Error::new(LmdbBeginTxn));
		}
		LmdbTxn::new(txn, self.dbi, true)
	}

	pub fn read(&self) -> Result<LmdbTxn, Error> {
		let mut txn: *mut MDB_txn = null_mut();
		let r = unsafe { mdb_txn_begin(self.env.env, null_mut(), MDB_READONLY, &mut txn) };
		if r != MDB_SUCCESS {
			return Err(Error::new(LmdbBeginTxn));
		}
		LmdbTxn::new(txn, self.dbi, false)
	}

	pub fn close(&mut self) {
		if !self.env.env.is_null() {
			unsafe { mdb_env_close(self.env.env) };
			self.env.env = null_mut();
		}
	}

	pub fn resize(&mut self, nsize: usize) -> Result<(), Error> {
		self.close();
		self.map_size = nsize;
		self.init()
	}

	pub fn size(&self) -> usize {
		self.map_size
	}

	fn init(&mut self) -> Result<(), Error> {
		self.env.env = null_mut();
		unsafe {
			if mdb_env_create(&mut self.env.env) != MDB_SUCCESS {
				return Err(Error::new(LmdbCreate));
			}
			if mdb_env_set_mapsize(self.env.env, self.map_size) != MDB_SUCCESS {
				self.close();
				return Err(Error::new(Alloc));
			}
			if mdb_env_set_maxdbs(self.env.env, MDB_MAX_DBS) != MDB_SUCCESS {
				self.close();
				return Err(Error::new(Alloc));
			}
			if mdb_env_open(self.env.env, self.c_path.as_ptr(), 0, FILE_MODE) != MDB_SUCCESS {
				self.close();
				return Err(Error::new(IO));
			}

			let mut txn: *mut MDB_txn = null_mut();
			if mdb_txn_begin(self.env.env, null_mut(), 0, &mut txn) != MDB_SUCCESS {
				return Err(Error::new(LmdbBeginTxn));
			}
			let rc = mdb_dbi_open(txn, self.c_name.as_ptr(), MDB_CREATE, &mut self.dbi);
			if rc != MDB_SUCCESS {
				return Err(Error::new(LmdbOpen));
			}
			if mdb_txn_commit(txn) != MDB_SUCCESS {
				return Err(Error::new(LmdbCommit));
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use lmdb::txn::test::{make_lmdb_test_dir, remove_lmdb_test_dir};

	#[test]
	fn test_lmdb3() -> Result<(), Error> {
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		let db_dir = "bin/.lmdb3";
		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;
			let a = String::new("a")?;
			let b = String::new("b")?;
			txn.put(&a, &b)?;
			txn.commit()?;
		}

		db.close();

		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;
			let a = String::new("a")?;
			let v = txn.get(&a)?.unwrap();
			assert_eq!(v[0], 'b' as u8);

			txn.del(&a)?;
			txn.commit()?;
		}

		db.close();
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let txn = db.read()?;
			let a = String::new("a")?;
			let v = txn.get(&a)?;
			assert!(v.is_none());
		}
		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}

	#[test]
	fn test_lmdb_resize() -> Result<(), Error> {
		let db_size = 1024 * 100;
		let db_name = "mydb";
		let db_dir = "bin/.lmdb4";
		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;
		let mut err = 0;
		{
			loop {
				let mut txn = db.write()?;
				let a = String::new("a")?;
				let b = [0u8; 1024 * 1024];
				match txn.put(&a, &b) {
					Ok(_) => {}
					Err(e) => {
						assert_eq!(e, Error::new(LmdbFull));
						err += 1;
						let nsize = 1024 * 1024 * 10;
						db.resize(nsize)?;
						continue;
					}
				}
				match txn.commit() {
					Ok(_) => {
						break;
					}
					Err(e) => {
						assert_eq!(e, Error::new(LmdbFull));
						err += 1;
						let nsize = 1024 * 1024 * 10;
						db.resize(nsize)?;
					}
				}
			}
		}
		remove_lmdb_test_dir(db_dir)?;
		assert_eq!(err, 1);

		Ok(())
	}

	#[test]
	fn test_lmdb_multi() -> Result<(), Error> {
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		let db_dir = "bin/.lmdb5";
		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;
			let a = String::new("a")?;
			let b = String::new("b")?;
			txn.put(&a, &b)?;
			txn.commit()?;
		}

		db.close();

		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		assert_eq!(db.size(), db_size);

		{
			let mut txn = db.write()?;
			let a = String::new("a")?;
			let v = txn.get(&a)?.unwrap();
			assert_eq!(v[0], 'b' as u8);

			txn.del(&a)?;
			txn.commit()?;
		}

		db.close();
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let txn = db.read()?;
			let a = String::new("a")?;
			let v = txn.get(&a)?;
			assert!(v.is_none());
		}

		let db_clone = db.try_clone()?;
		let db_clone2 = db.try_clone()?;

		{
			let mut txn = db.write()?;
			let a = String::new("a")?;
			let b = String::new("b")?;
			txn.put(&a, &b)?;

			let txn2 = db_clone.read()?;
			let a = String::new("a")?;
			let v = txn2.get(&a)?;
			assert!(v.is_none());

			txn.commit()?;
		}

		{
			let mut txn = db_clone.write()?;
			let a = String::new("a")?;
			let v = txn.get(&a)?.unwrap();
			assert_eq!(v[0], 'b' as u8);

			txn.del(&a)?;
			txn.commit()?;
		}

		{
			let txn = db_clone2.read()?;
			let a = String::new("a")?;
			let v = txn.get(&a)?;
			assert!(v.is_none());
		}

		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}
}
