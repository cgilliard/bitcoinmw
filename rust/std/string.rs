use core::cmp::PartialEq;
use core::convert::AsRef;
use core::fmt::Debug;
use core::fmt::Formatter as CoreFormatter;
use core::ptr::null;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;
use std::misc::{is_utf8_valid, slice_copy, strcmp, subslice};

pub struct String {
	value: Option<Rc<Box<[u8]>>>,
	end: usize,
	start: usize,
}

impl Debug for String {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> Result<(), FmtError> {
		// no_std compatiibility (cannot write but works for tests)
		#[cfg(test)]
		write!(_f, "{}", self.to_str())?;
		Ok(())
	}
}

impl Display for String {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		f.write_str(self.to_str(), self.len())
	}
}

impl PartialEq for String {
	fn eq(&self, other: &String) -> bool {
		strcmp(self.to_str(), other.to_str()) == 0
	}
}

impl AsRef<[u8]> for String {
	fn as_ref(&self) -> &[u8] {
		self.to_str().as_ref()
	}
}

impl Clone for String {
	fn clone(&self) -> Self {
		match &self.value {
			Some(value) => Self {
				value: Some(value.clone()),
				start: self.start,
				end: self.end,
			},
			None => Self::empty(),
		}
	}
}

impl String {
	pub fn new(s: &str) -> Result<Self, Error> {
		let end = s.len();
		let start = 0;
		if end == 0 {
			Ok(String::empty())
		} else {
			match try_box_slice!(0u8, end) {
				Ok(mut value) => {
					slice_copy(s.as_bytes(), &mut value, end)?;
					match Rc::new(value) {
						Ok(rc) => Ok(Self {
							value: Some(rc),
							start,
							end,
						}),
						Err(e) => Err(e),
					}
				}
				Err(e) => Err(e),
			}
		}
	}

	pub fn newb(b: &[u8]) -> Result<Self, Error> {
		is_utf8_valid(b)?;

		let end = b.len();
		let start = 0;
		if end == 0 {
			Ok(String::empty())
		} else {
			match try_box_slice!(0u8, end) {
				Ok(mut value) => {
					slice_copy(b, &mut value, end)?;
					match Rc::new(value) {
						Ok(rc) => Ok(Self {
							value: Some(rc),
							start,
							end,
						}),
						Err(e) => Err(e),
					}
				}
				Err(e) => Err(e),
			}
		}
	}

	pub fn empty() -> Self {
		Self {
			value: None,
			start: 0,
			end: 0,
		}
	}

	pub fn as_ptr(&self) -> *const u8 {
		match &self.value {
			Some(value) => value.get().as_ptr().raw() as *const u8,
			None => null(),
		}
	}

	pub fn to_str(&self) -> &str {
		match &self.value {
			Some(value) => {
				let b = value.get().as_ref();
				match subslice(b, self.start, self.end - self.start) {
					Ok(b) => unsafe { from_utf8_unchecked(b) },
					Err(_) => "", // not possible
				}
			}
			None => "",
		}
	}

	pub fn substring(&self, start: usize, end: usize) -> Result<Self, Error> {
		if start > end || end - start > self.len() {
			Err(Error::new(OutOfBounds))
		} else if start == end {
			Ok(Self {
				value: None,
				start: 0,
				end: 0,
			})
		} else {
			Ok(Self {
				value: self.value.clone(),
				start: start + self.start,
				end: self.start + end,
			})
		}
	}

	pub fn len(&self) -> usize {
		self.end - self.start
	}

	pub fn findn(&self, s: &str, offset: usize) -> Option<usize> {
		let self_str = self.to_str();
		let s_len = s.len();
		if s_len == 0 {
			return Some(offset);
		}

		let initial_len = self_str.len().saturating_sub(offset);
		if initial_len < s_len {
			return None;
		}

		let mut len = initial_len;
		let mut x = unsafe { self_str.as_ptr().add(offset) };
		while len >= s_len {
			let slice = unsafe { from_raw_parts(x, s_len) };
			let v = unsafe { from_utf8_unchecked(slice) };
			if v == s {
				return Some(self_str.len() - len);
			}
			len -= 1;
			x = unsafe { x.add(1) };
		}

		None
	}

	pub fn find(&self, s: &str) -> Option<usize> {
		self.findn(s, 0)
	}

	pub fn rfindn(&self, s: &str, offset: usize) -> Option<usize> {
		let self_str = self.to_str();
		let self_str_len = self_str.len();
		let s_len = s.len();
		if s_len == 0 {
			if offset < self_str_len {
				return Some(offset);
			} else {
				return Some(self_str_len);
			}
		}

		let offset = if offset < self_str_len {
			offset
		} else {
			self_str_len
		};

		let search_len = offset + 1;
		if search_len < s_len {
			return None;
		}

		let max_start = offset - (s_len - 1);
		let mut current_start = max_start;
		while current_start <= max_start {
			let x = unsafe { self_str.as_ptr().add(current_start) };
			let slice = unsafe { from_raw_parts(x, s_len) };
			let v = unsafe { from_utf8_unchecked(slice) };
			if v == s {
				return Some(current_start);
			}
			if current_start == 0 {
				break;
			}
			current_start -= 1;
		}
		None
	}

