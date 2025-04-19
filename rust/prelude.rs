// Internal
pub use std::{Error, ErrorKind::*};

// External
pub use core::clone::Clone;
pub use core::cmp::PartialEq;
pub use core::fmt::{Debug, Error as FmtError};
pub use core::iter::{IntoIterator, Iterator};
pub use core::marker::{Copy, Sized};
pub use core::ops::Drop;
pub use core::option::{Option, Option::None, Option::Some};
pub use core::result::{Result, Result::Err, Result::Ok};

// test
#[allow(unused)]
#[cfg(test)]
pub use core::panic;
