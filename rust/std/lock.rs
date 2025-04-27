use core::cell::UnsafeCell;
use prelude::*;
use std::constants::{WFLAG, WREQUEST};

pub struct Lock {
	pub state: UnsafeCell<u64>,
}

struct LockBoxInner {
	lock: Lock,
}

pub struct LockBox {
	inner: Rc<LockBoxInner>,
}

pub struct LockReadGuard<'a> {
	lock: &'a Lock,
	need_unlock: bool,
}

pub struct LockWriteGuard<'a> {
	lock: &'a Lock,
	need_unlock: bool,
}

impl LockWriteGuard<'_> {
	pub fn unlock(&mut self) {
		if self.need_unlock {
			let state = unsafe { &mut *self.lock.state.get() };
			astore!(&mut *state, 0);
			self.need_unlock = false;
		}
	}
}

impl LockReadGuard<'_> {
	pub fn unlock(&mut self) {
		if self.need_unlock {
			let state = unsafe { &mut *self.lock.state.get() };
			asub!(&mut *state, 1);
			self.need_unlock = false;
		}
	}
}

impl Drop for LockWriteGuard<'_> {
	fn drop(&mut self) {
		self.unlock();
	}
}

impl Drop for LockReadGuard<'_> {
	fn drop(&mut self) {
		self.unlock();
	}
}

impl Lock {
	pub fn new() -> Self {
		Self {
			state: 0_u64.into(),
		}
	}

	pub fn read<'a>(&'a self) -> LockReadGuard<'a> {
		let state = unsafe { &mut *self.state.get() };
		loop {
			let x = aload!(state) & !(WFLAG | WREQUEST);
			let y = x + 1;
			if cas!(state, &x, y) {
				break;
			}
			sched_yield!();
		}
		LockReadGuard {
			lock: self,
			need_unlock: true,
		}
	}

	pub fn write<'a>(&'a self) -> LockWriteGuard<'a> {
		let state = unsafe { &mut *self.state.get() };

		loop {
			let x = aload!(state) & !(WFLAG | WREQUEST);
			if cas!(state, &x, x | WREQUEST) {
				break;
			}
			sched_yield!();
		}
		loop {
			let x = WREQUEST;
			if cas!(state, &x, WFLAG) {
				break;
			}
			sched_yield!();
		}
		LockWriteGuard {
			lock: self,
			need_unlock: true,
		}
	}
}

impl Clone for LockBox {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl LockBox {
	pub fn new() -> Result<Self> {
		match Rc::new(LockBoxInner { lock: lock!() }) {
			Ok(inner) => Ok(Self { inner }),
			Err(e) => Err(e),
		}
	}

	pub fn read<'a>(&'a self) -> LockReadGuard<'a> {
		self.inner.lock.read()
	}

	pub fn write<'a>(&'a self) -> LockWriteGuard<'a> {
		self.inner.lock.write()
	}
}

#[cfg(test)]
mod test {
	use super::WFLAG;
	use prelude::*;
	use std::lock::Lock;
	#[test]
	fn test_lock() {
		let x = Lock::new();
		assert_eq!(unsafe { *x.state.get() }, 0);
		{
			let _v = x.write();
			assert_eq!(unsafe { *x.state.get() }, WFLAG);
		}
		assert_eq!(unsafe { *x.state.get() }, 0);
		{
			let _v = x.write();
			assert_eq!(unsafe { *x.state.get() }, WFLAG);
		}
		{
			let _v = x.read();
			assert_eq!(unsafe { *x.state.get() }, 1);
			{
				let _v = x.read();
				assert_eq!(unsafe { *x.state.get() }, 2);
				{
					let _v = x.read();
					assert_eq!(unsafe { *x.state.get() }, 3);
				}
				assert_eq!(unsafe { *x.state.get() }, 2);
			}
			assert_eq!(unsafe { *x.state.get() }, 1);
		}
		assert_eq!(unsafe { *x.state.get() }, 0);
	}

	#[test]
	fn test_lock_box() {
		let x = lock_box!().unwrap();
		let y = x.clone();
		assert_eq!(unsafe { *x.inner.lock.state.get() }, 0);
		{
			let _v = x.write();
			assert_eq!(unsafe { *x.inner.lock.state.get() }, WFLAG);
		}
		assert_eq!(unsafe { *x.inner.lock.state.get() }, 0);
		{
			let _v = x.write();
			assert_eq!(unsafe { *y.inner.lock.state.get() }, WFLAG);
		}
		{
			let _v = x.read();
			assert_eq!(unsafe { *x.inner.lock.state.get() }, 1);
			{
				let _v = x.read();
				assert_eq!(unsafe { *x.inner.lock.state.get() }, 2);
				{
					let _v = y.read();
					assert_eq!(unsafe { *x.inner.lock.state.get() }, 3);
				}
				assert_eq!(unsafe { *y.inner.lock.state.get() }, 2);
			}
			assert_eq!(unsafe { *x.inner.lock.state.get() }, 1);
		}
		assert_eq!(unsafe { *y.inner.lock.state.get() }, 0);
	}
}
