// Originally code from: rust library: https://docs.rs/crate/mur3/
//
// MIT License
//
// Copyright (c) 2021 TiKV Project
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

pub mod hash128 {
	use core::ptr;
	use core::{hash::Hasher, slice};

	const C1: u64 = 0x87c37b91114253d5;
	const C2: u64 = 0x4cf5ad432745937f;
	const C3: u64 = 0x52dce729;
	const C4: u64 = 0x38495ab5;

	/// Gets the 128-bit MurmurHash3 sum of data.
	///
	/// If you only need 64-bit result, just use the first returned value.
	/// To feed multiple byte slices, use `Hasher128` instead.
	///
	/// The function is optimized for 64 bit platform.
	pub fn murmurhash3_x64_128(bytes: &[u8], seed: u32) -> (u64, u64) {
		let nblocks = bytes.len() / 16;

		let mut h1 = seed as u64;
		let mut h2 = seed as u64;

		let mut start = bytes.as_ptr();
		for _ in 0..nblocks {
			let (k1, k2) = unsafe {
				let k1 = ptr::read_unaligned(start as *const u64);
				start = start.add(8);
				let k2 = ptr::read_unaligned(start as *const u64);
				start = start.add(8);
				(u64::from_le(k1), u64::from_le(k2))
			};
			let res = feed128(h1, h2, k1, k2);
			h1 = res.0;
			h2 = res.1;
		}

		unsafe {
			finish_tail128(
				start as *const u8,
				bytes.len() % 16,
				bytes.len() as u64,
				h1,
				h2,
			)
		}
	}

	#[inline]
	fn fmix64(mut k: u64) -> u64 {
		k ^= k >> 33;
		k = k.wrapping_mul(0xff51afd7ed558ccd);
		k ^= k >> 33;
		k = k.wrapping_mul(0xc4ceb9fe1a85ec53);
		k ^ (k >> 33)
	}

	#[inline]
	fn feed128(mut h1: u64, mut h2: u64, mut k1: u64, mut k2: u64) -> (u64, u64) {
		k1 = k1.wrapping_mul(C1);
		k1 = k1.rotate_left(31);
		k1 = k1.wrapping_mul(C2);

		h1 ^= k1;
		h1 = h1.rotate_left(27);
		h1 = h1.wrapping_add(h2);
		h1 = h1.wrapping_mul(5).wrapping_add(C3);

		k2 = k2.wrapping_mul(C2);
		k2 = k2.rotate_left(33);
		k2 = k2.wrapping_mul(C1);

		h2 ^= k2;
		h2 = h2.rotate_left(31);
		h2 = h2.wrapping_add(h1);
		h2 = h2.wrapping_mul(5).wrapping_add(C4);

		(h1, h2)
	}

	#[inline]
	unsafe fn read_u64(start: *const u8, len: usize) -> (*const u8, usize, u64) {
		if len >= 8 {
			return (
				start.add(8),
				len - 8,
				u64::from_le((start as *const u64).read_unaligned()),
			);
		}
		let res = read_partial_u64(start, len);
		(start, 0, res)
	}

	#[inline]
	unsafe fn read_partial_u64(start: *const u8, len: usize) -> u64 {
		let (off, mut res) = if len >= 4 {
			(
				4,
				u32::from_le((start as *const u32).read_unaligned()) as u64,
			)
		} else {
			(0, 0)
		};
		for i in off..len {
			res |= ((*start.add(i)) as u64) << (8 * i);
		}
		res
	}

	#[inline]
	unsafe fn finish_tail128(
		tail: *const u8,
		remain: usize,
		total: u64,
		mut h1: u64,
		mut h2: u64,
	) -> (u64, u64) {
		if remain > 0 {
			let res = read_u64(tail, remain);
			let mut k = res.2;
			k = k.wrapping_mul(C1);
			k = k.rotate_left(31);
			k = k.wrapping_mul(C2);
			h1 ^= k;

			if res.1 > 0 {
				k = read_partial_u64(res.0, res.1);
				k = k.wrapping_mul(C2);
				k = k.rotate_left(33);
				k = k.wrapping_mul(C1);
				h2 ^= k;
			}
		}

		h1 ^= total;
		h2 ^= total;
		h1 = h1.wrapping_add(h2);
		h2 = h2.wrapping_add(h1);
		h1 = fmix64(h1);
		h2 = fmix64(h2);
		h1 = h1.wrapping_add(h2);
		h2 = h2.wrapping_add(h1);
		(h1, h2)
	}

