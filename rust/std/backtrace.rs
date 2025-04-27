use core::ptr::null;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;
use std::ffi::{gen_backtrace, release};

#[derive(Clone)]
pub struct Backtrace(*const u8);

impl Drop for Backtrace {
	fn drop(&mut self) {
		if !self.0.is_null() && self.0 != 0x1 as *const u8 {
			unsafe {
				release(self.0);
			}
			self.0 = null();
		}
	}
}

impl Backtrace {
	pub fn new() -> Self {
		let ret = unsafe { gen_backtrace() };
		Self(ret)
	}

	pub const fn init() -> Self {
		Self(0x1 as *const u8)
	}

	pub fn as_str(&self) -> &str {
		if self.0.is_null() {
			""
		} else if self.0 == 0x1 as *const u8 {
			"backtrace not possible. Use err!(<err>) to enable backtrace!"
		} else {
			unsafe {
				let mut len = 0;
				// require null terminated string as expected.
				while *self.0.offset(len as isize) != b'\0' {
					len += 1;
				}
				let slice = from_raw_parts(self.0, len);
				from_utf8_unchecked(slice)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::ffi::getalloccount;

	#[test]
	fn test_backtrace1() -> Result<(), Error> {
		let init = unsafe { getalloccount() };
		{
			let _bt = Backtrace::new();
			//println!("bt='{}'", bt.as_str());
		}
		assert_eq!(unsafe { getalloccount() }, init);
		Ok(())
	}
}
