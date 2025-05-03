use core::result::Result as CoreResult;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;
use std::ffi::{cstring_len, release};

#[derive(Clone)]
pub struct Error {
	code: u128,
	display: fn() -> &'static str,
	bt: Backtrace,
}

impl PartialEq for Error {
	fn eq(&self, other: &Error) -> bool {
		self.code == other.code
	}
}

impl Debug for Error {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> CoreResult<(), FmtError> {
		#[cfg(not(rustc))]
		write!(
			_f,
			"{:?}",
			format!("{}", self).map_err(|_| { FmtError::default() })?
		)?;

		Ok(())
	}
}

impl Display for Error {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		let kind_str = (self.display)();
		unsafe {
			let bt = self.bt.as_ptr();
			let s = if bt.is_null() {
				"Backtrace disabled. To enable export RUST_BACKTRACE=1."
			} else {
				let len = cstring_len(bt);
				let slice = from_raw_parts(bt, len as usize);
				from_utf8_unchecked(slice)
			};

			release(bt);
			writef!(f, "ErrorKind={}\n{}", kind_str, s)?;
		}
		Ok(())
	}
}

impl Error {
	pub const fn new(code: u128, display: fn() -> &'static str, bt: Backtrace) -> Self {
		Self { code, display, bt }
	}
	pub fn code(&self) -> u128 {
		self.code
	}
	pub fn display(&self) -> &'static str {
		(self.display)()
	}
	pub fn set_bt(&mut self, bt: Backtrace) {
		self.bt = bt;
	}
}

#[cfg(test)]
mod test {
	use super::*;

	errors!(
		xIO,
		xIllegalArgument,
		xOutOfBounds,
		xIllegalState,
		xOutOfMemory,
		xOperationFailed
	);

	fn try_errors(x: u32) -> Result<()> {
		if x == 1 {
			err!(xOutOfBounds)
		} else if x == 2 {
			err!(xOperationFailed)
		} else if x == 3 {
			err!(xOutOfMemory)
		} else if x == 4 {
			err!(xIO)
		} else if x == 5 {
			err!(xIllegalArgument)
		} else if x == 6 {
			err!(xIllegalState)
		} else {
			Ok(())
		}
	}

	#[test]
	fn test_error_ret() -> Result<()> {
		assert_eq!(try_errors(1), err!(xOutOfBounds));
		assert_eq!(try_errors(2), err!(xOperationFailed));
		assert_eq!(try_errors(3), err!(xOutOfMemory));
		assert_ne!(try_errors(3), err!(xOutOfBounds));

		match try_errors(4) {
			Ok(_) => assert!(false),
			Err(e) => assert_eq!(e, xIO),
		}
		Ok(())
	}
}
