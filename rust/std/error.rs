use prelude::*;

#[derive(Clone)]
pub struct Error {
	code: u64,
	display: fn() -> &'static str,
	bt: Backtrace,
}

impl PartialEq for Error {
	fn eq(&self, other: &Error) -> bool {
		self.code == other.code
	}
}

impl Debug for Error {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> FmtResult {
		#[cfg(test)]
		{
			let kind_str = (self.display)();
			let bt_text = self.bt.as_str();
			if bt_text.len() == 0 {
				write!(_f, "ErrorKind={}\n{}", kind_str,
                                "Backtrace disabled. To view backtrace set env variable; export RUST_BACKTRACE=1.")?;
			} else {
				write!(_f, "ErrorKind={}\n{}", kind_str, bt_text)?;
			}
		}
		Ok(())
	}
}

impl Error {
	pub const fn new(code: u64, display: fn() -> &'static str, bt: Backtrace) -> Self {
		Self { code, display, bt }
	}
	pub fn code(&self) -> u64 {
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
