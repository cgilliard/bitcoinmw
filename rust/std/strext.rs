use prelude::*;

use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;

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
