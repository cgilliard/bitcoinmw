#![no_std]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![no_implicit_prelude]

#[macro_use]
pub mod std;

pub mod bible;
pub mod crypto;
pub mod lmdb;
pub mod mw;
pub mod net;
pub mod prelude;
mod real_main;
pub mod store;
pub mod util;
