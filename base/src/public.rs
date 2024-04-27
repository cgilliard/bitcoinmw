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

use crate::err;
use bmw_deps::failure::{Context, Fail};
use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::io::{Read, Write};

pub use crate::functions::*;

/// Base Error struct which is used throughout BitcoinMW. This should be returned from most
/// functions. Constructing a [`crate::Error`] should generally be done through the macros
/// included in this crate.
/// * [`crate::err`] (if the error is generated within this code base
/// or if a [`std::convert::From`] has been implemented)
/// or
/// * [`crate::map_err`] otherwise.
/// # See Also
/// * [`crate::err`]
/// * [`crate::map_err`]
/// # Examples
///```
/// use bmw_base::{Error, map_err, BaseErrorKind};
///
/// // return Error
/// fn main() -> Result<(), Error> {
///     // this can actually be done with just a '?' because this error
///     // a convert implemented. But just for demonstration purposes, map_err
///     // can be used.
///     let x: u32 = map_err!("1234".parse(), BaseErrorKind::Parse)?;
///     assert_eq!(x, 1234u32);
///
///     Ok(())
/// }
///```
#[derive(Debug, Fail)]
pub struct Error {
	pub(crate) inner: Context<Box<dyn ErrorKind>>,
}

/// The trait which needs to be implemented by each ErrorKind enum. Each crate can implement their
/// own enum which implements this trait. The trait itself doesn't have any functions.
/// # Examples
///```
/// use bmw_base::*;
/// use bmw_deps::failure;
/// use bmw_deps::failure::Fail;
///
/// #[derive(Clone, Eq, PartialEq, Debug, Fail)]
/// enum NewCrateErrorKind {
///     // blah error
///     #[fail(display = "blah error: {}", _0)]
///     Blah(String),
/// }
///
/// // implement the ErrorKind trait
/// impl ErrorKind for NewCrateErrorKind {}
///
/// // implement a From converter
/// impl From<NewCrateErrorKind> for Error {
///        fn from(kind: NewCrateErrorKind) -> Error {
///                Error::new(Box::new(kind))
///        }
/// }
///
/// fn main() -> Result<(), Error> {
///     let x = NewCrateErrorKind::Blah("blah error".to_string());
///     let err: Error = x.into();
///     assert_eq!(err.kind(), &kind!(NewCrateErrorKind::Blah, "blah error".to_string()));
///
///     Ok(())
/// }
///```
/// # See Also
/// * [`crate::Error`]
pub trait ErrorKind: Send + Sync + Display + Debug {}

/// The Base [`crate::ErrorKind`] implementation for BitcoinMW. Errors in this crate
/// are mapped to one of these errors. Each crate can implement their own errors using
/// the [`crate::ErrorKind`] trait.
/// # Also see
/// * [`crate::Error`]
/// * [`crate::ErrorKind`]
#[derive(Debug, Fail)]
pub enum BaseErrorKind {
	/// Parse error
	#[fail(display = "parse error: {}", _0)]
	Parse(String),
	/// Corrupted data error
	#[fail(display = "corrupted data error: {}", _0)]
	CorruptedData(String),
	/// Operation not supported Error
	#[fail(display = "operation not supported error: {}", _0)]
	OperationNotSupported(String),
	/// TryInto error
	#[fail(display = "try into error: {}", _0)]
	TryInto(String),
	/// Illegal state
	#[fail(display = "illegal state: {}", _0)]
	IllegalState(String),
	/// Configuration error
	#[fail(display = "configuration error: {}", _0)]
	Configuration(String),
	/// Build error
	#[fail(display = "builder error: {}", _0)]
	Builder(String),
	/// I/O error
	#[fail(display = "i/o error: {}", _0)]
	IO(String),
	/// TryFrom error
	#[fail(display = "try/from error: {}", _0)]
	TryFrom(String),
	/// OsString error
	#[fail(display = "osstring error: {}", _0)]
	OsString(String),
	/// Utf8 error
	#[fail(display = "utf8 error: {}", _0)]
	Utf8(String),
	/// Poison error
	#[fail(display = "poison error: {}", _0)]
	Poison(String),
	/// Alloc error
	#[fail(display = "alloc error: {}", _0)]
	Alloc(String),
	/// Misc error
	#[fail(display = "misc error: {}", _0)]
	Misc(String),
	/// SystemTime error
	#[fail(display = "system time error: {}", _0)]
	SystemTime(String),
	/// Errno error
	#[fail(display = "errno: {}", _0)]
	Errno(String),
	/// IllegalArgument
	#[fail(display = "illegal argument: {}", _0)]
	IllegalArgument(String),
}

