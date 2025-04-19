use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;

pub trait StrExt {
	fn findn(&self, s: &str, offset: usize) -> Option<usize>;
}

impl StrExt for str {
	fn findn(&self, s: &str, offset: usize) -> Option<usize> {
		if offset > self.len() {
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
}
