use core::fmt::Formatter as CoreFormatter;
use prelude::*;

macro_rules! define_errorkind_with_strings {
    ( $( $variant:ident ),* ) => {
        #[derive(PartialEq)]
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
    };
}

define_errorkind_with_strings!(
	Unknown,
	Alloc,
	InvalidBlindSum,
	IllegalArgument,
	ArrayIndexOutOfBounds,
	InvalidPublicKey,
	InvalidCommitment,
	InvalidSignature,
	Overflow,
	Underflow,
	PubkeyCreate,
	TooManySignatures,
	IllegalState,
	Serialization,
	IO,
	Todo
);

#[derive(PartialEq)]
pub struct Error {
	pub kind: ErrorKind,
}

impl Error {
	pub fn new(kind: ErrorKind) -> Self {
		Self { kind }
	}
}

impl Debug for Error {
	fn fmt(&self, _: &mut CoreFormatter<'_>) -> Result<(), FmtError> {
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_error() {
		let e1 = Error::new(Alloc);
		let e2 = Error::new(Todo);
		let e3 = Error::new(Alloc);
		let e4 = Error::new(IllegalArgument);
		let e5 = Error::new(IllegalState);
		let e6 = Error::new(ArrayIndexOutOfBounds);
		let e7 = Error::new(Serialization);
		let e8 = Error::new(IO);
		assert_eq!(e1, e3);
		assert!(e1 != e2);
		assert!(e4 != e5);
		assert!(e6 != e7);
		assert!(e8 != e1);
		assert!(e8.kind.as_str() == "IO");

		let res = match e3.kind {
			_ => 1,
		};
		assert_eq!(res, 1);
	}
}
