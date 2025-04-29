use core::mem::forget;
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
		let mut inner: Box<RcInner<T>> = unsafe { Box::from_raw(ptr) };
		unsafe {
			inner.leak();
		}
		aadd!(&mut inner.count, 1);
		Rc { inner }
	}
}

impl<T: ?Sized> Drop for Rc<T> {
	fn drop(&mut self) {
		let mut rci = self.inner.as_ptr();
		if asub!(&mut rci.count, 1) == 1 {
			unsafe {
				self.inner.unleak();
			}
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

impl<T> Rc<T> {
	pub fn new(value: T) -> Result<Self> {
		match Box::new(RcInner { value, count: 1 }) {
			Ok(mut inner) => {
				unsafe {
					inner.leak();
				}
				Ok(Self { inner })
			}
			Err(e) => Err(e),
		}
	}

	pub unsafe fn from_raw(ptr: Ptr<T>) -> Self {
		Self {
			inner: Box::from_raw(Ptr::new(ptr.raw() as *const RcInner<T>)),
		}
	}

	pub unsafe fn into_raw(self) -> Ptr<T> {
		let ret = Ptr::new(self.inner.as_ptr().raw() as *const T);
		forget(self);
		ret
	}

	pub unsafe fn set_to_drop(&mut self) {
		let rci = self.inner.as_mut();
		astore!(&mut rci.count, 1);
	}
}

#[cfg(test)]
mod test {
	#![allow(unknown_lints)]
	#![allow(static_mut_refs)]
	use super::*;

	#[test]
	fn test_rc1() -> Result<()> {
		let mut x1 = Rc::new(1)?;
		*x1 += 1;
		assert_eq!(*x1, 2);
		let mut x2 = x1.clone();
		assert_eq!(*x1, 2);
		assert_eq!(*x2, 2);

		*x2 += 1;
		assert_eq!(*x1, 3);
		assert_eq!(*x2, 3);

		*x1 += 1;
		assert_eq!(*x1, 4);
		assert_eq!(*x2, 4);

		Ok(())
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
	fn test_rc2() -> Result<()> {
		{
			let x = Rc::new(MyType { v: 1 })?;
			assert_eq!((*x).v, 1);
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
		Ok(())
	}

	#[test]
	fn test_rc_raw() -> Result<()> {
		/*
		let mut rc1 = Rc::new(1)?;
		let rc2 = rc1.clone();

		assert_eq!(*rc1, 1);
		assert_eq!(*rc2, 1);

		let ptr = unsafe { rc2.into_raw() };

		let mut rc2_b = unsafe { Rc::from_raw(ptr) };

		assert_eq!(*rc2_b, 1);
		assert_eq!(*rc1, 1);

		*rc2_b += 1;
		assert_eq!(*rc2_b, 2);
		assert_eq!(*rc1, 2);

		*rc1 += 1;
		assert_eq!(*rc2_b, 3);
		assert_eq!(*rc1, 3);
			*/

		Ok(())
	}

	#[test]
	fn test_box_dyn() -> Result<()> {
		let v = 7;
		let x = Rc::new(Box::new(move |x: u32| -> u32 { v + x + 1 })?)?;
		let y = x.clone();

		assert_eq!((*x)(4), 1 + 4 + 7);
		assert_eq!((*y)(3), 1 + 3 + 7);

		Ok(())
	}
}
