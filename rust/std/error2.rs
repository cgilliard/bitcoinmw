use core::fmt::Error as FormatError;
use core::fmt::Formatter as CoreFormatter;
use prelude::*;

#[derive(Clone)]
pub struct Error2 {
	code: u64,
	display: fn() -> &'static str,
	pub bt: Backtrace,
}

impl PartialEq for Error2 {
	fn eq(&self, other: &Error2) -> bool {
		self.code == other.code
	}
}

impl Debug for Error2 {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> Result<(), FormatError> {
		#[cfg(test)]
		{
			let kind_str = (self.display)();
			let bt_text = self.bt.as_str();
			if bt_text.len() == 0 {
				write!(_f, "ErrorKind={}\n{}", kind_str,
				"Backtrace disabled. To view backtrace set env variable; export RUST_BACKTRACE=1.")?;
			} else {
				write!(_f, "ErrorKind={}\nstack backtrace: {}", kind_str, bt_text)?;
			}
		}
		Ok(())
	}
}

impl Display for Error2 {
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
			writef!(f, "ErrorKind={}\nstack backtrace: {}", kind_str, bt_text)?;
		}
		Ok(())
		/*
		match CString::new(kind_str) {
			Ok(cstr) => unsafe {
				let value = CString::from_ptr(format_err(cstr.as_ptr(), cstr.len()), false);
				let value = value.as_str()?;
				writef!(f, "{}", value)
			},
			Err(_) => {
				// attempt to just write the error kind
				writef!(f, "[ErrorKind={}]: {}:{}", kind_str, file!(), line!())
			}
		}
				*/
	}
}

pub type Result2<T> = Result<T, Error2>;

const fn simple_hash(s: &str, line: u32) -> u64 {
	let mut hash = 0_u64;
	let bytes = s.as_bytes();
	let mut i = 0;
	while i < bytes.len() {
		hash = hash ^ (bytes[i] as u64);
		hash = (hash << 3) ^ (hash >> 2);
		i += 1;
	}
	hash = hash ^ (line as u64);
	hash = (hash << 5) ^ (hash >> 1);
	hash
}

#[macro_export]
macro_rules! errors {
    ($($error:ident),*) => {
        define_errors_inner!(@count 0, simple_hash(file!(), line!()), $($error),*);
    };
}

macro_rules! define_errors_inner {
	(@count $index:expr, $file_hash:expr, $head:ident $(, $tail:ident)*) => {
		#[allow(non_upper_case_globals)]
		pub const $head: Error2 = Error2{
                    code: $file_hash + $index,
                    display: || -> &'static str { stringify!($head) },
                    bt: Backtrace (crate::core::ptr::null())
                };
		define_errors_inner!(@count $index + 1, $file_hash, $($tail),*);
	};
	(@count $index:expr, $file_hash:expr,) => {};
}

errors!(IO, IllegalArgument, OutOfBounds);
errors!(IllegalState, OutOfMemory, OperationFailed);

#[cfg(test)]
mod test {
	/*
	use super::*;

	#[test]
	fn test_error_simple() -> Result2<()> {
		Err(IO)
	}

	#[test]
	fn test_error_simple2() -> Result2<()> {
		//panic!("1");
		Err(err!(IllegalArgument))
	}

	#[test]
	fn test_error_simple3() -> Result2<()> {
		Err(OutOfBounds)
	}

	#[test]
	fn test_error_simple4() -> Result2<()> {
		Err(IllegalState)
	}

	#[test]
	fn test_error_simple5() -> Result2<()> {
		Err(OutOfMemory)
	}

	#[test]
	fn test_error_simple6() -> Result2<()> {
		Err(OperationFailed)
	}

	fn test1() -> Result2<()> {
		Err(err!(OperationFailed))
	}
	#[test]
	fn test_error_simple7() -> Result2<()> {
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
