use core::ops::{Index, IndexMut};
use core::ptr::{copy_nonoverlapping, null};
use core::slice::from_raw_parts;
use prelude::*;
use std::ffi::{alloc, release};

pub struct CStr {
	ptr: *const u8,
	leak: bool,
}

impl Drop for CStr {
	fn drop(&mut self) {
		unsafe {
			if !self.leak && !self.ptr.is_null() {
				release(self.ptr);
				self.ptr = null();
			}
		}
	}
}

impl Index<usize> for CStr {
	type Output = u8;

	fn index(&self, index: usize) -> &Self::Output {
		unsafe { &*self.ptr.offset(index as isize) }
	}
}

impl IndexMut<usize> for CStr {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		unsafe { &mut *(self.ptr.offset(index as isize) as *mut u8) }
	}
}

impl CStr {
	pub fn new(s: &str) -> Result<Self, Error> {
		let len = s.len();
		unsafe {
			let ptr = alloc(len + 1) as *mut u8;
			if ptr.is_null() {
				return Err(Error::new(Alloc));
			}
			copy_nonoverlapping(s.as_ptr(), ptr, len);
			*ptr.add(len) = 0u8;
			Ok(Self { ptr, leak: false })
		}
	}

	pub fn from_ptr(ptr: *const u8, leak: bool) -> Self {
		Self { ptr, leak }
	}

	/*
	pub fn as_str(&self) -> Result<String, Error> {
		unsafe { String::newb(from_raw_parts(self.ptr, self.len())) }
	}
		*/

	pub fn len(&self) -> usize {
		let mut len = 0;
		unsafe {
			let mut current = self.ptr;
			while *current != 0 {
				len += 1;
				current = current.add(1);
				// Safety: Assume ptr is valid and null-terminated (from new)
				// No bound check; trust alloc(len + 1) in new
			}
		}
		len
	}

	/*
	pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
		let len = self.len();
		let mut r = Vec::with_capacity(len)?;
		unsafe {
			copy_nonoverlapping(self.ptr, r.as_mut_ptr(), len);
			r.set_len(len);
		}
		Ok(r)
	}
		*/

	pub fn as_ptr(&self) -> *const u8 {
		self.ptr
	}

	pub fn as_mut_ptr(&mut self) -> *mut u8 {
		self.ptr as *mut u8
	}

	pub fn copy_to_slice(&self, slice: &mut [u8], offset: usize) -> Result<(), Error> {
		let len = self.len();
		if offset + slice.len() > len {
			Err(Error::new(ArrayIndexOutOfBounds))
		} else {
			unsafe {
				copy_nonoverlapping(self.ptr.offset(offset as isize), slice.as_mut_ptr(), len);
			}
			Ok(())
		}
	}
}

/*
#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn test_cstr() -> Result<(), Error> {
		let c1 = CStr::new("mystr")?;
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

		Ok(())
	}
}
*/
