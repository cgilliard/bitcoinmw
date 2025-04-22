use core::fmt::Formatter as CoreFormatter;
use prelude::*;
use std::cstring::CString;
use std::ffi::{format_err, write};

macro_rules! define_errorkind_with_strings {
    ( $( $variant:ident ),* ) => {
        #[derive(PartialEq, Ord, PartialOrd, Eq)]
        pub enum ErrorKind {
            $( $variant ),*
        }

        impl ErrorKind {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $( Self::$variant => stringify!($variant) ),*
                }
            }
        }

        impl Display for ErrorKind {
            fn format(&self, f: &mut Formatter) -> Result<(), Error> {
                writef!(f, "{}", match self { $( Self::$variant => stringify!($variant) ),*  })
            }
        }
    };
}

define_errorkind_with_strings!(
	Unknown,
	Alloc,
	IllegalArgument,
	IllegalState,
	Utf8Error,
	Overflow,
	Underflow,
	IO,
	Duplicate,
	InsufficientEntropy,
	OperationFailed,
	ValidationFailed,
	Serialization,
	OutOfBounds,
	ThreadCreate,
	ThreadDetach,
	ThreadJoin,
	ChannelInit,
	ChannelSend,
	LmdbBeginTxn,
	LmdbCreate,
	LmdbOpen,
	LmdbCommit,
	LmdbFull,
	LmdbGet,
	LmdbPut,
	LmdbDel,
	LmdbCursor,
	Todo
);

#[derive(PartialEq)]
pub struct Error {
	pub kind: ErrorKind,
}

impl Error {
	#[inline]
	pub fn new(kind: ErrorKind) -> Self {
		Self { kind }
	}
}

impl Debug for Error {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> Result<(), FmtError> {
		// There doesn't seem to be a way to call formatter in no_std so we print to stdout
		// instead
		let kind_str = self.kind.as_str();
		match CString::new(kind_str) {
			Ok(cstr) => unsafe {
				let value = CString::from_ptr(format_err(cstr.as_ptr(), cstr.len()), false);
				write(2, "\n".as_ptr(), 1);
				write(2, value.as_ptr(), value.len());
				write(2, "\n".as_ptr(), 1);
				#[cfg(test)]
				write!(_f, "{}", value.as_str().unwrap().to_str())?;
			},
			Err(_) => {}
		}
		Ok(())
	}
}

impl Display for Error {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let kind_str = self.kind.as_str();
		match CString::new(kind_str) {
			Ok(cstr) => unsafe {
				let value = CString::from_ptr(format_err(cstr.as_ptr(), cstr.len()), false);
				let value = value.as_str()?;
				writef!(f, "{}", value)
			},
			Err(_) => {
				// attempt to just write the error kind
				writef!(
					f,
					"[ErrorKind={}]: {}:{}",
					self.kind.as_str(),
					file!(),
					line!()
				)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::ffi::getalloccount;

	fn test_error_return() -> Result<(), Error> {
		Err(Error::new(IllegalState))
	}

	#[test]
	fn test_error() -> Result<(), Error> {
		let initial = unsafe { getalloccount() };
		{
			let e1 = Error::new(Alloc);
			let e2 = Error::new(Todo);
			let e3 = Error::new(Alloc);
			let e4 = Error::new(IllegalArgument);
			let e5 = Error::new(IllegalState);
			let e6 = Error::new(OutOfBounds);
			let e7 = Error::new(Serialization);
			let e8 = Error::new(IO);
			assert_eq!(e1, e3);
			assert!(e1 != e2);
			assert!(e4 != e5);
			assert!(e6 != e7);
			assert!(e8 != e1);
			assert!(e8.kind.as_str() == "IO");
			assert!(e7.kind.as_str() == "Serialization");

			assert!(test_error_return().is_err());

			let res = match e3.kind {
				_ => 1,
			};
			assert_eq!(res, 1);
		}
		unsafe {
			assert_eq!(initial, getalloccount());
		}
		Ok(())
	}
}