/// The [`crate::Configurable`] trait, when implemented, allows structs to be configured.
/// Currently, [`u8`], [`u16`], [`u32`], [`u64`], [`u128`], [`usize`], [`std::string::String`] and a string tuple `(String, String)` are
/// supported. Also, a [`std::vec::Vec`] of any of these types are supported. This should generally be used with the
/// proc-macro Configurable, but that is done at a higher level crate so see its documentation
/// there in the `derive` crate.
pub trait Configurable {
	/// sets the configuration with the specified `name` to the specified [`prim@u8`] value
	fn set_u8(&mut self, name: &str, value: u8);
	/// sets the configuration with the specified `name` to the specified [`prim@u16`] value
	fn set_u16(&mut self, name: &str, value: u16);
	/// sets the configuration with the specified `name` to the specified [`prim@u32`] value
	fn set_u32(&mut self, name: &str, value: u32);
	/// sets the configuration with the specified `name` to the specified [`prim@u64`] value
	fn set_u64(&mut self, name: &str, value: u64);
	/// sets the configuration with the specified `name` to the specified [`prim@u128`] value
	fn set_u128(&mut self, name: &str, value: u128);
	/// sets the configuration with the specified `name` to the specified [`prim@usize`] value
	fn set_usize(&mut self, name: &str, value: usize);
	/// sets the configuration with the specified `name` to the specified [`std::string::String`] value
	fn set_string(&mut self, name: &str, value: String);
	/// sets the configuration with the specified `name` to the specified [`prim@bool`] value
	fn set_bool(&mut self, name: &str, value: bool);
	/// sets the configuration with the specified `name` to the specified `(String, String)` value
	fn set_string_tuple(&mut self, name: &str, value: (String, String));
	/// returns a [`std::collections::HashSet`] with the configurations that allow duplicates.
	/// This is used by the `config` macro when [`std::vec::Vec`] configuration
	/// options are used.
	fn allow_dupes(&self) -> HashSet<String>;
}

/// Writer trait used for serializing data.
/// # Examples
///```
/// use bmw_base::*;
///
///    // type that can be used to generate an error
///    #[derive(Debug, PartialEq)]
///    struct SerErr {
///         exp: u8,
///         empty: u8,
///     }
///
///     // serializable trait requires both a Reader/Writer.
///     impl Serializable for SerErr {
///         fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
///         // read data but return an error unless a specific value is set
///         reader.expect_u8(99)?;
///         reader.read_empty_bytes(1)?;
///         Ok(Self { exp: 99, empty: 0 })
///     }
///
///     fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
///         // write is regular with no errors
///         writer.write_u8(self.exp)?;
///         writer.write_u8(self.empty)?;
///         Ok(())
///     }
///
/// }
///
/// fn main() -> Result<(), Error> {
///     // test an error
///     let ser_out = SerErr { exp: 100, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_err());
///
///     // test with the values that do not generate an error
///     let ser_out = SerErr { exp: 99, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_ok());
///
///     Ok(())
/// }
///```
/// # Also see
/// * [`crate::Reader`]
/// * [`crate::BinReader`]
/// * [`crate::BinWriter`]
pub trait Writer {
	/// write a u8 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u8`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u8(&mut self, n: u8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n])
	}

	/// write an i8 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i8`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i8(&mut self, n: i8) -> Result<(), Error> {
		self.write_fixed_bytes(&[n as u8])
	}

	/// write a u16 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u16`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u16(&mut self, n: u16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i16 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u16`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i16(&mut self, n: i16) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u32 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u32`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u32(&mut self, n: u32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i32 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i32`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i32(&mut self, n: i32) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u64 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u64`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u64(&mut self, n: u64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i128 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i128`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i128(&mut self, n: i128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a u128 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`u128`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_u128(&mut self, n: u128) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write an i64 to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`i64`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_i64(&mut self, n: i64) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write a usize to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `n` - [`usize`] - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_usize(&mut self, n: usize) -> Result<(), Error> {
		self.write_fixed_bytes(n.to_be_bytes())
	}

	/// write `bytes` to the stream and specify the length so that variable length data may be
	/// written to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `bytes` - `AsRef<[u8]>` - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error> {
		self.write_u64(bytes.as_ref().len() as u64)?;
		self.write_fixed_bytes(bytes)
	}

	/// write `bytes` to the stream
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `bytes` - `AsRef<[u8]>` - the value to write.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_fixed_bytes<T: AsRef<[u8]>>(&mut self, bytes: T) -> Result<(), Error>;

	/// write `length` empty (0u8) bytes to a stream.
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to this [`crate::Writer`].
	/// * `length` - [`usize`] - the length of the empty bytes to write
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn write_empty_bytes(&mut self, length: usize) -> Result<(), Error> {
		for _ in 0..length {
			self.write_u8(0)?;
		}
		Ok(())
	}
}

