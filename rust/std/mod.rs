#[macro_use]
mod macros;

mod as_raw;
mod backtrace;
mod boxed;
mod clone;
mod constants;
mod cstring;
mod display;
mod format;
mod misc;
mod ptr;
mod rc;
mod sliceext;
mod strext;
mod string;
mod vec;

pub(crate) mod error;
pub(crate) mod ffi;

use std::error::Error;
pub type Result<T> = crate::core::result::Result<T, Error>;

pub use std::as_raw::{AsRaw, AsRawMut};
pub use std::backtrace::Backtrace;
pub use std::boxed::Box;
pub use std::clone::TryClone;
pub use std::cstring::CString;
pub use std::display::Display;
pub use std::format::Formatter;
pub use std::misc::*;
pub use std::ptr::Ptr;
pub use std::rc::Rc;
pub use std::sliceext::SliceExt;
pub use std::strext::StrExt;
pub use std::string::String;
pub use std::vec::Vec;
