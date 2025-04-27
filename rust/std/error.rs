use prelude::*;

errors!(
	Unknown,
	OutOfBounds,
	IllegalArgument,
	IllegalState,
	IO,
	CcapacityExceeded,
	Utf8Error,
	Alloc,
	Todo
);

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

impl Display for Error {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		let kind_str = (self.display)();
		let bt_text = self.bt.as_str();
		if bt_text.len() == 0 {
			writef!(
				f,
				"ErrorKind={}\n{}",
				kind_str,
				"Backtrace disabled. To view backtrace set env variable; export RUST_BACKTRACE=1."
			)?;
		} else {
			writef!(f, "ErrorKind={}\n{}", kind_str, bt_text)?;
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
		IO,
		IllegalArgument,
		OutOfBounds,
		IllegalState,
		OutOfMemory,
		OperationFailed
	);

	fn try_errors(x: u32) -> Result<()> {
		if x == 1 {
			err!(OutOfBounds)
		} else if x == 2 {
			err!(OperationFailed)
		} else if x == 3 {
			err!(OutOfMemory)
		} else if x == 4 {
			err!(IO)
		} else if x == 5 {
			err!(IllegalArgument)
		} else if x == 6 {
			err!(IllegalState)
		} else {
			Ok(())
		}
	}

	#[test]
	fn test_error_ret() -> Result<()> {
		assert_eq!(try_errors(1), err!(OutOfBounds));
		assert_eq!(try_errors(2), err!(OperationFailed));
		assert_eq!(try_errors(3), err!(OutOfMemory));
		assert_ne!(try_errors(3), err!(OutOfBounds));
		Ok(())
	}
}
