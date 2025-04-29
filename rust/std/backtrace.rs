use core::ptr::{copy_nonoverlapping, null};
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;
use std::constants::*;
use std::ffi::{alloc, cstring_len, gen_backtrace, release};

pub struct Backtrace(*const u8);

impl Drop for Backtrace {
	fn drop(&mut self) {
		if !self.0.is_null() && self.0 != BACKTRACE_INIT {
			unsafe {
				release(self.0 as *const u8);
			}
			self.0 = null();
		}
	}
}

impl Clone for Backtrace {
	fn clone(&self) -> Self {
		let mut len = 0;
		unsafe {
			// require null terminated string as expected.
			while *self.0.offset(len as isize) != b'\0' {
				len += 1;
			}

			let ptr = alloc(len + 1) as *mut u8;
			if ptr.is_null() {
				Self::init()
			} else {
				copy_nonoverlapping(self.0, ptr, len);
				*(ptr.add(len)) = b'\0';
				Self(ptr)
			}
		}
	}
}

impl Backtrace {
	pub fn new() -> Self {
		unsafe { Self(gen_backtrace()) }
	}

	pub const fn init() -> Self {
		Self(BACKTRACE_INIT)
	}

	pub fn as_str(&self) -> &str {
		if self.0.is_null() {
			"Backtrace disabled. To enable, set env variable: export RUST_BACKTRACE=1."
		} else if self.0 == BACKTRACE_INIT as *const u8 {
			"backtrace not initialized. Use err!(<err>) to initialize backtrace!"
		} else {
			unsafe {
				let len = self.len();
				let slice = from_raw_parts(self.0, len);
				from_utf8_unchecked(slice)
			}
		}
	}

	pub fn len(&self) -> usize {
		unsafe { cstring_len(self.0) as usize }
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[allow(dead_code)]
	extern "C" {
		fn write(fd: i32, s: *const u8, len: usize) -> i32;
	}

	#[test]
	fn test_backtrace1() -> Result<()> {
		let _bt = Backtrace::new();
		/*
		unsafe {
			write(2, "\n".as_ptr(), 1);
			let s = _bt.as_str();
			write(2, s.as_ptr(), s.len());
			write(2, "\n".as_ptr(), 1);
		}
				*/
		Ok(())
	}
}
