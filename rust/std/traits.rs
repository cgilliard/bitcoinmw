use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;
use std::misc::{slice_copy, subslice, subslice_mut};

pub trait TryClone {
	fn try_clone(&self) -> Result<Self>
	where
		Self: Sized;
}

impl<T: Clone> TryClone for T {
	fn try_clone(&self) -> Result<Self> {
		Ok(self.clone())
	}
}

pub trait AsRaw<T: ?Sized> {
	fn as_ptr(&self) -> *const T;
}

pub trait AsRawMut<T: ?Sized> {
	fn as_mut_ptr(&mut self) -> *mut T;
}

pub trait Display {
	fn format(&self, f: &mut Formatter) -> Result<()>;
}

pub trait StrExt {
	fn findn(&self, s: &str, offset: usize) -> Option<usize>;
	fn rfindn(&self, s: &str, offset: usize) -> Option<usize>;
}

impl StrExt for str {
	fn findn(&self, s: &str, offset: usize) -> Option<usize> {
		if offset > self.len() || self.len() < s.len() {
			return None;
		}
		if s.len() == 0 {
			return Some(offset);
		}
		let mut i = offset;
		while i <= self.len() - s.len() {
			let slice = unsafe {
				let ptr = self.as_ptr().add(i);
				from_utf8_unchecked(from_raw_parts(ptr, s.len()))
			};
			if slice == s {
				return Some(i);
			}
			i += 1;
		}
		None
	}

	fn rfindn(&self, s: &str, offset: usize) -> Option<usize> {
		let self_len = self.len();
		if offset > self_len || self_len < s.len() {
			return None;
		}
		if s.len() == 0 {
			return Some(offset);
		}

		if offset < s.len() - 1 {
			return None;
		}

		let max_start = offset - (s.len() - 1);
		let mut current_start = max_start;
		loop {
			let slice = unsafe {
				let ptr = self.as_ptr().add(current_start);
				let mut nlen = s.len();
				if nlen + current_start > self.len() {
					nlen = self.len();
				}
				from_utf8_unchecked(from_raw_parts(ptr, nlen))
			};
			if slice == s {
				return Some(current_start);
			}
			if current_start == 0 {
				break;
			}
			current_start -= 1;
		}
		None
	}
}

pub trait SliceExt<T: Copy> {
	fn slice_copy(&mut self, src: &[T]) -> Result<()>;
	fn subslice(&self, offset: usize, len: usize) -> Result<&[T]>;
	fn subslice_mut(&mut self, off: usize, len: usize) -> Result<&mut [T]>;
}

impl<T: Copy> SliceExt<T> for [T] {
	fn slice_copy(&mut self, src: &[T]) -> Result<()> {
		if self.len() != src.len() {
			return err!(OutOfBounds);
		}
		slice_copy(src, self, src.len())
	}

	fn subslice(&self, offset: usize, len: usize) -> Result<&[T]> {
		subslice(self, offset, len)
	}

	fn subslice_mut(&mut self, offset: usize, len: usize) -> Result<&mut [T]> {
		subslice_mut(self, offset, len)
	}
}