	/// A 128-bit Murmur3 hasher.
	#[repr(C)]
	pub struct Hasher128 {
		h1: u64,
		h2: u64,
		buf: [u8; 16],
		len: usize,
		consume: u64,
	}

	impl Hasher128 {
		/// Creates a hasher with given seed.
		pub fn with_seed(seed: u32) -> Hasher128 {
			Hasher128 {
				h1: seed as u64,
				h2: seed as u64,
				buf: [0; 16],
				len: 0,
				consume: 0,
			}
		}

		#[inline]
		fn feed(&mut self, k1: u64, k2: u64) {
			let (h1, h2) = feed128(self.h1, self.h2, k1, k2);

			self.h1 = h1;
			self.h2 = h2;
			self.consume += 16;
		}

		/// Gets the 128-bit hash result.
		///
		/// This function doesn't have any side effect. So calling it
		/// multiple times without feeding more data will return the
		/// same result. New data will resume calculation from last state.
		#[inline]
		pub fn finish128(&self) -> (u64, u64) {
			unsafe {
				finish_tail128(
					self.buf.as_ptr(),
					self.len,
					self.consume + self.len as u64,
					self.h1,
					self.h2,
				)
			}
		}
	}

	impl Hasher for Hasher128 {
		/// Feeds a byte slice to the hasher.
		fn write(&mut self, mut bytes: &[u8]) {
			if self.len + bytes.len() < 16 {
				unsafe {
					ptr::copy_nonoverlapping(
						bytes.as_ptr(),
						self.buf.as_mut_ptr().add(self.len),
						bytes.len(),
					);
				}
				self.len += bytes.len();
				return;
			} else if self.len != 0 {
				let (n1, n2) = unsafe {
					let cnt = 16 - self.len;
					ptr::copy_nonoverlapping(
						bytes.as_ptr(),
						self.buf.as_mut_ptr().add(self.len),
						cnt,
					);
					bytes = slice::from_raw_parts(bytes.as_ptr().add(cnt), bytes.len() - cnt);
					let n1 = ptr::read(self.buf.as_ptr() as *const u64);
					let n2 = ptr::read(self.buf.as_ptr().add(8) as *const u64);
					self.len = 0;
					(u64::from_le(n1), u64::from_le(n2))
				};
				self.feed(n1, n2);
			}
			let mut start = bytes.as_ptr();
			for _ in 0..bytes.len() / 16 {
				let (n1, n2) = unsafe {
					let n1 = ptr::read_unaligned(start as *const u64);
					start = start.add(8);
					let n2 = ptr::read_unaligned(start as *const u64);
					start = start.add(8);
					(u64::from_le(n1), u64::from_le(n2))
				};
				self.feed(n1, n2);
			}
			unsafe {
				let len = bytes.len() % 16;
				if len > 0 {
					ptr::copy_nonoverlapping(start, self.buf.as_mut_ptr(), len);
				}
				self.len = len;
			}
		}

		/// Gets the 64-bit hash value.
		///
		/// It's the same as `self.finish128().0`.
		#[inline]
		fn finish(&self) -> u64 {
			self.finish128().0
		}
	}
}

pub mod hash32 {
	use core::hash::Hasher;
	use core::{ptr, slice};

	const C1: u32 = 0xcc9e2d51;
	const C2: u32 = 0x1b873593;
	const C3: u32 = 0xe6546b64;
	const C4: u32 = 0x85ebca6b;
	const C5: u32 = 0xc2b2ae35;

	#[inline]
	fn fmix32(mut h: u32) -> u32 {
		h ^= h >> 16;
		h = h.wrapping_mul(C4);
		h ^= h >> 13;
		h = h.wrapping_mul(C5);
		h ^ (h >> 16)
	}

