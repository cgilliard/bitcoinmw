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
	Alloc,
	IllegalArgument,
	ArrayIndexOutOfBounds,
	IllegalState,
	Serialization,
	Secp,
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

#[cfg(test)]
impl Debug for Error {
	fn fmt(&self, _: &mut Formatter<'_>) -> Result<(), FmtError> {
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
		assert_eq!(e1, e3);
		assert!(e1 != e2);

		let res = match e3.kind {
			Alloc => 1,
			Todo => 2,
			_ => 3,
		};
		assert_eq!(res, 1);
	}
}
