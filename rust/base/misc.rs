use core::ptr::copy;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use core::str::from_utf8_unchecked;
use ffi::{getmicros, sleep_millis};
use prelude::*;

pub fn sleep(ms: u64) {
	unsafe {
		sleep_millis(ms);
	}
}

pub fn micros() -> u64 {
	unsafe { getmicros() }
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

pub const fn simple_hash(s: &str, line: u32) -> u128 {
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

	((line as u128) << 64) | (hash as u128)
}

pub const fn fnvhash(bytes: &[u8]) -> u64 {
	let mut hash = 0xCBF29CE484222325_u64; // FNV-1a 64-bit offset basis
	const PRIME: u64 = 0x100000001B3; // FNV-1a 64-bit prime

	// Hash the string bytes
	let mut i = 0;
	while i < bytes.len() {
		hash = hash ^ (bytes[i] as u64);
		hash = wrapping_mul(hash, PRIME);
		i += 1;
	}

	hash
}

pub fn slice_copy<T: Copy>(src: &[T], dst: &mut [T], len: usize) -> Result<()> {
	if dst.len() < len || src.len() < len {
		err!(OutOfBounds)
	} else {
		unsafe { copy(src.as_ptr(), dst.as_mut_ptr(), len) }
		Ok(())
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

pub fn subslice<N>(n: &[N], off: usize, len: usize) -> Result<&[N]> {
	if off > n.len() || len.checked_add(off).map_or(true, |end| end > n.len()) {
		err!(OutOfBounds)
	} else {
		Ok(unsafe { from_raw_parts(n.as_ptr().add(off), len) })
	}
}

pub fn subslice_mut<N>(n: &mut [N], off: usize, len: usize) -> Result<&mut [N]> {
	if off > n.len() || len.checked_add(off).map_or(true, |end| end > n.len()) {
		err!(OutOfBounds)
	} else {
		Ok(unsafe { from_raw_parts_mut(n.as_mut_ptr().add(off), len) })
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

pub fn from_utf8(bytes: &[u8]) -> Result<&str> {
	is_utf8_valid(bytes)?;
	unsafe { Ok(from_utf8_unchecked(bytes)) }
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

pub fn quick_sort<T: Ord>(arr: &mut [T]) {
	if arr.len() <= 1 {
		return;
	}
	let pivot_idx = partition(arr);
	if pivot_idx < arr.len() {
		let (left, right) = arr.split_at_mut(pivot_idx);
		quick_sort(left);
		if 1 <= right.len() {
			quick_sort(&mut right[1..]); // Skip pivot
		}
	}
}

pub fn partition<T: Ord>(arr: &mut [T]) -> usize {
	let pivot_idx = arr.len() - 1;
	let mut i = 0;
	for j in 0..pivot_idx {
		if j < arr.len() && pivot_idx < arr.len() && arr[j] <= arr[pivot_idx] {
			if i < arr.len() && j < arr.len() {
				arr.swap(i, j);
			}
			i += 1;
		}
	}
	if i < arr.len() && pivot_idx < arr.len() {
		arr.swap(i, pivot_idx);
	}
	i
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
