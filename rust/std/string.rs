use core::fmt::Debug;
use core::ops::{Index, Range, RangeFrom, RangeFull, RangeTo};
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use prelude::*;
use std::misc::{is_utf8_valid, slice_copy, strcmp, subslice, subslice_mut};
use std::traits::StrExt;

#[derive(Clone)]
pub struct StringDataStruct {
	value: Rc<Box<[u8]>>,
	end: usize,
	start: usize,
}

#[derive(Clone)]
pub struct SSODataStruct([u8; 24]);

#[derive(Clone)]
pub enum String {
	StringData(StringDataStruct),
	SSOData(SSODataStruct),
}

impl Ord for String {
	fn cmp(&self, other: &Self) -> Ordering {
		self.as_str().cmp(other.as_str())
	}
}

impl PartialOrd for String {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.as_str().partial_cmp(other.as_str())
	}
}

impl Eq for String {}

/*
impl Display for String {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		writef!(f, "{}", self.as_str())
	}
}
*/

impl Debug for String {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> FmtResult {
		// no_std compatiibility (cannot write but works for tests)
		#[cfg(test)]
		write!(_f, "{}", self.as_str())?;
		Ok(())
	}
}

impl Index<Range<usize>> for String {
	type Output = str; // Output is a string slice, not a String
	fn index(&self, r: Range<usize>) -> &Self::Output {
		let s = self.as_str();
		let len = s.len();
		if r.start > r.end
			|| r.end > len
			|| !s.is_char_boundary(r.start)
			|| !s.is_char_boundary(r.end)
		{
			exit!(
				"invalid range [{}..{}], len={}, invalid UTF-8 boundaries",
				r.start,
				r.end,
				len
			);
		}
		// Safe to use get_unchecked after validation
		unsafe { s.get_unchecked(r.start..r.end) }
	}
}

impl Index<RangeFrom<usize>> for String {
	type Output = str; // Output is a string slice, not a String
	fn index(&self, r: RangeFrom<usize>) -> &Self::Output {
		let s = self.as_str();
		let len = s.len();
		if r.start > len || !s.is_char_boundary(r.start) {
			exit!(
				"invalid range [{}..], len={}, invalid UTF-8 boundaries",
				r.start,
				len
			);
		}
		// Safe to use get_unchecked after validation
		unsafe { s.get_unchecked(r.start..len) }
	}
}

impl Index<RangeTo<usize>> for String {
	type Output = str; // Output is a string slice, not a String
	fn index(&self, r: RangeTo<usize>) -> &Self::Output {
		let s = self.as_str();
		let len = s.len();
		if r.end > len || !s.is_char_boundary(r.end) {
			exit!(
				"invalid range [..{}], len={}, invalid UTF-8 boundaries",
				r.end,
				len
			);
		}
		// Safe to use get_unchecked after validation
		unsafe { s.get_unchecked(0..r.end) }
	}
}

impl Index<RangeFull> for String {
	type Output = str; // Output is a string slice, not a String
	fn index(&self, _r: RangeFull) -> &Self::Output {
		let s = self.as_str();
		let len = s.len();
		// Safe to use get_unchecked after validation
		unsafe { s.get_unchecked(0..len) }
	}
}

impl PartialEq for String {
	fn eq(&self, other: &String) -> bool {
		strcmp(self.as_str(), other.as_str()) == 0
	}
}

impl AsRef<[u8]> for String {
	fn as_ref(&self) -> &[u8] {
		self.as_str().as_ref()
	}
}

impl String {
	pub fn new(s: &str) -> Result<Self> {
		let end = s.len();
		let start = 0;
		if end <= 23 {
			Ok(Self::sso(s))
		} else {
			match try_box_slice!(0u8, end) {
				Ok(mut value) => {
					slice_copy(s.as_bytes(), &mut value, end)?;
					match Rc::new(value) {
						Ok(rc) => Ok(Self::StringData(StringDataStruct {
							value: rc,
							start,
							end,
						})),
						Err(e) => Err(e),
					}
				}
				Err(e) => Err(e),
			}
		}
	}

	pub fn newb(b: &[u8]) -> Result<Self> {
		is_utf8_valid(b)?;
		unsafe { Self::new(from_utf8_unchecked(b)) }
	}

	pub fn empty() -> Self {
		Self::sso("")
	}

	pub fn as_ptr(&self) -> *const Self {
		self.as_str().as_ptr() as *const Self
	}

	pub fn as_str(&self) -> &str {
		match self {
			String::StringData(StringDataStruct { value, start, end }) => {
				let b = (*value).as_ref();
				match subslice(b, *start, end - *start) {
					Ok(b) => unsafe { from_utf8_unchecked(b) },
					Err(e) => exit!("unexpected error: subslice: {}", e),
				}
			}
			String::SSOData(SSODataStruct(b)) => unsafe {
				let c = from_raw_parts(b.as_ptr().add(1), b[0] as usize);
				from_utf8_unchecked(c)
			},
		}
	}

	pub fn as_bytes(&self) -> &[u8] {
		self.as_str().as_bytes()
	}

