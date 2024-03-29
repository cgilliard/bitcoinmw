// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # The BMW Error crate
//! This crate includes the main error structs, enums and macros used
//! in BMW for building and mapping errors. This crate offers
//! wrappers around the rust failure crate. The [`crate::map_err`]
//! macro can be used to conveniently map errors from 3rd party crates
//! into [`crate::ErrorKind`] in this crate. The [`crate::err`] macro
//! can be used to generate errors. In most cases errors should be created
//! using one of these two macros.
//!
//! # Examples
//!```
//! // Example of the err macro
//! use bmw_err::{Error, ErrorKind, ErrKind, err, map_err, try_into};
//! use std::path::PathBuf;
//! use std::fs::File;
//! use std::io::Write;
//!
//! fn process_file(path: &str) -> Result<(), Error> {
//!     if ! PathBuf::from(path).exists() {
//!         return Err(err!(ErrKind::IllegalArgument, "path does not exist"));
//!     }
//!
//!     // .. process file
//!
//!     Ok(())
//! }
//!
//! // Example of the map_err macro
//! fn show_map_err() -> Result<(), Error> {
//!     // map the file open error to a 'Log' Error. The text of the original error will be
//!     // included in the mapped error.
//!     let mut x = map_err!(File::open("/invalid/log/path.log"), ErrKind::Log)?;
//!     x.write(b"test")?;
//!
//!     // optionally an additional message can be included as below. The original
//!     // error's message will still be displayed.
//!     let file = map_err!(
//!         File::open("/path/to/something"),
//!         ErrKind::IO,
//!         "file open failed"
//!     )?;
//!     println!("file_type={:?}", file.metadata()?.file_type());
//!
//!
//!     Ok(())
//! }
//!
//! // Example of the err macro
//! fn return_err() -> Result<(), Error> {
//!     // return an error with the ErrKind::Misc with the text below
//!     Err(err!(ErrKind::Misc, "misc error occured"))
//! }
//!
//! // Example of the try_into macro
//! fn try_into_ex() -> Result<(), Error> {
//!     // try to convert a u128 to a usize
//!     let _v: usize = try_into!(100u128)?;
//!     Ok(())
//! }
//!
//! fn main() -> Result<(), Error> {
//!     try_into_ex()?;
//!     assert!(return_err().is_err());
//!     assert!(show_map_err().is_err());
//!     let mut buf = PathBuf::new();
//!     buf.push("/path/to/err");
//!     assert!(process_file(buf.to_str().unwrap()).is_err());
//!     Ok(())
//! }
//!
//!```
//!
//! # Formatting
//!
//! The [`crate::err`] macro may be used to format output just like [`std::println`] or
//! [`std::format`]. See the example below for details.
//!
//!```
//! use bmw_err::{Error, ErrKind, err};
//!
//! // The error macro can be used like the format macro to format output
//! fn test() -> Result<(), Error> {
//!     let  code = 404;
//!     // return an error with formatting
//!     Err(err!(ErrKind::Http, "The webserver returned the error code: {}", code))
//! }
//!
//! fn main() -> Result<(), Error> {
//!     assert!(test().is_err());
//!     Ok(())
//! }
//!```

use bmw_deps::failure;

mod error;
mod macros;
mod test;
mod types;

pub use crate::types::{ErrKind, Error, ErrorKind};
