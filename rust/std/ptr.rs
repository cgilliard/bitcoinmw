use core::mem::size_of;
use core::ops::{Deref, DerefMut};
use core::ptr::{null, write};
use core::str::from_utf8_unchecked;
use prelude::*;
use std::ffi::{alloc, ptr_add, release, resize};

pub struct Ptr<T: ?Sized> {
	ptr: *const T,
}

impl<T: ?Sized> Display for Ptr<T> {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		let bytes = pointer_to_bytes(self.ptr as *const u8);
		let bytes = bytes_to_hex_8(&bytes);
		let bstr = unsafe { from_utf8_unchecked(&bytes) };
		writef!(f, "0x{}", bstr)
	}
}

impl<T: ?Sized> PartialEq for Ptr<T> {
	fn eq(&self, other: &Self) -> bool {
		self.raw() as *const u8 as usize == other.raw() as *const u8 as usize
	}
}

impl<T: ?Sized> Clone for Ptr<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T: ?Sized> Copy for Ptr<T> {}

impl<T: ?Sized> Deref for Ptr<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		unsafe { &*self.raw() }
	}
}

impl<T> DerefMut for Ptr<T>
where
	T: ?Sized,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.raw() }
	}
}

impl<T: ?Sized> Ptr<T> {
	pub fn new(ptr: *const T) -> Self {
		Self { ptr }
	}

	pub fn new_bit_set(mut ptr: *const T) -> Self {
		let tmp = (&mut ptr) as *const _ as *const *const u8;
		unsafe {
			ptr_add(tmp as *mut _, 1);
		}
		Self { ptr }
	}

	pub fn is_null(&self) -> bool {
		self.raw().is_null()
	}

	pub fn set_bit(&mut self, v: bool) {
		let ptr = (&mut self.ptr) as *const _ as *const *const u8;
		if v && (self.ptr as *const u8 as usize) % 2 == 0 {
			unsafe {
				ptr_add(ptr as *mut _, 1);
			} // Add 1 to set the bit
		} else if !v && (self.ptr as *const u8 as usize) % 2 != 0 {
			unsafe {
				ptr_add(ptr as *mut _, -1);
			} // Subtract 1 to clear the bit
		}
	}

	pub fn get_bit(&self) -> bool {
		self.ptr as *const u8 as usize % 2 != 0
	}

	pub fn raw(&self) -> *mut T {
		if self.get_bit() {
			let mut ret = self.ptr;
			unsafe {
				ptr_add(&mut ret as *mut _ as *mut u8, -1);
			}
			ret as *mut T
		} else {
			self.ptr as *mut T
		}
	}

	pub fn release(&self) {
		unsafe {
			release(self.raw() as *const u8);
		}
	}

	pub fn resize<R>(&mut self, n: usize) -> Result<Ptr<R>> {
		let ptr = unsafe { resize(self.raw() as *const u8, n) };
		if ptr.is_null() {
			err!(Alloc)
		} else {
			Ok(Ptr {
				ptr: ptr as *const R,
			})
		}
	}

	pub fn as_ref(&self) -> &T {
		unsafe { &(*self.raw()) }
	}

	pub fn as_mut(&mut self) -> &mut T {
		unsafe { &mut (*self.raw()) }
	}
}

impl<T> Ptr<T> {
	pub fn alloc(t: T) -> Result<Self> {
		let ptr = unsafe { alloc(size_of::<T>()) as *const T };

		if ptr.is_null() {
			err!(Alloc)
		} else {
			unsafe {
				write(ptr as *mut T, t);
			}
			Ok(Self { ptr })
		}
	}

	pub fn null() -> Self {
		let ptr = null();
		Self { ptr }
	}

	pub fn offt(&mut self, n: usize) -> *mut T {
		unsafe { (self.raw() as *mut u8).add(n) as *mut T }
	}

	pub fn add(&mut self, n: usize) -> *mut T {
		unsafe { (self.raw() as *mut T).add(n) }
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::mem::size_of;
	use core::ptr::write;

	#[derive(Clone)]
	struct MyBox<T: ?Sized> {
		ptr: Ptr<T>,
	}

	impl<T: ?Sized> Drop for MyBox<T> {
		fn drop(&mut self) {
			unsafe {
				release(self.ptr.raw() as *mut u8);
			}
		}
	}

	impl<T> MyBox<T> {
		fn new(t: T) -> Self {
			unsafe {
				let ptr = alloc(size_of::<T>());
				write(ptr as *mut T, t);
				let ptr = Ptr::new(ptr as *mut T);
				Self { ptr }
			}
		}

		fn as_ref(&mut self) -> &T {
			unsafe { &*(self.ptr.raw() as *mut T) }
		}

		fn get_bit(&self) -> bool {
			self.ptr.get_bit()
		}
		fn set_bit(&mut self, v: bool) {
			self.ptr.set_bit(v);
		}
	}

	#[test]
	fn test_pointer() {
		let mut b = MyBox::new(123);
		b.set_bit(false);
		assert!(!b.get_bit());
		assert_eq!(b.as_ref(), &123);

		let mut b2 = MyBox::new(456);
		b2.set_bit(true);
		assert!(b2.get_bit());
		assert_eq!(b2.as_ref(), &456);

		let ptr = Ptr::alloc(1usize).unwrap();
		let ptr2 = Ptr::new(ptr.raw());
		let ptr3 = Ptr::alloc(2usize).unwrap();
		let ptr4 = Ptr::alloc(2usize).unwrap();

		assert!(ptr == ptr2);
		assert!(ptr != ptr3);
		assert!(ptr != ptr4);
		assert!(ptr3 != ptr4);
		ptr.release();
		ptr3.release();
		ptr4.release();
	}
}
