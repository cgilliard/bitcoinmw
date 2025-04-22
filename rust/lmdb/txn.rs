use core::convert::AsRef;
use core::iter::Iterator;
use core::mem::forget;
use core::ptr::null_mut;
use core::slice::from_raw_parts;
use lmdb::constants::{
	MDB_GET_CURRENT, MDB_MAP_FULL, MDB_NEXT, MDB_NOTFOUND, MDB_PREV, MDB_SET_RANGE, MDB_SUCCESS,
};
use lmdb::ffi::*;
use lmdb::types::{MDB_cursor, MDB_dbi, MDB_txn, MDB_val};
use prelude::*;
use std::misc::{slice_copy, slice_starts_with};
use std::CString;

struct LmdbTxnInner {
	txn: *mut MDB_txn,
}

pub struct LmdbTxn {
	txn: Rc<LmdbTxnInner>,
	dbi: MDB_dbi,
	write: bool,
}

pub struct LmdbCursor {
	cursor: *mut MDB_cursor,
	prefix: CString,
	is_first: bool,
}

impl Drop for LmdbTxnInner {
	fn drop(&mut self) {
		unsafe {
			if !self.txn.is_null() {
				mdb_txn_abort(self.txn);
				self.txn = null_mut();
			}
		}
	}
}

impl Clone for LmdbTxn {
	fn clone(&self) -> Self {
		Self {
			txn: self.txn.clone(),
			dbi: self.dbi,
			write: self.write,
		}
	}
}

impl Drop for LmdbCursor {
	fn drop(&mut self) {
		self.close();
	}
}

impl Iterator for LmdbCursor {
	type Item = (Vec<u8>, Vec<u8>);

	fn next(&mut self) -> Option<Self::Item> {
		if self.cursor.is_null() {
			return None;
		}
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
				self.is_first = false;
				mdb_cursor_get(self.cursor, &mut key_val, &mut data_val, MDB_GET_CURRENT)
			} else {
				mdb_cursor_get(self.cursor, &mut key_val, &mut data_val, MDB_NEXT)
			};

			if rc != MDB_SUCCESS {
				return None;
			}

			let key_slice = from_raw_parts(key_val.mv_data, key_val.mv_size);
			let mut key_vec = Vec::new();
			match key_vec.extend_from_slice(key_slice) {
				Ok(_) => {}
				Err(_e) => return None,
			}

			let prefix_slice = from_raw_parts(self.prefix.as_ptr(), self.prefix.len());
			if !slice_starts_with(key_vec.as_ref(), prefix_slice) {
				return None;
			}

			let data_slice = from_raw_parts(data_val.mv_data, data_val.mv_size);
			let mut data_vec = Vec::new();
			match data_vec.extend_from_slice(data_slice) {
				Ok(_) => {}
				Err(_e) => return None,
			}

			Some((key_vec, data_vec))
		}
	}
}

impl LmdbCursor {
	pub fn next_bytes(
		&mut self,
		key: &mut [u8],
		value: &mut [u8],
	) -> Result<Option<(usize, usize)>, Error> {
		if self.cursor.is_null() {
			return Err(Error::new(IllegalState));
		}
		unsafe {
			let mut key_val = MDB_val {
				mv_size: 0,
				mv_data: null_mut(),
			};

			let mut data_val = MDB_val {
				mv_size: 0,
				mv_data: null_mut(),
			};

			let rc = if self.is_first {
				self.is_first = false;
				mdb_cursor_get(self.cursor, &mut key_val, &mut data_val, MDB_GET_CURRENT)
			} else {
				mdb_cursor_get(self.cursor, &mut key_val, &mut data_val, MDB_NEXT)
			};

			if rc == MDB_NOTFOUND {
				return Ok(None);
			} else if rc != MDB_SUCCESS {
				return Err(Error::new(LmdbCursor));
			}

			let key_len = if key_val.mv_size > key.len() {
				key.len()
			} else {
				key_val.mv_size
			};

			let key_slice = from_raw_parts(key_val.mv_data, key_val.mv_size);

			let prefix_slice = from_raw_parts(self.prefix.as_ptr(), self.prefix.len());
			if !slice_starts_with(key_slice, prefix_slice) {
				return Ok(None);
			}

			slice_copy(key_slice, key, key_len)?;

			let value_len = if data_val.mv_size > value.len() {
				value.len()
			} else {
				data_val.mv_size
			};

			let data_slice = from_raw_parts(data_val.mv_data, data_val.mv_size);
			slice_copy(data_slice, value, value_len)?;

			Ok(Some((key_val.mv_size, data_val.mv_size)))
		}
	}

