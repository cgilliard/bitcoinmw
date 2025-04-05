// Internal
pub use std::error::{Error, ErrorKind, ErrorKind::*};

// External
pub use core::fmt::{Debug, Error as FmtError, Formatter};
pub use core::ops::Drop;
pub use core::result::{Result, Result::Err, Result::Ok};
