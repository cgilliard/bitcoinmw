// Copyright (c) 2020 Stu Small
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

use core::cmp::min;
use core::mem::size_of;
use core::ops::Shl;
use core::slice::from_raw_parts;
use std::misc::from_le_bytes_u64;

pub fn murmur3_128_of_u64(source: u64, seed: u32) -> u128 {
	let slice = unsafe { from_raw_parts(&source as *const u64 as *const u8, size_of::<u64>()) };
	murmur3_x64_128_of_slice(slice, seed)
}

pub fn murmur3_x64_128_of_slice(source: &[u8], seed: u32) -> u128 {
	const C1: u64 = 0x87c3_7b91_1142_53d5;
	const C2: u64 = 0x4cf5_ad43_2745_937f;
	const C3: u64 = 0x52dc_e729;
	const C4: u64 = 0x3849_5ab5;
	const R1: u32 = 27;
	const R2: u32 = 31;
	const R3: u32 = 33;
	const M: u64 = 5;
	let mut h1: u64 = seed as u64;
	let mut h2: u64 = seed as u64;
	let mut buf = source;
	let mut processed: usize = 0;
	loop {
		match min(buf.len(), 16) {
			16 => {
				processed += 16;
				let s1 = unsafe { from_raw_parts(buf.as_ptr(), 8) };
				let s2 = unsafe { from_raw_parts(buf.as_ptr().add(8), 8) };
				let k1 = from_le_bytes_u64(s1);
				let k2 = from_le_bytes_u64(s2);
				h1 ^= k1.wrapping_mul(C1).rotate_left(R2).wrapping_mul(C2);
				h1 = h1
					.rotate_left(R1)
					.wrapping_add(h2)
					.wrapping_mul(M)
					.wrapping_add(C3);
				h2 ^= k2.wrapping_mul(C2).rotate_left(R3).wrapping_mul(C1);
				h2 = h2
					.rotate_left(R2)
					.wrapping_add(h1)
					.wrapping_mul(M)
					.wrapping_add(C4);

				buf = unsafe {
					let ptr = buf.as_ptr().add(16);
					let len = buf.len() - 16;
					from_raw_parts(ptr, len)
				}
			}
			0 => {
				h1 ^= processed as u64;
				h2 ^= processed as u64;
				h1 = h1.wrapping_add(h2);
				h2 = h2.wrapping_add(h1);
				h1 = fmix64(h1);
				h2 = fmix64(h2);
				h1 = h1.wrapping_add(h2);
				h2 = h2.wrapping_add(h1);
				return ((h2 as u128) << 64) | (h1 as u128);
			}
			_ => {
				let read = buf.len();
				processed += read;

				let mut k1 = 0;
				let mut k2 = 0;
				if read >= 15 {
					k2 ^= (buf[14] as u64).shl(48u64);
				}
				if read >= 14 {
					k2 ^= (buf[13] as u64).shl(40u64);
				}
				if read >= 13 {
					k2 ^= (buf[12] as u64).shl(32u64);
				}
				if read >= 12 {
					k2 ^= (buf[11] as u64).shl(24u64);
				}
				if read >= 11 {
					k2 ^= (buf[10] as u64).shl(16u64);
				}
				if read >= 10 {
					k2 ^= (buf[9] as u64).shl(8u64);
				}
				if read >= 9 {
					k2 ^= buf[8] as u64;
					k2 = k2.wrapping_mul(C2).rotate_left(33u32).wrapping_mul(C1);
					h2 ^= k2;
				}
				if read >= 8 {
					k1 ^= (buf[7] as u64).shl(56);
				}
				if read >= 7 {
					k1 ^= (buf[6] as u64).shl(48);
				}
				if read >= 6 {
					k1 ^= (buf[5] as u64).shl(40);
				}
				if read >= 5 {
					k1 ^= (buf[4] as u64).shl(32);
				}
				if read >= 4 {
					k1 ^= (buf[3] as u64).shl(24);
				}
				if read >= 3 {
					k1 ^= (buf[2] as u64).shl(16);
				}
				if read >= 2 {
					k1 ^= (buf[1] as u64).shl(8);
				}
				if read >= 1 {
					k1 ^= buf[0] as u64;
				}

				k1 = k1.wrapping_mul(C1);
				k1 = k1.rotate_left(31);
				k1 = k1.wrapping_mul(C2);
				h1 ^= k1;

				// handle checks for mrustc by checking read < len.
				// this should always be true
				let len = buf.len();
				if read < len {
					buf = &buf[read..len];
				}
			}
		}
	}
}

fn fmix64(k: u64) -> u64 {
	const C1: u64 = 0xff51_afd7_ed55_8ccd;
	const C2: u64 = 0xc4ce_b9fe_1a85_ec53;
	const R: u32 = 33;
	let mut tmp = k;
	tmp ^= tmp >> R;
	tmp = tmp.wrapping_mul(C1);
	tmp ^= tmp >> R;
	tmp = tmp.wrapping_mul(C2);
	tmp ^= tmp >> R;
	tmp
}
