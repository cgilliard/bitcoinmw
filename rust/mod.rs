#![no_std]
#![feature(new_range_api)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![no_implicit_prelude]

#[macro_use]
pub mod std;
#[macro_use]
pub mod util;

pub mod bible;
pub mod crypto;
pub mod lmdb;
pub mod mw;
pub mod net;
pub mod prelude;
mod real_main;
pub mod store;