	pub fn rewind(&mut self) -> Result<(), Error> {
		let mut key_val = MDB_val {
			mv_size: 0,
			mv_data: null_mut(),
		};
		let mut data_val = MDB_val {
			mv_size: 0,
			mv_data: null_mut(),
		};

		// If at initial position, no previous key to rewind to
		if self.is_first {
			self.close();
			return Err(Error::new(OutOfBounds));
		}

		// Move to previous key
		let res = unsafe { mdb_cursor_get(self.cursor, &mut key_val, &mut data_val, MDB_PREV) };
		if res == MDB_NOTFOUND {
			self.is_first = true;
			Ok(())
		} else if res != MDB_SUCCESS {
			self.close();
			Err(Error::new(LmdbCursor))
		} else {
			let key_slice = unsafe { from_raw_parts(key_val.mv_data, key_val.mv_size) };
			let prefix_slice = unsafe { from_raw_parts(self.prefix.as_ptr(), self.prefix.len()) };
			if !slice_starts_with(key_slice, prefix_slice) {
				self.close();
				return Err(Error::new(OutOfBounds));
			}

			Ok(())
		}
	}

	fn close(&mut self) {
		if !self.cursor.is_null() {
			unsafe {
				mdb_cursor_close(self.cursor);
			}
			self.cursor = null_mut();
		}
	}
}

impl LmdbTxn {
	pub fn new(txn: *mut MDB_txn, dbi: MDB_dbi, write: bool) -> Result<Self, Error> {
		Ok(Self {
			txn: Rc::new(LmdbTxnInner { txn })?,
			dbi,
			write,
		})
	}

