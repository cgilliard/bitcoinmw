mod cstring;
mod error;
mod ffi;
mod ptr;

pub use std::cstring::CStr;
pub use std::error::{Error, ErrorKind};
pub use std::ptr::Ptr;

#[cfg(test)]
pub use std::ffi::getalloccount;
