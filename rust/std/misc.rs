use core::ptr::copy;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use prelude::*;

#[derive(Dummy)]
#[allow(dead_code)]
pub struct MyStruct {
	x: u32,
	y: u64,
}

pub const fn wrapping_mul(a: u64, b: u64) -> u64 {
	// Split a and b into high and low 32-bit parts
	let a_low = (a & 0xFFFFFFFF) as u32;
	let a_high = ((a >> 32) & 0xFFFFFFFF) as u32;
	let b_low = (b & 0xFFFFFFFF) as u32;
	let b_high = ((b >> 32) & 0xFFFFFFFF) as u32;

	// Compute partial products
	let low_low = (a_low as u64) * (b_low as u64);
	let low_high = (a_low as u64) * (b_high as u64);
	let high_low = (a_high as u64) * (b_low as u64);

	// Combine for lower 64 bits
	let low = low_low;
	let mid = low_high + high_low + (low_low >> 32);

	// Final result: lower 32 bits of low + lower 32 bits of mid
	(low & 0xFFFFFFFF) | ((mid & 0xFFFFFFFF) << 32)
}

pub const fn simple_hash(s: &str, line: u32) -> u64 {
	let mut hash = 0xCBF29CE484222325_u64; // FNV-1a 64-bit offset basis
	const PRIME: u64 = 0x100000001B3; // FNV-1a 64-bit prime

	// Hash the string bytes
	let bytes = s.as_bytes();
	let mut i = 0;
	while i < bytes.len() {
		hash = hash ^ (bytes[i] as u64);
		hash = wrapping_mul(hash, PRIME);
		i += 1;
	}

	// Hash the line number (as 4 bytes, little-endian)
	hash = hash ^ ((line & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);
	hash = hash ^ (((line >> 8) & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);
	hash = hash ^ (((line >> 16) & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);
	hash = hash ^ (((line >> 24) & 0xFF) as u64);
	hash = wrapping_mul(hash, PRIME);

	hash
}

pub fn subslice<N>(n: &[N], off: usize, len: usize) -> Result<&[N]> {
	if off > n.len() || len.checked_add(off).map_or(true, |end| end > n.len()) {
		err!(OutOfBounds)
	} else {
		Ok(unsafe { from_raw_parts(n.as_ptr().add(off), len) })
	}
}

pub fn slice_starts_with<N: PartialEq>(slice: &[N], prefix: &[N]) -> bool {
	let slice_len = slice.len();
	let prefix_len = prefix.len();
	if slice_len < prefix_len {
		false
	} else {
		for i in 0..prefix_len {
			if slice[i] != prefix[i] {
				return false;
			}
		}
		true
	}
}

pub fn subslice_mut<N>(n: &mut [N], off: usize, len: usize) -> Result<&mut [N]> {
	if off > n.len() || len.checked_add(off).map_or(true, |end| end > n.len()) {
		err!(OutOfBounds)
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

pub fn u128_as_str(mut n: u128, offset: usize, buf: &mut [u8], base: u8) -> usize {
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

pub fn i128_as_str(mut n: i128, buf: &mut [u8], base: u8) -> usize {
	if n < 0 {
		n *= -1;
		if buf.len() < 2 {
			0
		} else {
			buf[0] = b'-';
			u128_as_str(n as u128, 1, buf, base) + 1
		}
	} else {
		u128_as_str(n as u128, 0, buf, base)
	}
}

pub fn slice_copy<T: Copy>(src: &[T], dst: &mut [T], len: usize) -> Result<()> {
	if dst.len() < len || src.len() < len {
		err!(OutOfBounds)
	} else {
		unsafe { copy(src.as_ptr(), dst.as_mut_ptr(), len) }
		Ok(())
	}
}

pub fn is_utf8_valid(bytes: &[u8]) -> Result<()> {
	let mut i = 0;
	while i < bytes.len() {
		let b = bytes[i];
		let len = if b <= 0x7F {
			// 1-byte (ASCII)
			1
		} else if (b & 0xE0) == 0xC0 {
			// 2-byte
			2
		} else if (b & 0xF0) == 0xE0 {
			// 3-byte
			3
		} else if (b & 0xF8) == 0xF0 {
			// 4-byte
			4
		} else {
			// Invalid leading byte
			return err!(Utf8Error);
		};

		// Check if there are enough bytes
		if i + len > bytes.len() {
			return err!(Utf8Error);
		}
		// Check continuation bytes
		for j in 1..len {
			if i + j < bytes.len() && (bytes[i + j] & 0xC0) != 0x80 {
				return err!(Utf8Error);
			}
		}

		i += len;
	}
	Ok(())
}

pub fn from_le_bytes_u32(bytes: &[u8]) -> Result<u32> {
	if bytes.len() >= 4 {
		Ok((bytes[0] as u32)
			| ((bytes[1] as u32) << 8)
			| ((bytes[2] as u32) << 16)
			| ((bytes[3] as u32) << 24))
	} else {
		err!(IllegalArgument)
	}
}

pub fn from_le_bytes_u64(bytes: &[u8]) -> Result<u64> {
	if bytes.len() >= 8 {
		Ok((bytes[0] as u64)
			| ((bytes[1] as u64) << 8)
			| ((bytes[2] as u64) << 16)
			| ((bytes[3] as u64) << 24)
			| ((bytes[4] as u64) << 32)
			| ((bytes[5] as u64) << 40)
			| ((bytes[6] as u64) << 48)
			| ((bytes[7] as u64) << 56))
	} else {
		err!(IllegalArgument)
	}
}

fn nibble_to_hex(nibble: u8) -> u8 {
	if nibble < 10 {
		nibble + 48 // '0' = 48
	} else {
		nibble + 87 // 'a' = 97 - 10
	}
}

pub fn bytes_to_hex_33(bytes: &[u8; 33]) -> [u8; 66] {
	let mut hex = [0u8; 66];

	for i in 0..33 {
		let byte = bytes[i];
		let high = (byte >> 4) & 0x0F;
		let low = byte & 0x0F;

		hex[2 * i] = nibble_to_hex(high);
		hex[2 * i + 1] = nibble_to_hex(low);
	}

	hex
}

pub fn to_le_bytes_u64(value: u64, bytes: &mut [u8]) -> Result<()> {
	if bytes.len() == 8 {
		bytes[0] = value as u8;
		bytes[1] = (value >> 8) as u8;
		bytes[2] = (value >> 16) as u8;
		bytes[3] = (value >> 24) as u8;
		bytes[4] = (value >> 32) as u8;
		bytes[5] = (value >> 40) as u8;
		bytes[6] = (value >> 48) as u8;
		bytes[7] = (value >> 56) as u8;
		Ok(())
	} else {
		err!(IllegalArgument)
	}
}

#[inline]
pub fn to_le_bytes_u32(value: u32, bytes: &mut [u8]) -> Result<()> {
	if bytes.len() >= 4 {
		bytes[0] = value as u8;
		bytes[1] = (value >> 8) as u8;
		bytes[2] = (value >> 16) as u8;
		bytes[3] = (value >> 24) as u8;
		Ok(())
	} else {
		err!(IllegalArgument)
	}
}

pub fn bytes_to_hex_64(bytes: &[u8; 64]) -> [u8; 128] {
	let mut hex = [0u8; 128];

	for i in 0..64 {
		let byte = bytes[i];
		let high = (byte >> 4) & 0x0F;
		let low = byte & 0x0F;

		hex[2 * i] = nibble_to_hex(high);
		hex[2 * i + 1] = nibble_to_hex(low);
	}

	hex
}

pub fn to_be_bytes_u64(value: u64, bytes: &mut [u8]) -> Result<()> {
	if bytes.len() == 8 {
		bytes[0] = (value >> 56) as u8;
		bytes[1] = (value >> 48) as u8;
		bytes[2] = (value >> 40) as u8;
		bytes[3] = (value >> 32) as u8;
		bytes[4] = (value >> 24) as u8;
		bytes[5] = (value >> 16) as u8;
		bytes[6] = (value >> 8) as u8;
		bytes[7] = value as u8;
		Ok(())
	} else {
		err!(IllegalArgument)
	}
}

#[inline]
pub fn u256_less_than_or_equal(max_value: &[u8; 32], value: &[u8; 32]) -> bool {
	for i in 0..32 {
		let m = max_value[i];
		let v = value[i];
		if v < m {
			return true;
		}
		if v > m {
			return false;
		}
	}
	true
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
	fn test_slice_copy() -> Result<()> {
		let mut arr = vec![Copyable { x: 1, y: 2, z: -1 }]?;
		arr.push(Copyable { x: 3, y: 4, z: -2 })?;
		arr.push(Copyable { x: 7, y: 7, z: -3 })?;

		let mut n = Vec::new();
		n.resize(3)?;
		slice_copy(arr.slice(0, 3), n.mut_slice(0, 3), 3)?;
		assert_eq!(n[0].x, 1);
		assert_eq!(n[1].x, 3);
		assert_eq!(n[2].x, 7);
		assert_eq!(n[2].y, 7);
		assert_eq!(n[2].z, -3);
		assert_eq!(n.len(), 3);

		let v = vec![0u64, 1u64, 2u64]?;
		let mut v2 = vec![9u64, 9u64, 9u64]?;
		slice_copy(v.slice(0, 3), v2.mut_slice(0, 3), 3)?;

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