	pub fn iter<K: AsRef<[u8]> + ?Sized>(&self, key: &K) -> Result<LmdbCursor, Error> {
		if self.txn.txn.is_null() {
			return Err(Error::new(IllegalState));
		}
		unsafe {
			let mut cursor: *mut MDB_cursor = null_mut();
			let rc = mdb_cursor_open(self.txn.txn, self.dbi, &mut cursor);
			if rc != MDB_SUCCESS {
				return Err(Error::new(LmdbCursor));
			}

			// Create prefix from raw bytes, supporting binary data
			let key_bytes = key.as_ref();
			let prefix_cstr = CString::from_slice(key_bytes)?;

			// Position cursor at key or next
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
				return Err(Error::new(LmdbCursor));
			}

			Ok(LmdbCursor {
				cursor,
				prefix: prefix_cstr,
				is_first: true,
			})
		}
	}

	pub fn get<K: AsRef<[u8]> + ?Sized>(&self, key: &K) -> Result<Option<&[u8]>, Error> {
		if self.txn.txn.is_null() {
			return Err(Error::new(IllegalState));
		}
		let mut key_val = MDB_val {
			mv_size: key.as_ref().len(),
			mv_data: key.as_ref().as_ptr() as *mut u8,
		};
		let mut data_val = MDB_val {
			mv_size: 0,
			mv_data: null_mut(),
		};
		unsafe {
			let rc = mdb_get(self.txn.txn, self.dbi, &mut key_val, &mut data_val);
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

	pub fn put<K: AsRef<[u8]> + ?Sized, V: AsRef<[u8]> + ?Sized>(
		&mut self,
		key: &K,
		value: &V,
	) -> Result<(), Error> {
		if self.txn.txn.is_null() || !self.write {
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
			let r = mdb_put(self.txn.txn, self.dbi, &mut key_val, &mut data_val, 0);
			if r == MDB_SUCCESS {
				Ok(())
			} else if r == MDB_MAP_FULL {
				// set txn to null because drop should not abort in this case
				self.txn.txn = null_mut();
				return Err(Error::new(LmdbFull));
			} else {
				return Err(Error::new(LmdbPut));
			}
		}
	}

	pub fn del<K: AsRef<[u8]> + ?Sized>(&mut self, key: &K) -> Result<bool, Error> {
		if self.txn.txn.is_null() || !self.write {
			return Err(Error::new(IllegalState));
		}
		let mut key_val = MDB_val {
			mv_size: key.as_ref().len(),
			mv_data: key.as_ref().as_ptr() as *mut u8,
		};
		unsafe {
			let r = mdb_del(self.txn.txn, self.dbi, &mut key_val, null_mut());
			if r == MDB_SUCCESS {
				Ok(true)
			} else if r == MDB_NOTFOUND {
				Ok(false)
			} else if r == MDB_MAP_FULL {
				// set txn to null because drop should not abort in this case
				self.txn.txn = null_mut();
				return Err(Error::new(LmdbFull));
			} else {
				return Err(Error::new(LmdbDel));
			}
		}
	}

	pub fn commit(mut self) -> Result<(), Error> {
		if self.txn.txn.is_null() || !self.write {
			return Err(Error::new(IllegalState));
		}
		unsafe {
			let r = mdb_txn_commit(self.txn.txn);
			if r != MDB_SUCCESS {
				if r == MDB_MAP_FULL {
					self.txn.txn = null_mut();
					return Err(Error::new(LmdbFull));
				} else {
					return Err(Error::new(LmdbCommit));
				}
			}
		}
		forget(self);
		Ok(())
	}
}

#[cfg(test)]
pub mod test {
	use super::*;
	use core::convert::AsMut;
	use lmdb::db::Lmdb;
	use std::ffi::{mkdir, rmdir, unlink};

	pub fn make_lmdb_test_dir(s: &str) -> Result<(), Error> {
		remove_lmdb_test_dir(s)?;
		let cstr = CString::new(s)?;
		unsafe {
			mkdir(cstr.as_ptr(), 0o700);
		}
		Ok(())
	}

	pub fn remove_lmdb_test_dir(s: &str) -> Result<(), Error> {
		let data_file = format!("{}/data.mdb", s)?;
		let lock_file = format!("{}/lock.mdb", s)?;
		let dir = CString::new(s)?;
		let data = CString::new(data_file.to_str())?;
		let lock = CString::new(lock_file.to_str())?;

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
				txn.put(&target, &[v])?;
			}

			assert_eq!(txn.get(&String::new("def")?)?, None);

			txn.commit()?;
		}

		db.close();

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
				txn.put(&target, &[v])?;
			}

			let k1 = String::new("def")?;
			let v1 = Box::new(1)?;
			txn.put(&k1, &v1)?;

			txn.commit()?;
		}

		db.close();

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

			let target = String::new("zzz")?;
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
			for (k, v) in txn.iter(&String::new("test")?)? {
				if i == 0 {
					assert_eq!(unsafe { *v.as_ptr() }, 'z' as u8);
					assert_eq!(v[0], 'z' as u8);
					assert_eq!(k, vec![b't', b'e', b's', b't', b'1']?);
				} else if i == 1 {
					assert_eq!(unsafe { *v.as_ptr() }, 'y' as u8);
					assert_eq!(v[0], 'y' as u8);
					assert_eq!(k, vec![b't', b'e', b's', b't', b'2']?);
				} else if i == 2 {
					assert_eq!(unsafe { *v.as_ptr() }, 'x' as u8);
					assert_eq!(v[0], 'x' as u8);
					assert_eq!(k, vec![b't', b'e', b's', b't', b'3']?);
				}
				i += 1;
			}
			assert_eq!(i, 3);
			txn.commit()?;
		}

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}

	#[test]
	fn test_simple_iter() -> Result<(), Error> {
		let db_dir = "bin/.lmdb_simple_iter";
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;

			txn.put(&String::new("test3")?, &['x' as u8; 1])?;
			txn.put(&String::new("test2")?, &['y' as u8; 1])?;
			txn.put(&String::new("test1")?, &['z' as u8; 1])?;
			txn.commit()?;
		}

		db.close();
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let txn = db.read()?;
			let mut values = Vec::new();
			let mut keys = Vec::new();
			for (k, v) in txn.iter(&String::new("test")?)? {
				keys.push(k)?;
				values.push(v)?;
			}

			assert_eq!(keys.len(), 3);
			assert_eq!(keys[0], vec![b't', b'e', b's', b't', b'1']?);
			assert_eq!(keys[1], vec![b't', b'e', b's', b't', b'2']?);
			assert_eq!(keys[2], vec![b't', b'e', b's', b't', b'3']?);

			assert_eq!(values[0], vec!['z' as u8]?);
			assert_eq!(values[1], vec!['y' as u8]?);
			assert_eq!(values[2], vec!['x' as u8]?);
		}

		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}

	#[test]
	fn test_raw_iter() -> Result<(), Error> {
		let db_dir = "bin/.lmdb_raw_iter";
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		let test1 = "test1".as_bytes();
		let test2 = "test2".as_bytes();
		let test3 = "test3".as_bytes();
		let test4 = "zzz".as_bytes();

		{
			let mut txn = db.write()?;

			txn.put(&test3, &['x' as u8; 1])?;
			txn.put(&test2, &['y' as u8; 1])?;
			txn.put(&test1, &['z' as u8; 1])?;
			txn.put(&test4, &['a' as u8; 1])?;
			txn.commit()?;
		}

		db.close();
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let txn = db.read()?;
			let test = [b't', b'e', b's', b't'];
			let mut iter = txn.iter(&test)?;
			let mut count = 0;
			loop {
				let mut key = [0u8; 128];
				let mut value = [0u8; 128];
				let res = iter.next_bytes(&mut key, &mut value)?;
				match res {
					Some((klen, vlen)) => {
						if count == 0 {
							assert_eq!(&key[0..5], "test1".as_bytes());
							assert_eq!(klen, 5);
							assert_eq!(vlen, 1);
							assert_eq!(value[0], b'z');
						} else if count == 1 {
							assert_eq!(&key[0..5], "test2".as_bytes());
							assert_eq!(klen, 5);
							assert_eq!(vlen, 1);
							assert_eq!(value[0], b'y');
						} else {
							assert_eq!(&key[0..5], "test3".as_bytes());
							assert_eq!(klen, 5);
							assert_eq!(vlen, 1);
							assert_eq!(value[0], b'x');
						}
						count += 1;
					}
					None => break,
				}
			}

			assert_eq!(count, 3);
		}

		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}

	#[test]
	fn test_iter_rewind() -> Result<(), Error> {
		let db_dir = "bin/.lmdb_iter_rewind";
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;

			txn.put(&String::new("test3")?, &['x' as u8; 1])?;
			txn.put(&String::new("test2")?, &['y' as u8; 1])?;
			txn.put(&String::new("test1")?, &['z' as u8; 1])?;
			txn.put(&String::new("zzz")?, &['a' as u8; 1])?;
			txn.commit()?;
		}

		db.close();
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let txn = db.read()?;
			let mut iter = txn.iter(&String::new("test")?)?;

			let (k, _v) = iter.next().unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'1']?);
			let (k, _v) = iter.next().unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'2']?);
			iter.rewind()?;
			let (k, _v) = iter.next().unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'2']?);
			let (k, _v) = iter.next().unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'3']?);
			iter.rewind()?;
			iter.rewind()?;
			let (k, _v) = iter.next().unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'2']?);
			let (k, _v) = iter.next().unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'3']?);

			iter.rewind()?;
			iter.rewind()?;
			iter.rewind()?;
			let (k, _v) = iter.next().unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'1']?);
			iter.rewind()?;
			assert!(iter.rewind().is_err());
			assert!(iter.next().is_none());
		}

		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}

	#[test]
	fn test_rewind_and_allocate() -> Result<(), Error> {
		let db_dir = "bin/.lmdb_rewind_and_allocate";
		let db_size = 1024 * 1024 * 100;
		let db_name = "mydb";
		make_lmdb_test_dir(db_dir)?;
		let mut db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;

			txn.put(&String::new("test3")?, &['x' as u8; 1])?;
			txn.put(&String::new("test2")?, &['y' as u8; 1])?;
			txn.put(&String::new("test1")?, &['z' as u8; 1])?;
			txn.put(&String::new("zzz")?, &['a' as u8; 1])?;
			txn.commit()?;
		}

		db.close();
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let txn = db.read()?;
			let mut iter = txn.iter(&String::new("test")?)?;

			let (k_size, v_size) = iter.next_bytes(&mut [], &mut [])?.unwrap();
			assert_eq!(k_size, 5);
			assert_eq!(v_size, 1);

			let mut k = Vec::with_capacity(k_size)?;
			let mut v = Vec::with_capacity(v_size)?;
			k.resize(k_size)?;
			v.resize(v_size)?;

			iter.rewind()?;
			let (_, _) = iter.next_bytes(k.as_mut(), v.as_mut())?.unwrap();
			assert_eq!(k.len(), 5);
			assert_eq!(k, vec![b't', b'e', b's', b't', b'1']?);
			assert_eq!(v, vec![b'z']?);

			let (k_size, v_size) = iter.next_bytes(&mut [], &mut [])?.unwrap();

			let mut k = Vec::with_capacity(k_size)?;
			let mut v = Vec::with_capacity(v_size)?;
			k.resize(k_size)?;
			v.resize(v_size)?;

			iter.rewind()?;
			let (_, _) = iter.next_bytes(k.as_mut(), v.as_mut())?.unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'2']?);
			assert_eq!(v, vec![b'y']?);

			let (k_size, v_size) = iter.next_bytes(&mut [], &mut [])?.unwrap();

			let mut k = Vec::with_capacity(k_size)?;
			let mut v = Vec::with_capacity(v_size)?;
			k.resize(k_size)?;
			v.resize(v_size)?;

			iter.rewind()?;
			let (_, _) = iter.next_bytes(k.as_mut(), v.as_mut())?.unwrap();
			assert_eq!(k, vec![b't', b'e', b's', b't', b'3']?);
			assert_eq!(v, vec![b'x']?);

			assert_eq!(iter.next_bytes(&mut [], &mut [])?, None);
		}

		remove_lmdb_test_dir(db_dir)?;
		Ok(())
	}

	#[test]
	fn test_bad_txn_scenarios() -> Result<(), Error> {
		let db_dir = "bin/.lmdb_bad_txn";
		let db_size = 1024 * 1024;
		let db_name = "mydb";
		make_lmdb_test_dir(db_dir)?;
		let db = Lmdb::new(db_dir, db_name, db_size)?;

		{
			let mut txn = db.write()?;
			assert!(txn.put(&[b'a'], &[0u8; 1024 * 1024 * 10]).is_err());
			assert!(txn.commit().is_err());
		}

		remove_lmdb_test_dir(db_dir)?;

		Ok(())
	}
}