	#[inline]
	fn feed32(mut h: u32, mut k: u32) -> u32 {
		k = k.wrapping_mul(C1);
		k = k.rotate_left(15);
		k = k.wrapping_mul(C2);

		h ^= k;
		h = h.rotate_left(13);
		h.wrapping_mul(5).wrapping_add(C3)
	}

	#[inline]
	unsafe fn finish_tail32(mut tail: *const u8, end: *const u8, total: u64, mut h: u32) -> u32 {
		if tail != end {
			let mut k: u32 = 0;
			for i in 0..3 {
				k ^= ((*tail) as u32) << (8 * i);
				tail = tail.add(1);
				if tail == end {
					break;
				}
			}
			k = k.wrapping_mul(C1);
			k = k.rotate_left(15);
			k = k.wrapping_mul(C2);
			h ^= k;
		}
		h ^= total as u32;
		fmix32(h)
	}

	/// Gets the 32-bit MurmurHash3 sum of data.
	///
	/// To feed multiple byte slices, use `Hasher32` instead.
	pub fn murmurhash3_x86_32(bytes: &[u8], seed: u32) -> u32 {
		let nblocks = bytes.len() / 4;
		let mut h = seed;
		let mut start = bytes.as_ptr();

		for _ in 0..nblocks {
			let k = u32::from_le(unsafe { ptr::read_unaligned(start as *const u32) });
			h = feed32(h, k);
			start = unsafe { start.add(4) };
		}

		unsafe {
			finish_tail32(
				start as *const u8,
				bytes.as_ptr().add(bytes.len()),
				bytes.len() as u64,
				h,
			)
		}
	}

	/// A 32-bit Murmur3 hasher.
	#[repr(C)]
	pub struct Hasher32 {
		h: u32,
		buf: [u8; 4],
		len: usize,
		consume: u64,
	}

	impl Hasher32 {
		/// Creates a hasher with given seed.
		pub fn with_seed(seed: u32) -> Hasher32 {
			Hasher32 {
				h: seed,
				buf: [0; 4],
				len: 0,
				consume: 0,
			}
		}

		#[inline]
		fn feed(&mut self, k: u32) {
			self.h = feed32(self.h, k);
			self.consume += 4;
		}

		/// Gets the 32-bit hash result.
		///
		/// This function doesn't have any side effect. So calling it
		/// multiple times without feeding more data will return the
		/// same result. New data will resume calculation from last state.
		#[inline]
		pub fn finish32(&self) -> u32 {
			unsafe {
				finish_tail32(
					self.buf.as_ptr(),
					self.buf.as_ptr().add(self.len),
					self.consume + self.len as u64,
					self.h,
				)
			}
		}
	}

	impl Hasher for Hasher32 {
		/// Feeds a byte slice to the hasher.
		fn write(&mut self, mut bytes: &[u8]) {
			if self.len + bytes.len() < 4 {
				unsafe {
					ptr::copy_nonoverlapping(
						bytes.as_ptr(),
						self.buf.as_mut_ptr().add(self.len),
						bytes.len(),
					);
				}
				self.len += bytes.len();
				return;
			} else if self.len != 0 {
				let n = unsafe {
					let cnt = 4 - self.len;
					ptr::copy_nonoverlapping(
						bytes.as_ptr(),
						self.buf.as_mut_ptr().add(self.len),
						cnt,
					);
					bytes = slice::from_raw_parts(bytes.as_ptr().add(cnt), bytes.len() - cnt);
					let n = ptr::read(self.buf.as_ptr() as *const u32);
					self.len = 0;
					u32::from_le(n)
				};
				self.feed(n);
			}
			let mut start = bytes.as_ptr();
			for _ in 0..bytes.len() / 4 {
				let n = unsafe {
					let n = ptr::read_unaligned(start as *const u32);
					start = start.add(4);
					u32::from_le(n)
				};
				self.feed(n);
			}
			unsafe {
				let len = bytes.len() % 4;
				if len > 0 {
					ptr::copy_nonoverlapping(start, self.buf.as_mut_ptr(), len);
				}
				self.len = len;
			}
		}

