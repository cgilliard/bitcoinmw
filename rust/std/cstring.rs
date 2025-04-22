use core::convert::AsRef;
use core::ops::{Index, IndexMut};
use core::slice::{from_raw_parts, from_raw_parts_mut};
use prelude::*;
use std::ffi::alloc;
use std::misc::{slice_copy, subslice};
use std::Ptr;

pub struct CString {
	ptr: Ptr<u8>,
}

impl Drop for CString {
	fn drop(&mut self) {
		if !self.ptr.is_null() && !self.ptr.get_bit() {
			self.ptr.release();
			self.ptr = Ptr::null();
		}
	}
}

impl Index<usize> for CString {
	type Output = u8;

	fn index(&self, index: usize) -> &Self::Output {
		unsafe { &*self.ptr.raw().offset(index as isize) }
	}
}

impl IndexMut<usize> for CString {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		unsafe { &mut *(self.ptr.raw().offset(index as isize) as *mut u8) }
	}
}

impl AsRef<[u8]> for CString {
	fn as_ref(&self) -> &[u8] {
		unsafe { from_raw_parts(self.ptr.raw(), self.len()) }
	}
}

impl CString {
	pub fn new(s: &str) -> Result<Self, Error> {
		let len = s.len();
		unsafe {
			let ptr = alloc(len + 1) as *mut u8;
			if ptr.is_null() {
				return Err(Error::new(Alloc));
			}
			slice_copy(s.as_ref(), from_raw_parts_mut(ptr, len + 1), len)?;
			*ptr.add(len) = 0u8;
			let ptr = Ptr::new(ptr);

			Ok(Self { ptr })
		}
	}

	pub fn from_slice(bytes: &[u8]) -> Result<Self, Error> {
		let len = bytes.len();
		unsafe {
			let ptr = alloc(len + 1) as *mut u8;
			if ptr.is_null() {
				return Err(Error::new(Alloc));
			}
			slice_copy(bytes, from_raw_parts_mut(ptr, len + 1), len)?;
			*ptr.add(len) = 0u8;
			let ptr = Ptr::new(ptr);

			Ok(Self { ptr })
		}
	}

	pub fn from_ptr(ptr: *const u8, leak: bool) -> Self {
		let mut ptr = Ptr::new(ptr);
		ptr.set_bit(leak);
		Self { ptr }
	}

	pub fn as_str(&self) -> Result<String, Error> {
		String::newb(self.as_ref())
	}

	pub fn len(&self) -> usize {
		let mut len = 0;
		while self[len] != 0 {
			len += 1;
		}
		len
	}

	pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
		let len = self.len();
		let mut r = Vec::with_capacity(len)?;
		r.resize(len)?;
		let mut slice = r.mut_slice(0, len);
		slice_copy(self.as_ref(), &mut slice, len)?;
		Ok(r)
	}

	pub fn as_ptr(&self) -> *const u8 {
		self.ptr.raw()
	}

	pub fn as_mut_ptr(&mut self) -> *mut u8 {
		self.ptr.raw() as *mut u8
	}

	pub fn copy_to_slice(&self, slice: &mut [u8], offset: usize) -> Result<(), Error> {
		let len = self.len();
		if offset + slice.len() > len {
			Err(Error::new(OutOfBounds))
		} else {
			let b = subslice(self.as_ref(), offset, len - offset)?;
			slice_copy(b, slice, len - offset)
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn test_cstr() -> Result<(), Error> {
		let c1 = CString::new("mystr")?;
		assert_eq!(c1.as_str()?, String::new("mystr")?);

		let ptr = c1.as_ptr();
		unsafe {
			assert_eq!(*ptr, 'm' as u8);
			assert_eq!(*ptr.offset(1), 'y' as u8);
			assert_eq!(*ptr.offset(2), 's' as u8);
			assert_eq!(*ptr.offset(3), 't' as u8);
			assert_eq!(*ptr.offset(4), 'r' as u8);
			assert_eq!(*ptr.offset(5), 0 as u8);
		}

		assert_eq!(c1[0], 'm' as u8);
		assert_eq!(c1[1], 'y' as u8);
		assert_eq!(c1[2], 's' as u8);
		assert_eq!(c1[3], 't' as u8);
		assert_eq!(c1[4], 'r' as u8);
		assert_eq!(c1[5], 0 as u8);

		let mut slice = [0u8; 5];
		c1.copy_to_slice(&mut slice, 0)?;
		assert_eq!(
			slice,
			['m' as u8, 'y' as u8, 's' as u8, 't' as u8, 'r' as u8]
		);

		assert!(c1.copy_to_slice(&mut slice, 1).is_err());
		let mut slice = [0u8; 3];
		c1.copy_to_slice(&mut slice, 2)?;
		assert_eq!(slice, ['s' as u8, 't' as u8, 'r' as u8]);

		let init = unsafe { getalloccount() };
		{
			let x1 = unsafe { alloc(100) };
			let _c1 = CString::from_ptr(x1, true);
			let _c2 = CString::from_ptr(x1, false);
		}
		assert_eq!(unsafe { getalloccount() }, init);

		Ok(())
	}

	#[test]
	fn test_as_bytes() -> Result<(), Error> {
		let str = CString::new("abc")?;
		assert_eq!(str.as_bytes()?, vec![b'a', b'b', b'c']?);
		Ok(())
	}
}
