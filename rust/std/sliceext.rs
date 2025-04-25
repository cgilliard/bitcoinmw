use prelude::*;
use std::misc::slice_copy;

pub trait SliceExt<T: Copy> {
	fn slice_copy(&mut self, src: &[T]) -> Result<(), Error>;
}

impl<T: Copy> SliceExt<T> for [T] {
	fn slice_copy(&mut self, src: &[T]) -> Result<(), Error> {
		if self.len() != src.len() {
			return Err(Error::new(OutOfBounds));
		}
		slice_copy(src, self, src.len())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_slice_copy() -> Result<(), Error> {
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

		Ok(())
	}
}
