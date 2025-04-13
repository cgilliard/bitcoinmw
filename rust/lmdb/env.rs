use core::ptr::{null, null_mut};
use lmdb::constants::{FILE_MODE, MDB_CREATE, MDB_READONLY, MDB_SUCCESS};
use lmdb::ffi::*;
use lmdb::txn::LmdbTxn;
use lmdb::types::{MDB_dbi, MDB_env, MDB_txn};
use prelude::*;
use std::ffi::{alloc, release};
use std::misc::CStr;

pub struct LmdbEnv {
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

impl LmdbEnv {
	pub fn new(path: &str, map_size: usize, max_dbs: usize) -> Result<Self, Error> {
		let mut env: *mut MDB_env = null_mut();
		unsafe {
			if mdb_env_create(&mut env) != MDB_SUCCESS {
				return Err(Error::new(LmdbCreate));
			}
			if mdb_env_set_mapsize(env, map_size) != MDB_SUCCESS {
				mdb_env_close(env);
				return Err(Error::new(Alloc));
			}
			if mdb_env_set_maxdbs(env, max_dbs) != MDB_SUCCESS {
				mdb_env_close(env);
				return Err(Error::new(Alloc));
			}
			let c_path = CStr::new(path)?;
			if mdb_env_open(env, c_path.as_ptr(), 0, FILE_MODE) != MDB_SUCCESS {
				mdb_env_close(env);
				return Err(Error::new(IO));
			}
		}
		Ok(LmdbEnv { env })
	}

	pub fn begin_txn(&self, write: bool) -> Result<LmdbTxn, Error> {
		let mut txn: *mut MDB_txn = null_mut();
		unsafe {
			let flags = if write { 0 } else { MDB_READONLY };
			if mdb_txn_begin(self.env, null_mut(), flags, &mut txn) != MDB_SUCCESS {
				return Err(Error::new(LmdbBeginTxn));
			}
		}
		Ok(LmdbTxn::new(txn, write))
	}

	pub fn open_db(&self, name: &str) -> Result<LmdbDb, Error> {
		let name_cstr = CStr::new(name)?;
		let mut txn = self.begin_txn(true)?;
		let mut dbi = MDB_dbi(0);
		unsafe {
			let rc = mdb_dbi_open(txn.txn(), name_cstr.as_ptr(), MDB_CREATE, &mut dbi);
			if rc != MDB_SUCCESS {
				return Err(Error::new(LmdbOpen));
			}
		}
		txn.commit()?;
		let env = self;
		Ok(LmdbDb { dbi, env })
	}
}

pub struct LmdbDb<'a> {
	dbi: MDB_dbi,
	env: &'a LmdbEnv,
}

impl LmdbDb<'_> {
	pub fn dbi(&self) -> MDB_dbi {
		self.dbi
	}

	pub fn write(&self) -> Result<LmdbTxn, Error> {
		self.env.begin_txn(true)
	}

	pub fn read(&self) -> Result<LmdbTxn, Error> {
		self.env.begin_txn(false)
	}
}