		/// Gets the 64-bit hash value.
		///
		/// It's the same as `self.finish32() as u64`.
		#[inline]
		fn finish(&self) -> u64 {
			self.finish32() as u64
		}
	}
}

#[cfg(test)]
mod test {
	use core::hash::Hasher;
	use util::murmur::hash128::{murmurhash3_x64_128, Hasher128};
	use util::murmur::hash32::{murmurhash3_x86_32, Hasher32};
	const DATA: &[(u32, u32, u64, u64, &str)] = &[
		(0x00, 0x00000000, 0x0000000000000000, 0x0000000000000000, ""),
		(
			0x00,
			0x248bfa47,
			0xcbd8a7b341bd9b02,
			0x5b1e906a48ae1d19,
			"hello",
		),
		(
			0x00,
			0x149bbb7f,
			0x342fac623a5ebc8e,
			0x4cdcbc079642414d,
			"hello, world",
		),
		(
			0x00,
			0xe31e8a70,
			0xb89e5988b737affc,
			0x664fc2950231b2cb,
			"19 Jan 2038 at 3:14:07 AM",
		),
		(
			0x00,
			0xd5c48bfc,
			0xcd99481f9ee902c9,
			0x695da1a38987b6e7,
			"The quick brown fox jumps over the lazy dog.",
		),
		(0x01, 0x514e28b7, 0x4610abe56eff5cb5, 0x51622daa78f83583, ""),
		(
			0x01,
			0xbb4abcad,
			0xa78ddff5adae8d10,
			0x128900ef20900135,
			"hello",
		),
		(
			0x01,
			0x6f5cb2e9,
			0x8b95f808840725c6,
			0x1597ed5422bd493b,
			"hello, world",
		),
		(
			0x01,
			0xf50e1f30,
			0x2a929de9c8f97b2f,
			0x56a41d99af43a2db,
			"19 Jan 2038 at 3:14:07 AM",
		),
		(
			0x01,
			0x846f6a36,
			0xfb3325171f9744da,
			0xaaf8b92a5f722952,
			"The quick brown fox jumps over the lazy dog.",
		),
		(0x2a, 0x087fcd5c, 0xf02aa77dfa1b8523, 0xd1016610da11cbb9, ""),
		(
			0x2a,
			0xe2dbd2e1,
			0xc4b8b3c960af6f08,
			0x2334b875b0efbc7a,
			"hello",
		),
		(
			0x2a,
			0x7ec7c6c2,
			0xb91864d797caa956,
			0xd5d139a55afe6150,
			"hello, world",
		),
		(
			0x2a,
			0x58f745f6,
			0xfd8f19ebdc8c6b6a,
			0xd30fdc310fa08ff9,
			"19 Jan 2038 at 3:14:07 AM",
		),
		(
			0x2a,
			0xc02d1434,
			0x74f33c659cda5af7,
			0x4ec7a891caf316f0,
			"The quick brown fox jumps over the lazy dog.",
		),
	];

	#[test]
	fn test_murmur128_strings() {
		for (seed, h32, h64_1, h64_2, s) in DATA {
			let (h1, h2) = murmurhash3_x64_128(s.as_bytes(), *seed);
			assert_eq!((h1, h2), (*h64_1, *h64_2), "key: {}, seed: {:0x}", s, seed);

			let mut hasher = Hasher128::with_seed(*seed);
			hasher.write(s.as_bytes());
			assert_eq!(
				hasher.finish128(),
				(*h64_1, *h64_2),
				"key: {}, seed: {:0x}",
				s,
				seed
			);
			assert_eq!(hasher.finish(), *h64_1, "key: {}, seed: {:0x}", s, seed);

			let h = murmurhash3_x86_32(s.as_bytes(), *seed);
			assert_eq!(h, *h32, "key: {}, seed: {:0x}", s, seed);

			let mut hasher = Hasher32::with_seed(*seed);
			hasher.write(s.as_bytes());
			assert_eq!(
				hasher.finish(),
				*h32 as u64,
				"key: {}, seed: {:0x}",
				s,
				seed
			);
		}
	}
}
