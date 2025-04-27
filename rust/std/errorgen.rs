use core::fmt::Error as FormatError;
use core::fmt::Formatter as CoreFormatter;
use prelude::*;

#[derive(Clone)]
pub struct ErrorGen {
	pub code: u64,
	pub display: fn() -> &'static str,
	pub bt: Backtrace,
}

impl PartialEq for ErrorGen {
	fn eq(&self, other: &ErrorGen) -> bool {
		self.code == other.code
	}
}

impl Debug for ErrorGen {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> Result<(), FormatError> {
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

impl Display for ErrorGen {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
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

pub type ResultGen<T> = Result<T, ErrorGen>;

errors!(IO, IllegalArgument, OutOfBounds);
errors!(IllegalState, OutOfMemory, OperationFailed);

#[cfg(test)]
mod test {
	/*
	use super::*;

	#[test]
	fn test_error_simple() -> ResultGen<()> {
		err!(IO)
	}

	#[test]
	fn test_error_simple2() -> ResultGen<()> {
		//panic!("1");
		err!(IllegalArgument)
	}

	#[test]
	fn test_error_simple3() -> ResultGen<()> {
		err!(OutOfBounds)
	}

	#[test]
	fn test_error_simple4() -> ResultGen<()> {
		err!(IllegalState)
	}

	#[test]
	fn test_error_simple5() -> ResultGen<()> {
		err!(OutOfMemory)
	}

	#[test]
	fn test_error_simple6() -> ResultGen<()> {
		Err(OperationFailed)
	}

	fn test1() -> ResultGen<()> {
		err!(OperationFailed)
	}
	#[test]
	fn test_error_simple7() -> ResultGen<()> {
		//panic!("1");
		let v = 123;
		let x = v + 1;
		let _z = x + 3;
		test1()?;

		println!("test");
		Ok(())
	}
	*/
}
