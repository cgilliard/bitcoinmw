// Internal
extern crate bitcoinmw_macros;

pub use self::bitcoinmw_macros::Dummy;
pub use std::error::*;
/*
pub use std::{
	slice_copy, AsRaw, AsRawMut, Backtrace, Box, CString, Display, Formatter, Ptr, Rc, Result,
	SliceExt, StrExt, String, TryClone, Vec,
};
*/
pub use std::*;
/*
pub use std::misc::simple_hash;
pub use std::sliceext::SliceExt;
pub use std::strext::StrExt;
pub use std::{
	spawn, spawnj, AsRaw, Backtrace, Box, Display, Error, ErrorGen, ErrorKind, ErrorKind::*,
	Formatter, Lock, LockBox, Ptr, Rc, ResultGen, String, TryClone, Vec,
};

// macros
pub use bitcoinmw_macros::Dummy;
*/

// External
pub use core::clone::Clone;
pub use core::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};
pub use core::convert::{AsRef, From, Into, TryFrom};
pub use core::default::Default;
pub use core::fmt::Formatter as CoreFormatter;
pub use core::fmt::Result as FmtResult;
pub use core::fmt::{Debug, Error as FmtError};
pub use core::hash::{BuildHasher, Hash, Hasher};
pub use core::iter::{IntoIterator, Iterator};
pub use core::marker::{Copy, Sized};
pub use core::ops::Drop;
pub use core::option::{Option, Option::None, Option::Some};
pub use core::result::Result::{Err, Ok};

// test
#[allow(unused)]
#[cfg(test)]
pub use core::panic;