/// Reader trait used for deserializing data.
/// # Examples
///```
/// use bmw_base::*;
///
///    // type that can be used to generate an error
///    #[derive(Debug, PartialEq)]
///    struct SerErr {
///         exp: u8,
///         empty: u8,
///     }
///
///     // serializable trait requires both a Reader/Writer.
///     impl Serializable for SerErr {
///         fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
///         // read data but return an error unless a specific value is set
///         reader.expect_u8(99)?;
///         reader.read_empty_bytes(1)?;
///         Ok(Self { exp: 99, empty: 0 })
///     }
///
///     fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
///         // write is regular with no errors
///         writer.write_u8(self.exp)?;
///         writer.write_u8(self.empty)?;
///         Ok(())
///     }
///
/// }
///
/// fn main() -> Result<(), Error> {
///     // test an error
///     let ser_out = SerErr { exp: 100, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_err());
///
///     // test with the values that do not generate an error
///     let ser_out = SerErr { exp: 99, empty: 0 };
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &ser_out)?;
///     let ser_in: Result<SerErr, Error> = deserialize(&mut &v[..]);
///     assert!(ser_in.is_ok());
///
///     Ok(())
/// }
///```
/// # Also see
/// * [`crate::Writer`]
/// * [`crate::BinReader`]
/// * [`crate::BinWriter`]
pub trait Reader {
	/// read a u8 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u8`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u8(&mut self) -> Result<u8, Error>;
	/// read an i8 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i8`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i8(&mut self) -> Result<i8, Error>;
	/// read an i16 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i16`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i16(&mut self) -> Result<i16, Error>;
	/// read a u16 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u16`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u16(&mut self) -> Result<u16, Error>;
	/// read a u32 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u32`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u32(&mut self) -> Result<u32, Error>;
	/// read a u64 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u64`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u64(&mut self) -> Result<u64, Error>;
	/// read a u128 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`u128`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_u128(&mut self) -> Result<u128, Error>;
	/// read an i128 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i128`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i128(&mut self) -> Result<i128, Error>;
	/// read an i32 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i32`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i32(&mut self) -> Result<i32, Error>;
	/// read an i64 from the reader and return the value
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`i64`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_i64(&mut self) -> Result<i64, Error>;
	/// read a fixed length of bytes from the reader store them in `buf`
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// * `buf` - [&mut `[u8]`] - a mutable reference to a byte array to store the returned
	/// data in.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_fixed_bytes(&mut self, buf: &mut [u8]) -> Result<(), Error>;
	/// read usize from the Reader.
	/// # Input Parameters
	/// `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// # Return
	/// The [`usize`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_usize(&mut self) -> Result<usize, Error>;
	/// expect a specific byte, otherwise return an error
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// * `val` - [`u8`] - the expected value to be read.
	/// # Return
	/// The [`u8`] that is read from the [`crate::Reader`].
	/// # Errors
	/// * [`crate::BaseErrorKind::CorruptedData`] - if the byte read is not equal to `val`
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn expect_u8(&mut self, val: u8) -> Result<u8, Error>;

