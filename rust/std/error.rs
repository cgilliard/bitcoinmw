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
	InvalidRangeProof,
	InvalidSignature,
	InvalidTransaction,
	InvalidSecretKey,
	InsufficientBlockHash,
	InvalidBlockHash,
	BlockNotFound,
	KernelNotFound,
	MultipleCoinbase,
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
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> Result<(), FmtError> {
		// to support mrustc builds we cannot 'write'. Only used in tests to display debugging
		// information. In production we should not rely on printout of Errors.
		#[cfg(test)]
		write!(
			_f,
			"ErrorKind={}: {}:{}",
			self.kind.as_str(),
			file!(),
			line!()
		)?;
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
