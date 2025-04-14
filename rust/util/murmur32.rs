// Copyright (c) 2020 Stu Small
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

use core::cmp::min;
use core::mem::size_of;
use core::slice::from_raw_parts;
use std::misc::subslice;

pub const MURMUR_SEED: u32 = 0x31337;

const C1: u32 = 0x85eb_ca6b;
const C2: u32 = 0xc2b2_ae35;
const R1: u32 = 16;
const R2: u32 = 13;
const M: u32 = 5;
const N: u32 = 0xe654_6b64;

pub fn murmur3_32_of_u64(source: u64, seed: u32) -> u32 {
	let slice = unsafe { from_raw_parts(&source as *const u64 as *const u8, size_of::<u64>()) };
	murmur3_32_of_slice(slice, seed)
}

pub fn murmur3_32_of_slice(source: &[u8], seed: u32) -> u32 {
	let mut buffer = source;
	let mut processed = 0;
	let mut state = seed;
	loop {
		match min(buffer.len(), 4) {
			0 => return finish(state, processed),
			1 => {
				processed += 1;
				let k: u32 = buffer[0] as u32;
				state ^= calc_k(k);
				return finish(state, processed);
			}
			2 => {
				processed += 2;
				let k: u32 = ((buffer[1] as u32) << 8) | (buffer[0] as u32);
				state ^= calc_k(k);
				return finish(state, processed);
			}
			3 => {
				processed += 3;
				let k: u32 =
					((buffer[2] as u32) << 16) | ((buffer[1] as u32) << 8) | (buffer[0] as u32);
				state ^= calc_k(k);
				return finish(state, processed);
			}
			4 => {
				processed += 4;
				let k: u32 = ((buffer[3] as u32) << 24)
					| ((buffer[2] as u32) << 16)
					| ((buffer[1] as u32) << 8)
					| (buffer[0] as u32);
				state ^= calc_k(k);
				state = state.rotate_left(R2);
				state = (state.wrapping_mul(M)).wrapping_add(N);
				// SAFETY: unwrap ok because we know processed is '4' so we have
				// sufficient bytes in the slice
				buffer = subslice(buffer, 4, buffer.len() - 4).unwrap();
			}
			_ => {}
		};
	}
}

fn finish(state: u32, processed: u32) -> u32 {
	let mut hash = state;
	hash ^= processed;
	hash ^= hash.wrapping_shr(R1);
	hash = hash.wrapping_mul(C1);
	hash ^= hash.wrapping_shr(R2);
	hash = hash.wrapping_mul(C2);
	hash ^= hash.wrapping_shr(R1);
	hash
}

fn calc_k(k: u32) -> u32 {
	const C1: u32 = 0xcc9e_2d51;
	const C2: u32 = 0x1b87_3593;
	const R1: u32 = 15;
	k.wrapping_mul(C1).rotate_left(R1).wrapping_mul(C2)
}