	/// Read bytes, expect them all to be 0u8. Otherwise, reutrn an error.
	/// # Input Parameters
	/// * `&mut self` - a mutable reference to self the [`crate::Reader`].
	/// * `length` - [`usize`] - the length to read.
	/// # Return
	/// [`unit`]
	/// # Errors
	/// * [`crate::BaseErrorKind::CorruptedData`] - if the byte read is not equal to `val`
	/// * [`crate::BaseErrorKind::IO`] - if an i/o error occurs
	/// # Also see
	/// * [`crate::Reader`]
	/// * [`crate::Writer`]
	/// * [`crate::BinReader`]
	/// * [`crate::BinWriter`]
	fn read_empty_bytes(&mut self, length: usize) -> Result<(), Error> {
		for _ in 0..length {
			if self.read_u8()? != 0u8 {
				return err!(BaseErrorKind::CorruptedData, "expected 0u8");
			}
		}
		Ok(())
	}
}

/// This is the trait used by all data structures to serialize and deserialize data.
/// Anything stored in them must implement this trait. Commonly needed implementations
/// are built in the ser module in this crate. These include Vec, String, integer types among
/// other things.
///
/// # Examples
///
///```
/// use bmw_base::*;
/// use std::fmt::Debug;
///
/// #[derive(Debug, PartialEq)]
/// struct SerEx {
///     a: u8,
///     b: u128,
/// }
///
/// impl Serializable for SerEx {
///     fn read<R: Reader>(reader: &mut R) -> Result<Self, Error> {
///         let a = reader.read_u8()?;
///         let b = reader.read_u128()?;
///
///         let ret = Self {
///             a: a,
///             b,
///         };
///
///         Ok(ret)
///     }
///     fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error> {
///         writer.write_u8(self.a)?;
///         writer.write_u128(self.b)?;
///         Ok(())
///     }
/// }
///
/// // helper function that serializes and deserializes a Serializable and tests them for
/// // equality
/// fn ser_helper<S: Serializable + Debug + PartialEq>(input: S) -> Result<(), Error> {
///     let mut v: Vec<u8> = vec![];
///     serialize(&mut v, &input)?;
///     let output: S = deserialize(&mut &v[..])?;
///     assert_eq!(output, input);
///     Ok(())
/// }
///
/// fn main() -> Result<(), Error> {
///     let v = SerEx {
///         a: 100,
///         b: 1_000,
///     };
///     ser_helper(v)?;
///     Ok(())
/// }
///```
pub trait Serializable {
	/// read data from the reader and build the underlying type represented by that
	/// data.
	fn read<R: Reader>(reader: &mut R) -> Result<Self, Error>
	where
		Self: Sized;
	/// write data to the writer representing the underlying type.
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), Error>;
}

/// Utility wrapper for an underlying byte Writer. Defines higher level methods
/// to write numbers, byte vectors, hashes, etc by implementing the [`crate::Writer`]
/// trait.
/// # Also see
/// * [`crate::BinReader`]
/// * [`crate::Reader`]
/// * [`crate::Writer`]
pub struct BinWriter<'a> {
	pub(crate) sink: &'a mut dyn Write,
}

/// Utility wrapper for an underlying byte Reader. Defines higher level methods
/// to write numbers, byte vectors, hashes, etc by implementing the [`crate::Reader`]
/// trait.
/// # Also see
/// * [`crate::BinWriter`]
/// * [`crate::Reader`]
/// * [`crate::Writer`]
pub struct BinReader<'a, R: Read> {
	pub(crate) source: &'a mut R,
}