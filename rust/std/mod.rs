#[macro_use]
mod macros;

mod as_raw;
mod boxed;
mod channel;
mod clone;
mod constants;
mod cstring;
mod display;
mod error;
mod format;
mod lock;
mod ptr;
mod rc;
mod string;
mod thread;
mod vec;

pub(crate) mod ffi;
pub(crate) mod misc;
pub(crate) mod sliceext;
pub(crate) mod strext;

pub use std::as_raw::AsRaw;
pub use std::boxed::Box;
pub use std::channel::{channel, Receiver, Sender};
pub use std::clone::TryClone;
pub use std::cstring::CString;
pub use std::display::Display;
pub use std::error::{Error, ErrorKind};
pub use std::format::Formatter;
pub use std::lock::{Lock, LockBox};
pub use std::ptr::Ptr;
pub use std::rc::Rc;
pub use std::string::String;
pub use std::thread::{spawn, spawnj};
pub use std::vec::Vec;

#[cfg(test)]
pub use std::ffi::getalloccount;
