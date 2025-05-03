use core::marker::Unsize;
use core::mem::size_of;
use core::ops::{CoerceUnsized, Deref, DerefMut};
use core::ptr::drop_in_place;
use core::ptr::{null_mut, write};
use prelude::*;
use std::ffi::{alloc, release};

pub struct Box<T: ?Sized> {
	ptr: Ptr<T>,
}

impl<T, U> CoerceUnsized<Box<U>> for Box<T>
where
	T: Unsize<U> + ?Sized,
	U: ?Sized,
{
}

impl<T: ?Sized + Clone> TryClone for Box<T> {
	fn try_clone(&self) -> Result<Self> {
		Box::new((*(self.ptr)).clone())
	}
}

impl<T: ?Sized> Drop for Box<T> {
	fn drop(&mut self) {
		if !self.ptr.get_bit() {
			let value_ptr = self.ptr.raw();
			if !value_ptr.is_null() {
				unsafe {
					drop_in_place(value_ptr);
					release(value_ptr as *const u8);
				}
				self.ptr.set_bit(true);
			}
		}
	}
}

impl<T: ?Sized> AsRaw<T> for Box<T>
where
	Self: Sized,
{
	fn as_ptr(&self) -> *const T {
		self.ptr.raw() as *const T
	}
}

impl<T> Deref for Box<T>
where
	T: ?Sized,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.ptr.raw() }
	}
}

impl<T> DerefMut for Box<T>
where
	T: ?Sized,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.ptr.raw() }
	}
}

impl<T> AsRef<T> for Box<T> {
	fn as_ref(&self) -> &T {
		unsafe { &*self.ptr.raw() }
	}
}

impl<T> AsMut<T> for Box<T> {
	fn as_mut(&mut self) -> &mut T {
		unsafe { &mut *self.ptr.raw() }
	}
}

impl<T> Box<T> {
	pub fn new(t: T) -> Result<Self> {
		let size = size_of::<T>();
		let ptr = if size == 0 {
			let mut ptr: Ptr<T> = Ptr::new(null_mut());
			ptr.set_bit(true);
			ptr
		} else {
			let mut ptr = unsafe {
				let rptr = alloc(size) as *mut T;
				if rptr.is_null() {
					return err!(Alloc);
				}
				write(rptr, t);
				Ptr::new(rptr)
			};
			ptr.set_bit(false);
			ptr
		};
		Ok(Box { ptr })
	}
}

impl<T: ?Sized> Box<T> {
	pub unsafe fn leak(&mut self) {
		self.ptr.set_bit(true);
	}

	pub unsafe fn unleak(&mut self) {
		self.ptr.set_bit(false);
	}

	pub unsafe fn from_raw(ptr: Ptr<T>) -> Box<T> {
		Box { ptr }
	}

