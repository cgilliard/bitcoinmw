#![no_std]
#![feature(new_range_api)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![no_implicit_prelude]

mod constants;

#[macro_use]
pub mod macros;

pub mod backtrace;
pub mod boxed;
pub mod error;
pub mod errors;
pub mod ffi;
pub mod format;
pub mod misc;
pub mod prelude;
pub mod ptr;
pub mod rc;
pub mod result;
pub mod string;
pub mod traits;
pub mod vec;
