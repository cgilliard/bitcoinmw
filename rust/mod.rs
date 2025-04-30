#![no_std]
#![feature(new_range_api)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![no_implicit_prelude]

#[macro_use]
pub mod std;

pub mod net;
pub mod prelude;
mod real_main;
pub mod util;
