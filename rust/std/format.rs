use core::str::from_utf8_unchecked;
use prelude::*;
use std::ffi::f64_to_str;
use std::misc::{slice_copy, i128_to_str, subslice, subslice_mut, u128_to_str};

pub struct Formatter {
	buffer: Vec<u8>,
}

impl Formatter {
	pub fn new() -> Self {
		Self { buffer: Vec::new() }
	}

	pub fn with_capacity(size: usize) -> Result<Self, Error> {
		Ok(Self {
			buffer: Vec::with_capacity(size)?,
		})
	}

	pub fn clear(&mut self) -> Result<(), Error> {
		self.buffer.clear()
	}

	pub fn write_str(&mut self, s: &str, len: usize) -> Result<(), Error> {
		let bytes = s.as_bytes();
		let start = self.buffer.len();
		match self.buffer.resize(start + len) {
			Ok(_) => {}
			Err(e) => return Err(e),
		}
		let dest_slice = subslice_mut(&mut self.buffer, start, len)?;
		slice_copy(bytes, dest_slice, len)?;
		Ok(())
	}

	pub fn as_str(&self) -> &str {
		if self.buffer.len() == 0 {
			""
		} else {
			unsafe { from_utf8_unchecked(&self.buffer.slice(0, self.buffer.len())) }
		}
	}
}

macro_rules! impl_display_unsigned {
	($($t:ty),*) => {
		$(
			impl Display for $t {
				fn format(&self, f: &mut Formatter) -> Result<(), Error> {
					let mut buf = [0u8; 64];
					let len = u128_to_str((*self) as u128, 0, &mut buf, 10);
					unsafe { f.write_str(from_utf8_unchecked(&buf), len) }
				}
			}
		)*
	};
}

impl_display_unsigned!(u8, u16, u32, u64, u128);

impl Display for usize {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let mut buf = [0u8; 64];
		let len = u128_to_str(*self as u128, 0, &mut buf, 10);
		unsafe { f.write_str(from_utf8_unchecked(&buf), len) }
	}
}

impl Display for char {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let mut buf = [0u8; 4];
		let len = self.encode_utf8(&mut buf).len();
		if len > 0 {
			let buf = subslice(&buf, 0, len)?;
			let buf = unsafe { from_utf8_unchecked(&buf) };
			f.write_str(buf, len)
		} else {
			Ok(())
		}
	}
}

macro_rules! impl_display_signed {
	($($t:ty),*) => {
		$(
			impl Display for $t {
				fn format(&self, f: &mut Formatter) -> Result<(), Error> {
					let mut buf = [0u8; 64];
					let len = i128_to_str((*self) as i128, &mut buf, 10);
					unsafe { f.write_str(from_utf8_unchecked(&buf), len) }
				}
			}
		)*
	};
}

impl_display_signed!(i8, i16, i32, i64, i128);

impl Display for f64 {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let mut buf = [0u8; 512];
		let len = unsafe { f64_to_str(*self, buf.as_mut_ptr(), 512) };
		if len > 0 {
			unsafe { f.write_str(from_utf8_unchecked(&buf), len as usize) }
		} else {
			Err(Error::new(IO))
		}
	}
}

impl Display for bool {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		if *self {
			f.write_str("true", 4)
		} else {
			f.write_str("false", 5)
		}
	}
}

impl Display for f32 {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let mut buf = [0u8; 512];
		let len = unsafe { f64_to_str((*self) as f64, buf.as_mut_ptr(), 512) };
		if len > 0 {
			unsafe { f.write_str(from_utf8_unchecked(&buf), len as usize) }
		} else {
			Err(Error::new(IO))
		}
	}
}

impl Display for &str {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		f.write_str(self, self.len())
	}
}

macro_rules! impl_display_array {
    ($($n:expr),*) => {
        $(
            impl<T: Display> Display for [T; $n] {
        fn format(&self, f: &mut Formatter) -> Result<(), Error> {
                let len = self.len();
                writef!(f, "[")?;
                if len > 0 {
                        for i in 0..len {
                                if i != len - 1 {
                                        writef!(f, "{}, ", self[i])?;
                                } else {
                                        writef!(f, "{}", self[i])?;
                                }
                        }
                }
                writef!(f, "]")?;
                Ok(())
        }
}
        )*
    };
}

impl_display_array!(
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
	27, 28, 29, 30, 31, 32, 33, 40, 48, 56, 64, 96, 128, 256
);

impl<T: Display> Display for &[T] {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let len = self.len();
		writef!(f, "[")?;
		if len > 0 {
			for i in 0..len {
				if i != len - 1 {
					writef!(f, "{}, ", self[i])?;
				} else {
					writef!(f, "{}", self[i])?;
				}
			}
		}
		writef!(f, "]")?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::ffi::getalloccount;

	#[test]
	fn test_formatter1() {
		let mut fmt = Formatter::new();
		fmt.write_str("ok1", 3).unwrap();
		fmt.write_str("hi hi hi", 8).unwrap();
		fmt.write_str(" ", 1).unwrap();
		fmt.write_str("7", 1).unwrap();
		assert_eq!(fmt.as_str(), "ok1hi hi hi 7");
		let mut fmt = Formatter::new();
		fmt.write_str("===", 3).unwrap();
		166u64.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		assert_eq!(fmt.as_str(), "===166===");
	}

	#[test]
	fn test_formatter_unsigned() {
		let mut fmt = Formatter::new();
		1234u128.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		1234u64.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		1234u32.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		1234u16.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		123u8.format(&mut fmt).unwrap();
		assert_eq!(fmt.as_str(), "1234===1234===1234===1234===123");
	}

	#[test]
	fn test_formatter_signed() {
		let mut fmt = Formatter::new();
		(-1234i128).format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		(-1234i64).format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		(-1234i32).format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		(-1234i16).format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		(-123i8).format(&mut fmt).unwrap();
		assert_eq!(fmt.as_str(), "-1234===-1234===-1234===-1234===-123");

		let mut fmt = Formatter::new();
		1234i128.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		1234i64.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		1234i32.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		1234i16.format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		123i8.format(&mut fmt).unwrap();
		assert_eq!(fmt.as_str(), "1234===1234===1234===1234===123");
	}

	#[test]
	fn test_float() {
		let mut fmt = Formatter::new();
		(-123.456f64).format(&mut fmt).unwrap();
		fmt.write_str("===", 3).unwrap();
		(123.1f32).format(&mut fmt).unwrap();

		assert_eq!(fmt.as_str(), "-123.45600===123.10000");
	}

	#[test]
	fn test_format() {
		let init = unsafe { getalloccount() };
		{
			let mut f = Formatter::new();
			writef!(
				&mut f,
				"test {} {} {} {} {} {} {} end",
				1,
				-2,
				3,
				4.5,
				true,
				"ok",
				String::new("xyz").unwrap()
			)
			.unwrap();
			assert_eq!(f.as_str(), "test 1 -2 3 4.50000 true ok xyz end");

			let x = format!("this is a test {} {}", 7, 8).unwrap();
			assert_eq!(x.to_str(), "this is a test 7 8");
		}
		assert_eq!(init, unsafe { getalloccount() });
	}

	#[test]
	fn test_grow_formatter() -> Result<(), Error> {
		let init = unsafe { getalloccount() };
		{
			let mut f = Formatter::with_capacity(64)?;
			writef!(&mut f, "abc")?;
			writef!(&mut f, "def")?;
			let x = 101;
			writef!(&mut f, "{}", x)?;

			assert_eq!(f.as_str(), "abcdef101");
		}
		assert_eq!(init, unsafe { getalloccount() });

		Ok(())
	}
}
