#![no_std]

mod constants;

#[macro_use]
pub mod macros;

pub mod backtrace;
pub mod error;
pub mod errors;
pub mod ffi;
pub mod misc;
pub mod prelude;
pub mod result;

pub struct Error {
	pub code: u32,
}

pub struct MyTest {
	pub x: u32,
	pub y: u64,
}