	pub unsafe fn into_raw(mut self) -> Ptr<T> {
		self.leak();
		Ptr::new(self.ptr.raw())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::ops::{Fn, FnMut};

	#[test]
	fn test_box1() {
		let mut x = Box::new(4).unwrap();
		let y = x.as_ref();
		assert_eq!(*y, 4);

		let z = x.as_mut();
		*z = 10;
		assert_eq!(*z, 10);
		let a = x.try_clone().unwrap();
		let b = a.as_ref();
		assert_eq!(*b, 10);
	}

	trait GetData {
		fn get_data(&self) -> i32;
	}

	struct TestSample {
		data: i32,
	}

	impl GetData for TestSample {
		fn get_data(&self) -> i32 {
			self.data
		}
	}

	#[test]
	fn test_box2() {
		let b1: Box<TestSample> = Box::new(TestSample { data: 1 }).unwrap();
		let ptr = unsafe { b1.into_raw() };
		let b2: Box<dyn GetData> = unsafe { Box::from_raw(ptr) };
		assert_eq!(b2.get_data(), 1);

		let b3 = box_dyn!(TestSample { data: 2 }, GetData);
		assert_eq!(b3.get_data(), 2);

		let b4 = Box::new(|x| 5 + x).unwrap();
		assert_eq!(b4(5), 10);
	}

	struct BoxTest<CLOSURE>
	where
		CLOSURE: Fn(i32) -> i32,
	{
		x: Box<dyn GetData>,
		y: Box<CLOSURE>,
		z: Box<[u8]>,
	}

	struct BoxTest2<T> {
		v: Box<[T]>,
	}

	#[test]
	fn test_box3() {
		let x = BoxTest {
			x: box_dyn!(TestSample { data: 8 }, GetData),
			y: Box::new(|x| x + 4).unwrap(),
			z: box_slice!(3u8, 32),
		};
		assert_eq!(x.x.get_data(), 8);
		assert_eq!((x.y)(14), 18);
		assert_eq!(x.z[5], 3u8);

		let v: Box<[u64]> = box_slice!(1u64, 10);
		let y = BoxTest2 { v };
		assert_eq!(y.v[2], 1u64);

		let z: Box<[u64]> = box_slice!(7999u64, 10);
		assert_eq!(z[9], 7999u64);

		let z: Box<[u8]> = box_slice!(4u8, 10);
		assert_eq!(z[0], 4);

		let z: Box<[i8]> = box_slice!(-9i8, 20);
		assert_eq!(z[1], -9i8);
	}

	#[test]
	fn test_try_box_dyn() {
		let x = try_box_dyn!(TestSample { data: 89 }, GetData).unwrap();
		assert_eq!(x.get_data(), 89);

		let x: Box<[u64]> = try_box_slice!(7u64, 30).unwrap();
		assert_eq!(x[4], 7u64);

		let x: Box<[i128]> = try_box_slice!(-7i128, 30).unwrap();
		assert_eq!(x[29], -7i128);
		assert_eq!(x.len(), 30);
	}

	#[test]
	fn test_box4() {
		let mut box1 = Box::new([9u8; 992]).unwrap();
		for i in 0..992 {
			assert_eq!(9u8, box1.as_ref()[i]);
		}
		let box1_mut = box1.as_mut();
		for i in 0..992 {
			box1_mut[i] = 8;
		}
		for i in 0..992 {
			assert_eq!(8u8, box1.as_ref()[i]);
		}

		let mut box2: Box<[u8]> = box_slice!(0u8, 20000);
		for i in 0..20000 {
			box2.as_mut()[i] = 10;
		}

		for i in 0..20000 {
			assert_eq!(box2.as_ref()[i], 10);
		}
	}

	static mut COUNT: i32 = 0;

	struct DropBox {
		x: u32,
	}

	impl Drop for DropBox {
		fn drop(&mut self) {
			assert_eq!(self.x, 1);
			unsafe {
				COUNT += 1;
			}
		}
	}

	#[test]
	fn test_drop_box() {
		{
			let _big: Box<[u8]> = box_slice!(0u8, 100000);
			let _v = Box::new(DropBox { x: 1 }).unwrap();
			assert_eq!(unsafe { COUNT }, 0);
		}
		assert_eq!(unsafe { COUNT }, 1);
	}

	static mut CLONE_DROP_COUNT: i32 = 0;

	#[derive(Debug, PartialEq)]
	struct CloneBox {
		x: u32,
	}

	impl Clone for CloneBox {
		fn clone(&self) -> Self {
			Self { x: self.x }
		}
	}

	impl Drop for CloneBox {
		fn drop(&mut self) {
			assert_eq!(self.x, 10);
			unsafe {
				CLONE_DROP_COUNT += 1;
			}
		}
	}
	#[test]
	fn test_clone_box() -> Result<()> {
		{
			let x = CloneBox { x: 10 };
			let y = Box::new(x).unwrap();
			let z = y.try_clone()?;
			assert_eq!(*z, *y);
			assert_eq!(unsafe { CLONE_DROP_COUNT }, 0);
		}
		assert_eq!(unsafe { CLONE_DROP_COUNT }, 2);
		Ok(())
	}

	#[test]
	fn test_box_index() -> Result<()> {
		let mut mybox: Box<[u64]> = box_slice!(0u64, 3);
		mybox[0] = 1;
		mybox[1] = 2;
		mybox[2] = 3;
		assert_eq!(mybox[0], 1);
		assert_eq!(mybox[1], 2);
		assert_eq!(mybox[2], 3);

		assert_eq!(*Box::new(1)?, *Box::new(1)?);

		Ok(())
	}

	struct Connection {
		x: u64,
		y: u64,
	}
	type OnRecvInner = dyn FnMut(Connection, &[u8]) -> Result<u64> + 'static;
	type OnRecv = Box<OnRecvInner>;

	#[test]
	fn test_box_coerce() -> Result<()> {
		let y = || -> i32 { 1 };
		let ww: Box<dyn Fn() -> i32> = Box::new(y)?;

		let mut b: OnRecv = Box::new(|c: Connection, _b: &[u8]| -> Result<u64> {
			let _x = c.x;
			let _y = c.y;
			Ok(1)
		})?;

		assert!(b(Connection { x: 2, y: 4 }, &[b'a']).is_ok());

		assert_eq!(ww(), 1);

		let x = 1234;
		let mut b: OnRecv = Box::new(move |c: Connection, _b: &[u8]| -> Result<u64> {
			let v = x + c.x + c.y;
			Ok(v)
		})?;

		assert_eq!(b(Connection { x: 1, y: 2 }, &[])?, 1234 + 1 + 2);

		Ok(())
	}
}
