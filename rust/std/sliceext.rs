use prelude::*;
use std::misc::{slice_copy, subslice, subslice_mut};

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

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_slice_copy() -> Result<()> {
		let mut x = [0u64; 4];
		let y = [1, 2, 3, 4];
		assert_eq!(x[0], 0u64);
		assert_eq!(x[1], 0u64);
		assert_eq!(x[2], 0u64);
		assert_eq!(x[3], 0u64);
		x.slice_copy(&y)?;
		assert_eq!(x[0], 1u64);
		assert_eq!(x[1], 2u64);
		assert_eq!(x[2], 3u64);
		assert_eq!(x[3], 4u64);
		let v = x.subslice(0, 2)?;
		assert_eq!(v.len(), 2);
		assert_eq!(v[0], 1);
		assert_eq!(v[1], 2);

		let mut x = [3u8; 5];
		let y = [1, 2, 3, 4, 5];
		assert_eq!(x[0], 3u8);
		assert_eq!(x[1], 3u8);
		assert_eq!(x[2], 3u8);
		assert_eq!(x[3], 3u8);
		assert_eq!(x[4], 3u8);
		x.slice_copy(&y)?;
		assert_eq!(x[0], 1u8);
		assert_eq!(x[1], 2u8);
		assert_eq!(x[2], 3u8);
		assert_eq!(x[3], 4u8);
		assert_eq!(x[4], 5u8);

		let z = x.subslice(1, 2)?;
		assert_eq!(z.len(), 2);
		assert_eq!(z[0], 2);
		assert_eq!(z[1], 3);

		let w = x.subslice_mut(1, 3)?;
		assert_eq!(w.len(), 3);
		assert_eq!(w[0], 2);
		assert_eq!(w[1], 3);
		assert_eq!(w[2], 4);
		w[2] = 5;
		assert_eq!(w[2], 5);

		Ok(())
	}
}