	pub fn substring(&self, nstart: usize, nend: usize) -> Result<Self> {
		if nstart > nend || nend - nstart > self.len() {
			return err!(OutOfBounds);
		}
		let s = self.as_str();
		if nend > s.len() {
			return err!(OutOfBounds);
		}
		if !s.is_char_boundary(nstart) || !s.is_char_boundary(nend) {
			return err!(Utf8Error);
		}
		match self {
			String::StringData(StringDataStruct {
				value,
				start: base_start,
				end: base_end,
			}) => {
				let new_start = base_start + nstart;
				let new_end = base_start + nend;
				if new_end > *base_end {
					return err!(OutOfBounds);
				}
				Ok(Self::StringData(StringDataStruct {
					value: value.clone(),
					start: new_start,
					end: new_end,
				}))
			}
			String::SSOData(SSODataStruct(b)) => {
				if nend - nstart <= 23 {
					let mut v = [0u8; 24];
					v[0] = (nend - nstart) as u8;
					unsafe {
						let src = b.as_ptr().offset(1 + nstart as isize);
						let vlen = v.len();
						let subslice = subslice_mut(&mut v, 1, vlen - 1)?;
						slice_copy(from_raw_parts(src, nend - nstart), subslice, nend - nstart)?;
					}
					Ok(Self::SSOData(SSODataStruct(v)))
				} else {
					unsafe {
						let s =
							from_raw_parts(b.as_ptr().offset(1 + nstart as isize), nend - nstart);
						let s = from_utf8_unchecked(s);
						Self::new(s)
					}
				}
			}
		}
	}

	pub fn findn(&self, s: &str, offset: usize) -> Option<usize> {
		let str1 = self.as_str();
		str1.findn(s, offset)
	}

	pub fn find(&self, s: &str) -> Option<usize> {
		self.findn(s, 0)
	}

	pub fn rfindn(&self, s: &str, offset: usize) -> Option<usize> {
		let str1 = self.as_str();
		str1.rfindn(s, offset)
	}

	pub fn rfind(&self, s: &str) -> Option<usize> {
		self.rfindn(s, self.len())
	}

	pub fn len(&self) -> usize {
		match self {
			String::StringData(StringDataStruct {
				value: _,
				start,
				end,
			}) => end - start,
			String::SSOData(b) => b.0[0] as usize,
		}
	}

	fn sso(s: &str) -> Self {
		let len = s.len();
		let mut v = [0u8; 24];
		v[0] = len as u8;
		let _ = slice_copy(&s.as_bytes(), &mut v[1..], len);
		Self::SSOData(SSODataStruct(v))
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_strings2() {
		let x1 = String::new("abcdefghijkl").unwrap();
		assert_eq!(x1.len(), 12);
		assert_eq!(x1.as_str(), "abcdefghijkl");
		assert_eq!(x1.substring(3, 6).unwrap().as_str(), "def");
		let x2 = x1.substring(3, 9).unwrap();
		assert_eq!(x2.as_str(), "defghi");
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
		let x7 = x6.as_str().as_bytes();
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

	#[test]
	fn test_other_strings() -> Result<()> {
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
		assert_eq!(s.rfindn("{}", 13).unwrap(), 11);
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
		assert_eq!(s.rfindn("{}", 14), None); // out of bounds 14

		let s = String::new("test {} {} {} ")?;
		assert_eq!(s.findn("{}", 11).unwrap(), 11);
		assert_eq!(s.findn("{}", 12), None);

		let s2 = String::new("0123456789012345678901234567890123456789").unwrap();
		let x5 = s2.clone();

		assert_eq!(s2.find("012"), Some(0));
		assert_eq!(x5.rfind("012"), Some(30));

		Ok(())
	}

	#[test]
	fn test_string_lens() -> Result<()> {
		// create a base string and check for various lengths to ensure transaction out of sso
		// is ok
		let s_base = "01234567890123456789012345678901234567890123456789";
		unsafe {
			for i in 0..s_base.len() {
				let s_slice = from_raw_parts(s_base.as_ptr(), i);
				let s_x = from_utf8_unchecked(s_slice);
				let s = String::new(s_x)?;
				assert_eq!(s.len(), i);
				assert!(s.find("abc").is_none());
				if i != 0 {
					assert_eq!(s.find("0"), Some(0));
					if i >= 4 {
						assert_eq!(s.find("123"), Some(1));
					}
				}
				assert_eq!(s.find(s_x), Some(0));
			}
		}

		Ok(())
	}

	#[test]
	fn test_chained_substrings() -> Result<()> {
		// create a base string and check for various lengths to ensure transaction out of sso
		// is ok
		let s_base = "01234567890123456789012345678901234567890123456789";
		let s_base_len = s_base.len();
		let s = String::new(s_base)?;
		assert_eq!(s.len(), s_base_len);
		assert_eq!(s.find("1234"), Some(1));

		let s1 = s.substring(1, s.len())?;
		assert_eq!(s1.len(), s_base_len - 1);
		assert_eq!(s1.find("1234"), Some(0));

		let s2 = s1.substring(1, s1.len())?;
		assert_eq!(s2.len(), s_base_len - 2);
		assert_eq!(s2.find("2345"), Some(0));

		let s3 = s2.substring(1, s2.len())?;

		assert_eq!(s3.len(), s_base_len - 3);
		assert_eq!(s3.find("4567"), Some(1));

		assert!(s2.substring(1, s2.len() + 1).is_err());

		assert_eq!(&s[0..10], "0123456789");
		assert_eq!(&s[2..3], "2");

		Ok(())
	}
}
