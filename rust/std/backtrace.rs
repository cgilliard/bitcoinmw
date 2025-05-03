use core::ptr::{null, null_mut};
use prelude::*;
use std::constants::*;
use std::ffi::{backtrace, gen_backtrace, getenv};

#[repr(C)]
#[derive(Clone)]
pub struct Backtrace {
	entries: [*mut (); MAX_BACKTRACE_ENTRIES],
	size: i32,
}

impl Display for Backtrace {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		unsafe {
			use core::slice::from_raw_parts;
			use core::str::from_utf8_unchecked;
			use std::ffi::{cstring_len, release};

			let bt = self.as_ptr();
			let s = if bt.is_null() {
				"Backtrace disabled. To enable export RUST_BACKTRACE=1."
			} else {
				let len = cstring_len(bt);
				let slice = from_raw_parts(bt, len as usize);
				from_utf8_unchecked(slice)
			};
			release(bt);
			writef!(f, "{}", s)?;
		}
		Ok(())
	}
}

impl Backtrace {
	pub fn new() -> Self {
		let mut ret = Backtrace {
			entries: [null_mut(); MAX_BACKTRACE_ENTRIES],
			size: 0,
		};
		ret.capture();
		ret
	}

	pub const fn init() -> Self {
		Self {
			entries: [null_mut(); MAX_BACKTRACE_ENTRIES],
			size: 0,
		}
	}

	pub fn capture(&mut self) {
		let size = unsafe {
			if getenv("RUST_BACKTRACE\0".as_ptr()).is_null() {
				0
			} else {
				backtrace(self.entries.as_mut_ptr(), MAX_BACKTRACE_ENTRIES as i32)
			}
		};
		self.size = size;
	}

	pub unsafe fn as_ptr(&self) -> *const u8 {
		if self.size <= 0 {
			null()
		} else {
			unsafe { gen_backtrace(self.entries.as_ptr(), self.size) }
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::ffi::release;

	#[allow(dead_code)]
	extern "C" {
		fn write(fd: i32, s: *const u8, len: usize) -> i32;
	}

	#[test]
	fn test_backtrace() -> Result<()> {
		let mut bt = Backtrace::new();
		bt.capture();
		let ptr = unsafe { bt.as_ptr() };
		if !ptr.is_null() {
			/*
			use std::ffi::cstring_len;
			let len = unsafe { cstring_len(ptr) };
			unsafe {
				write(2, ptr, len as usize);
			}
			*/
			unsafe {
				release(ptr);
			}
		}

		Ok(())
	}
}
