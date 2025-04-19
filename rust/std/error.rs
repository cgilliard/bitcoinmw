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

define_errorkind_with_strings!(Unknown, Alloc);

pub struct Error {
	pub kind: ErrorKind,
}
