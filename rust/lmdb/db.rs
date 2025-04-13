use core::ptr::null_mut;
use lmdb::constants::{FILE_MODE, MDB_CREATE, MDB_MAX_DBS, MDB_READONLY, MDB_SUCCESS};
use lmdb::ffi::*;
use lmdb::txn::LmdbTxn;
use lmdb::types::{MDB_dbi, MDB_env, MDB_txn};
use prelude::*;
use std::misc::CStr;

pub struct Lmdb {
	env: *mut MDB_env,
	dbi: MDB_dbi,
}

impl Drop for Lmdb {
	fn drop(&mut self) {
		unsafe {
			if !self.env.is_null() {
				mdb_env_close(self.env);
				self.env = null_mut();
			}
		}
	}
}

impl Lmdb {
	pub fn new(path: &str, name: &str, map_size: usize) -> Result<Self, Error> {
		let mut env: *mut MDB_env = null_mut();
		let mut dbi;
		unsafe {
			if mdb_env_create(&mut env) != MDB_SUCCESS {
				return Err(Error::new(LmdbCreate));
			}
			if mdb_env_set_mapsize(env, map_size) != MDB_SUCCESS {
				mdb_env_close(env);
				return Err(Error::new(Alloc));
			}
			if mdb_env_set_maxdbs(env, MDB_MAX_DBS) != MDB_SUCCESS {
				mdb_env_close(env);
				return Err(Error::new(Alloc));
			}
			let c_path = CStr::new(path)?;
			if mdb_env_open(env, c_path.as_ptr(), 0, FILE_MODE) != MDB_SUCCESS {
				mdb_env_close(env);
				return Err(Error::new(IO));
			}
			let name_cstr = CStr::new(name)?;
			let mut txn: *mut MDB_txn = null_mut();
			if mdb_txn_begin(env, null_mut(), 0, &mut txn) != MDB_SUCCESS {
				return Err(Error::new(LmdbBeginTxn));
			}
			dbi = MDB_dbi(0);
			let rc = mdb_dbi_open(txn, name_cstr.as_ptr(), MDB_CREATE, &mut dbi);
			if rc != MDB_SUCCESS {
				return Err(Error::new(LmdbOpen));
			}
			if mdb_txn_commit(txn) != MDB_SUCCESS {
				return Err(Error::new(LmdbCommit));
			}
		}
		Ok(Lmdb { env, dbi })
	}

	pub fn write(&self) -> Result<LmdbTxn, Error> {
		let mut txn: *mut MDB_txn = null_mut();
		unsafe {
			if mdb_txn_begin(self.env, null_mut(), 0, &mut txn) != MDB_SUCCESS {
				return Err(Error::new(LmdbBeginTxn));
			}
		}
		Ok(LmdbTxn::new(txn, self.dbi, true))
	}

	pub fn read(&self) -> Result<LmdbTxn, Error> {
		let mut txn: *mut MDB_txn = null_mut();
		unsafe {
			if mdb_txn_begin(self.env, null_mut(), MDB_READONLY, &mut txn) != MDB_SUCCESS {
				return Err(Error::new(LmdbBeginTxn));
			}
		}
		Ok(LmdbTxn::new(txn, self.dbi, false))
	}

	pub fn close(&mut self) -> Result<(), Error> {
		unsafe {
			if !self.env.is_null() {
				mdb_env_close(self.env);
				self.env = null_mut();
			}
		}
		Ok(())
	}
}
