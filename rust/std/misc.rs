#![allow(dead_code)]

use core::ptr::{copy_nonoverlapping, null};
use core::slice::from_raw_parts;
use prelude::*;
use std::ffi::{alloc, release};

pub fn subslice<N>(n: &[N], off: usize, len: usize) -> Result<&[N], Error> {
	if len + off > n.len() {
		Err(Error::new(ArrayIndexOutOfBounds))
	} else {
		Ok(unsafe { from_raw_parts(n.as_ptr().add(off), len) })
	}
}

pub fn to_be_bytes_u64(value: u64, bytes: &mut [u8]) {
	if bytes.len() >= 8 {
		bytes[0] = (value >> 56) as u8;
		bytes[1] = (value >> 48) as u8;
		bytes[2] = (value >> 40) as u8;
		bytes[3] = (value >> 32) as u8;
		bytes[4] = (value >> 24) as u8;
		bytes[5] = (value >> 16) as u8;
		bytes[6] = (value >> 8) as u8;
		bytes[7] = value as u8;
	}
}

pub fn to_be_bytes_u32(value: u32, bytes: &mut [u8]) {
	if bytes.len() >= 4 {
		bytes[0] = (value >> 24) as u8;
		bytes[1] = (value >> 16) as u8;
		bytes[2] = (value >> 8) as u8;
		bytes[3] = value as u8;
	}
}

pub fn to_be_bytes_u16(value: u16, bytes: &mut [u8]) {
	if bytes.len() >= 2 {
		bytes[0] = (value >> 8) as u8;
		bytes[1] = value as u8;
	}
}

pub fn to_be_bytes_u128(value: u128, bytes: &mut [u8]) {
	if bytes.len() >= 16 {
		bytes[0] = (value >> 120) as u8;
		bytes[1] = (value >> 112) as u8;
		bytes[2] = (value >> 104) as u8;
		bytes[3] = (value >> 96) as u8;
		bytes[4] = (value >> 88) as u8;
		bytes[5] = (value >> 80) as u8;
		bytes[6] = (value >> 72) as u8;
		bytes[7] = (value >> 64) as u8;
		bytes[8] = (value >> 56) as u8;
		bytes[9] = (value >> 48) as u8;
		bytes[10] = (value >> 40) as u8;
		bytes[11] = (value >> 32) as u8;
		bytes[12] = (value >> 24) as u8;
		bytes[13] = (value >> 16) as u8;
		bytes[14] = (value >> 8) as u8;
		bytes[15] = value as u8;
	}
}

pub fn from_be_bytes_u64(bytes: &[u8]) -> u64 {
	if bytes.len() >= 8 {
		((bytes[0] as u64) << 56)
			| ((bytes[1] as u64) << 48)
			| ((bytes[2] as u64) << 40)
			| ((bytes[3] as u64) << 32)
			| ((bytes[4] as u64) << 24)
			| ((bytes[5] as u64) << 16)
			| ((bytes[6] as u64) << 8)
			| (bytes[7] as u64)
	} else {
		0
	}
}

pub fn from_be_bytes_u16(bytes: &[u8]) -> u16 {
	if bytes.len() >= 2 {
		((bytes[0] as u16) << 8) | (bytes[1] as u16)
	} else {
		0
	}
}

pub fn to_le_bytes_u64(value: u64, bytes: &mut [u8]) {
	if bytes.len() >= 8 {
		bytes[0] = value as u8;
		bytes[1] = (value >> 8) as u8;
		bytes[2] = (value >> 16) as u8;
		bytes[3] = (value >> 24) as u8;
		bytes[4] = (value >> 32) as u8;
		bytes[5] = (value >> 40) as u8;
		bytes[6] = (value >> 48) as u8;
		bytes[7] = (value >> 56) as u8;
	}
}

pub fn to_le_bytes_u16(value: u16, bytes: &mut [u8]) {
	if bytes.len() >= 2 {
		bytes[0] = value as u8;
		bytes[1] = (value >> 8) as u8;
	}
}

pub fn from_le_bytes_u64(bytes: &[u8]) -> u64 {
	if bytes.len() >= 8 {
		(bytes[0] as u64)
			| ((bytes[1] as u64) << 8)
			| ((bytes[2] as u64) << 16)
			| ((bytes[3] as u64) << 24)
			| ((bytes[4] as u64) << 32)
			| ((bytes[5] as u64) << 40)
			| ((bytes[6] as u64) << 48)
			| ((bytes[7] as u64) << 56)
	} else {
		0
	}
}

pub fn from_le_bytes_u16(bytes: &[u8]) -> u16 {
	if bytes.len() >= 2 {
		(bytes[0] as u16) | ((bytes[1] as u16) << 8)
	} else {
		0
	}
}

pub fn to_le_bytes_u32(value: u32, bytes: &mut [u8]) {
	if bytes.len() >= 4 {
		bytes[0] = value as u8;
		bytes[1] = (value >> 8) as u8;
		bytes[2] = (value >> 16) as u8;
		bytes[3] = (value >> 24) as u8;
	}
}

pub fn from_le_bytes_u32(bytes: &[u8]) -> u32 {
	if bytes.len() >= 4 {
		(bytes[0] as u32)
			| ((bytes[1] as u32) << 8)
			| ((bytes[2] as u32) << 16)
			| ((bytes[3] as u32) << 24)
	} else {
		0
	}
}

pub fn from_be_bytes_u32(bytes: &[u8]) -> u32 {
	if bytes.len() >= 4 {
		((bytes[0] as u32) << 24)
			| ((bytes[1] as u32) << 16)
			| ((bytes[2] as u32) << 8)
			| (bytes[3] as u32)
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

#[inline]
pub fn u256_less_than_or_equal(max_value: &[u8; 32], value: &[u8; 32]) -> bool {
	let mut i = 0;
	while i < 32 {
		let m = max_value[i];
		let v = value[i];
		if v < m {
			return true;
		}
		if v > m {
			return false;
		}
		i += 1;
	}
	true
}

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

	pub fn as_str(&self) -> Result<String, Error> {
		unsafe { String::newb(crate::core::slice::from_raw_parts(self.ptr, self.len())) }
	}

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

	pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
		let len = self.len();
		let mut r = Vec::with_capacity(len)?;
		unsafe {
			copy_nonoverlapping(self.ptr, r.as_mut_ptr(), len);
			r.set_len(len);
		}
		Ok(r)
	}

	pub fn as_ptr(&self) -> *const u8 {
		self.ptr
	}

	pub fn as_mut_ptr(&mut self) -> *mut u8 {
		self.ptr as *mut u8
	}
}

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

		Ok(())
	}
}
