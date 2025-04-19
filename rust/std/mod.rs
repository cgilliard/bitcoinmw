#[macro_use]
mod macros;

mod boxed;
mod clone;
mod cmp;
mod constants;
mod cstring;
mod display;
mod error;
mod ffi;
mod format;
mod misc;
mod ptr;
mod rc;
mod strext;
mod string;
mod vec;

pub use std::boxed::Box;
pub use std::clone::TryClone;
pub use std::cmp::Ord;
pub use std::cstring::CStr;
pub use std::display::Display;
pub use std::error::{Error, ErrorKind};
pub use std::format::Formatter;
pub use std::ptr::Ptr;
pub use std::rc::Rc;
pub use std::string::String;
pub use std::vec::Vec;

#[cfg(test)]
pub use std::ffi::getalloccount;
