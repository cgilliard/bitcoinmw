use core::ops::{Deref, DerefMut};
use prelude::*;

struct RcInner<T: ?Sized> {
	count: u64,
	value: T,
}

pub struct Rc<T: ?Sized> {
	inner: Box<RcInner<T>>,
}

impl<T: ?Sized> Clone for Rc<T> {
	fn clone(&self) -> Self {
		let ptr = self.inner.as_ptr();
		let mut inner: Box<RcInner<T>> = Box::from_raw(ptr);
		inner.leak();
		aadd!(&mut inner.count, 1);
		Rc { inner }
	}
}

impl<T: ?Sized> Drop for Rc<T> {
	fn drop(&mut self) {
		let rci = self.inner.as_mut();
		if asub!(&mut rci.count, 1) == 1 {
			self.inner.unleak();
		}
	}
}

impl<T: ?Sized> Deref for Rc<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		&self.inner.value
	}
}

impl<T: ?Sized> DerefMut for Rc<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner.value
	}
}

impl<T: ?Sized> Rc<T> {
	pub fn get(&self) -> &T {
		&self.inner.value
	}

	pub fn get_mut(&mut self) -> Option<&mut T> {
		if aload!(&mut (*self.inner).count) == 1 {
			Some(&mut self.inner.value)
		} else {
			None
		}
	}

	pub unsafe fn get_mut_unchecked(&mut self) -> &mut T {
		&mut self.inner.value
	}
}

impl<T> Rc<T> {
	pub fn new(value: T) -> Result<Self, Error> {
		match Box::new(RcInner { value, count: 1 }) {
			Ok(mut inner) => {
				inner.leak();
				Ok(Self { inner })
			}
			Err(e) => Err(e),
		}
	}
}

#[cfg(test)]
mod test {
	#![allow(unknown_lints)]
	#![allow(static_mut_refs)]
	use super::*;

	#[test]
	fn test_rc1() {
		let mut x1 = Rc::new(1).unwrap();
		assert!(x1.get_mut().is_some());
		let mut x2 = x1.clone();
		assert!(x1.get_mut().is_none());
		assert!(x2.get_mut().is_none());

		unsafe {
			*x1.get_mut_unchecked() += 1;
		}
		assert_eq!(*x1.get(), 2);
		assert_eq!(*x2.get(), 2);
	}

	static mut VTEST: usize = 0;

	struct MyType {
		v: usize,
	}

	impl Drop for MyType {
		fn drop(&mut self) {
			unsafe {
				VTEST += 1;
			}
		}
	}

	#[test]
	fn test_rc2() {
		{
			let x = Rc::new(MyType { v: 1 }).unwrap();
			assert_eq!(x.get().v, 1);
			{
				let _y = x.clone();
				let _z = MyType { v: 2 };
				unsafe {
					assert_eq!(VTEST, 0);
				}
			}
			unsafe {
				assert_eq!(VTEST, 1);
			}
		}
		unsafe {
			assert_eq!(VTEST, 2);
		}
	}
}
