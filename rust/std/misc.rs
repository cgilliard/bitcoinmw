use core::ptr::copy;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use prelude::*;

pub fn subslice<N>(n: &[N], off: usize, len: usize) -> Result<&[N], Error> {
	if off > n.len() || len.checked_add(off).map_or(true, |end| end > n.len()) {
		Err(Error::new(ArrayIndexOutOfBounds))
	} else {
		Ok(unsafe { from_raw_parts(n.as_ptr().add(off), len) })
	}
}

pub fn subslice_mut<N>(n: &mut [N], off: usize, len: usize) -> Result<&mut [N], Error> {
	if off > n.len() || len.checked_add(off).map_or(true, |end| end > n.len()) {
		Err(Error::new(ArrayIndexOutOfBounds))
	} else {
		Ok(unsafe { from_raw_parts_mut(n.as_mut_ptr().add(off), len) })
	}
}

pub fn strcmp(a: &str, b: &str) -> i32 {
	let len = if a.len() > b.len() { b.len() } else { a.len() };
	let x = a.as_bytes();
	let y = b.as_bytes();

	for i in 0..len {
		if x[i] != y[i] {
			return if x[i] > y[i] { 1 } else { -1 };
		}
	}

	if a.len() < b.len() {
		1
	} else if a.len() > b.len() {
		-1
	} else {
		0
	}
}

pub fn u128_to_str(mut n: u128, offset: usize, buf: &mut [u8], base: u8) -> usize {
	let buf_len = buf.len();
	let mut i = buf_len - 1;

	while n > 0 {
		if i == 0 {
			break;
		}
		if i < buf_len && base != 0 {
			let digit = (n % base as u128) as u8;
			buf[i] = if digit < 10 {
				b'0' + digit
			} else {
				b'a' + (digit - 10)
			};
		}
		if base != 0 {
			n /= base as u128;
		}
		i -= 1;
	}

	let mut len = buf_len - i - 1;

	if len == 0 && buf_len > 0 && offset < buf_len {
		buf[offset] = b'0';
		len = 1;
	} else {
		let mut k = 0;
		for j in i + 1..buf_len {
			if k + offset < buf_len {
				buf[k + offset] = buf[j];
			}
			k += 1;
		}
	}

	len
}

pub fn i128_to_str(mut n: i128, buf: &mut [u8], base: u8) -> usize {
	if n < 0 {
		n *= -1;
		if buf.len() < 2 {
			0
		} else {
			buf[0] = b'-';
			u128_to_str(n as u128, 1, buf, base) + 1
		}
	} else {
		u128_to_str(n as u128, 0, buf, base)
	}
}

pub fn array_copy<T: Copy>(src: &[T], dst: &mut [T], len: usize) -> Result<(), Error> {
	if dst.len() < len || src.len() < len {
		Err(Error::new(ArrayIndexOutOfBounds))
	} else {
		unsafe { copy(src.as_ptr(), dst.as_mut_ptr(), len) }
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Copy, Clone)]
	struct Copyable {
		x: u32,
		y: u64,
		z: i32,
	}

	#[test]
	fn test_array_copy() -> Result<(), Error> {
		let mut arr = vec![Copyable { x: 1, y: 2, z: -1 }]?;
		arr.push(Copyable { x: 3, y: 4, z: -2 })?;
		arr.push(Copyable { x: 7, y: 7, z: -3 })?;

		let mut n = Vec::new();
		n.resize(3)?;
		array_copy(arr.slice(0, 3), n.mut_slice(0, 3), 3)?;
		assert_eq!(n[0].x, 1);
		assert_eq!(n[1].x, 3);
		assert_eq!(n[2].x, 7);
		assert_eq!(n[2].y, 7);
		assert_eq!(n[2].z, -3);
		assert_eq!(n.len(), 3);

		let v = vec![0u64, 1u64, 2u64]?;
		let mut v2 = vec![9u64, 9u64, 9u64]?;
		array_copy(v.slice(0, 3), v2.mut_slice(0, 3), 3)?;

		assert_eq!(v2[0], 0);
		assert_eq!(v2[1], 1);
		assert_eq!(v2[2], 2);
		assert_eq!(v2.len(), 3);
		assert_eq!(v[0], 0);
		assert_eq!(v[1], 1);
		assert_eq!(v[2], 2);
		assert_eq!(v.len(), 3);
		Ok(())
	}
}