	pub fn rfind(&self, s: &str) -> Option<usize> {
		self.rfindn(s, self.len())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::ffi::getalloccount;

	#[test]
	fn test_strings() {
		let initial = unsafe { getalloccount() };
		{
			let x1 = String::new("abcdefghijkl").unwrap();
			assert_eq!(x1.len(), 12);
			assert_eq!(x1.to_str(), "abcdefghijkl");
			assert_eq!(x1.substring(3, 6).unwrap().to_str(), "def");
			let x2 = x1.substring(3, 9).unwrap();
			assert_eq!(x2.to_str(), "defghi");
			assert_eq!(x2, String::new("defghi").unwrap());
			assert_eq!(x1, String::new("abcdefghijkl").unwrap());
			assert_eq!(x1.find("bc"), Some(1));
			assert_eq!(x1.find("aa"), None);
			assert_eq!(x1.find(""), Some(0));
			let x2 = String::new("").unwrap();
			assert_eq!(x2.len(), 0);
			let x3 = String::new("aaabbbcccaaa").unwrap();
			assert_eq!(x3.rfind("aaa"), Some(9));
			assert_eq!(x3.rfind("ajlsfdjklasdjlfalsjkdfjklasdf"), None);
			assert_eq!(x3.rfind("aaaa"), None);
			assert_eq!(x3.find("ajlsfdjklasdjlfalsjkdfjklasdf"), None);
			let x4 = String::new("0123456789012345678901234567890123456789").unwrap();
			assert_eq!(x4.find("012"), Some(0));

			let x5 = x4.clone();
			assert_eq!(x5.find("012"), Some(0));
			assert_eq!(x5.rfind("012"), Some(30));

			let x6 = x5.substring(5, 15).unwrap();
			let x7 = x6.to_str().as_bytes();
			assert_eq!(x7.len(), 10);
			assert_eq!(x7[0], b'5');
			let x8 = x5.substring(6, 6).unwrap();
			assert_eq!(x8.len(), 0);

			let x9 = match String::new("test") {
				Ok(s) => s,
				Err(_e) => String::new("").unwrap(),
			};
			assert_eq!(x9.len(), 4);
		}

		unsafe {
			assert_eq!(initial, getalloccount());
		}
	}

	#[test]
	fn test_other_strings() -> Result<(), Error> {
		let s = String::new("")?;
		assert_eq!(s, String::empty());

		let s = String::new("test {} {} {}")?;
		assert_eq!(s.findn("{}", 0).unwrap(), 5);
		assert_eq!(s.findn("{}", 5).unwrap(), 5);
		assert_eq!(s.findn("{}", 6).unwrap(), 8);
		assert_eq!(s.findn("{}", 7).unwrap(), 8);
		assert_eq!(s.findn("{}", 8).unwrap(), 8);
		assert_eq!(s.findn("{}", 9).unwrap(), 11);
		assert_eq!(s.findn("{}", 11).unwrap(), 11);
		assert_eq!(s.findn("{}", 12), None);

		assert_eq!(s.rfindn("{}", 12).unwrap(), 11);
		assert_eq!(s.rfindn("{}", 14).unwrap(), 11);
		assert_eq!(s.rfindn("{}", 11).unwrap(), 8);
		assert_eq!(s.rfindn("{}", 10).unwrap(), 8);
		assert_eq!(s.rfindn("{}", 9).unwrap(), 8);
		assert_eq!(s.rfindn("{}", 8).unwrap(), 5);
		assert_eq!(s.rfindn("{}", 7).unwrap(), 5);
		assert_eq!(s.rfindn("{}", 6).unwrap(), 5);
		assert_eq!(s.rfindn("{}", 5), None);
		assert_eq!(s.rfindn("{}", 4), None);
		assert_eq!(s.rfindn("{}", 3), None);
		assert_eq!(s.rfindn("{}", 2), None);
		assert_eq!(s.rfindn("{}", 1), None);
		assert_eq!(s.rfindn("{}", 0), None);

		let s = String::new("test {} {} {} ")?;
		assert_eq!(s.findn("{}", 11).unwrap(), 11);
		assert_eq!(s.findn("{}", 12), None);

		let s2 = String::new("0123456789012345678901234567890123456789").unwrap();
		let x5 = s2.clone();

		assert_eq!(s2.find("012"), Some(0));
		assert_eq!(x5.rfind("012"), Some(30));

		Ok(())
	}
}
