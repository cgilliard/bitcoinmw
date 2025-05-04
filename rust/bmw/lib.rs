#![no_std]
#![feature(new_range_api)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![no_implicit_prelude]

#[cfg(not(test))]
use core::panic::PanicInfo;
#[cfg(not(test))]
#[panic_handler]
fn bmw_panic(_info: &PanicInfo) -> ! {
	loop {} // Infinite loop on panic
}

#[macro_use]
extern crate base;
extern crate macros;

pub use base::ffi;
pub use base::misc;

pub mod crypto;
pub mod prelude;
mod real_main;
pub mod util;
