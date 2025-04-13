use lmdb::types::{MDB_dbi, MDB_env, MDB_txn, MDB_val};

extern "C" {
	pub fn mdb_env_create(env: *mut *mut MDB_env) -> i32;
	pub fn mdb_env_open(env: *mut MDB_env, path: *const u8, flags: u32, mode: u32) -> i32;
	pub fn mdb_env_close(env: *mut MDB_env);
	pub fn mdb_env_set_mapsize(env: *mut MDB_env, size: usize) -> i32;
	pub fn mdb_env_set_maxdbs(env: *mut MDB_env, size: usize) -> i32;
	pub fn mdb_txn_begin(
		env: *mut MDB_env,
		parent: *mut MDB_txn,
		flags: u32,
		txn: *mut *mut MDB_txn,
	) -> i32;
	pub fn mdb_txn_commit(txn: *mut MDB_txn) -> i32;
	pub fn mdb_txn_abort(txn: *mut MDB_txn);
	pub fn mdb_dbi_open(txn: *mut MDB_txn, name: *const u8, flags: u32, dbi: *mut MDB_dbi) -> i32;
	pub fn mdb_put(
		txn: *mut MDB_txn,
		dbi: MDB_dbi,
		key: *mut MDB_val,
		data: *mut MDB_val,
		flags: u32,
	) -> i32;
	pub fn mdb_get(txn: *mut MDB_txn, dbi: MDB_dbi, key: *mut MDB_val, data: *mut MDB_val) -> i32;
	pub fn mdb_del(txn: *mut MDB_txn, dbi: MDB_dbi, key: *mut MDB_val, data: *mut MDB_val) -> i32;
}
