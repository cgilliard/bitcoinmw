use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;

pub trait StrExt {
	fn findn(&self, s: &str, offset: usize) -> Option<usize>;
	fn rfindn(&self, s: &str, offset: usize) -> Option<usize>;
}

impl StrExt for str {
	fn findn(&self, s: &str, offset: usize) -> Option<usize> {
		if offset > self.len() || self.len() < s.len() {
			return None;
		}
		// Use your findn logic or a simple search
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
		let self_str_len = self.len();
		let s_len = s.len();
		if s_len == 0 {
			if offset < self_str_len {
				return Some(offset);
			} else {
				return Some(self_str_len);
			}
		}

		let offset = if offset < self_str_len {
			offset
		} else {
			self_str_len
		};

		let search_len = offset + 1;
		if search_len < s_len {
			return None;
		}

		let max_start = offset - (s_len - 1);
		let mut current_start = max_start;
		while current_start <= max_start {
			let x = unsafe { self.as_ptr().add(current_start) };
			let slice = unsafe { from_raw_parts(x, s_len) };
			let v = unsafe { from_utf8_unchecked(slice) };
			if v == s {
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
