use core::convert::AsRef;
use core::ops::{Index, IndexMut};
use core::ptr::null_mut;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use ffi::{alloc, release};
use misc::{slice_copy, subslice};
use prelude::*;

pub struct CString {
	ptr: *mut u8,
	leak: bool,
}

impl Drop for CString {
	fn drop(&mut self) {
		if !self.ptr.is_null() && !self.leak {
			unsafe { release(self.ptr) };
			self.ptr = null_mut();
		}
	}
}

impl Index<usize> for CString {
	type Output = u8;

	fn index(&self, index: usize) -> &Self::Output {
		let len = self.len();
		if index >= len {
			exit!("index out of bounds for CString {} >= {}", index, len);
		}
		unsafe { &*self.ptr.offset(index as isize) }
	}
}

impl IndexMut<usize> for CString {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		unsafe { &mut *(self.ptr.offset(index as isize) as *mut u8) }
	}
}

impl AsRef<[u8]> for CString {
	fn as_ref(&self) -> &[u8] {
		unsafe { from_raw_parts(self.ptr, self.len()) }
	}
}

impl CString {
	pub fn new(s: &str) -> Result<Self> {
		let len = s.len();
		unsafe {
			let ptr = alloc(len + 1) as *mut u8;
			if ptr.is_null() {
				return err!(Alloc);
			}
			slice_copy(s.as_ref(), from_raw_parts_mut(ptr, len + 1), len)?;
			*ptr.add(len) = 0u8;
			Ok(Self { ptr, leak: false })
		}
	}

	pub fn from_slice(bytes: &[u8]) -> Result<Self> {
		let len = bytes.len();
		unsafe {
			let ptr = alloc(len + 1) as *mut u8;
			if ptr.is_null() {
				return err!(Alloc);
			}
			slice_copy(bytes, from_raw_parts_mut(ptr, len + 1), len)?;
			*ptr.add(len) = 0u8;
			Ok(Self { ptr, leak: false })
		}
	}

	pub fn from_ptr(ptr: *const u8, leak: bool) -> Self {
		Self {
			ptr: ptr as *mut u8,
			leak,
		}
	}

	pub fn as_str(&self) -> Result<String> {
		String::newb(self.as_ref())
	}

	pub fn len(&self) -> usize {
		let mut len = 0;
		while unsafe { *self.ptr.offset(len as isize) } != 0 {
			len += 1;
		}
		len
	}

	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		let len = self.len();
		let mut r = Vec::with_capacity(len)?;
		r.resize(len)?;
		r[0..len].slice_copy(self.as_ref())?;
		Ok(r)
	}

	pub fn as_ptr(&self) -> *const u8 {
		self.ptr
	}

	pub fn as_mut_ptr(&mut self) -> *mut u8 {
		self.ptr
	}

	pub fn copy_to_slice(&self, slice: &mut [u8], offset: usize) -> Result<()> {
		let len = self.len();
		if offset + slice.len() > len {
			err!(OutOfBounds)
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
	fn test_cstr() -> Result<()> {
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
		assert_eq!(c1.len(), 5);

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

		let x1 = unsafe { alloc(100) };
		let _c1 = CString::from_ptr(x1 as *const u8, true);
		let _c2 = CString::from_ptr(x1 as *const u8, false);

		Ok(())
	}

	#[test]
	fn test_as_bytes() -> Result<()> {
		let str = CString::new("abc")?;
		assert_eq!(str.as_bytes()?, vec![b'a', b'b', b'c']?);

		Ok(())
	}
}
