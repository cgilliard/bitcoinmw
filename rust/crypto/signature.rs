use prelude::*;
use std::misc::bytes_to_hex_64;

#[repr(C)]
#[derive(Clone)]
pub struct Signature([u8; 64]);

#[repr(C)]
#[derive(PartialEq, Clone, Copy)]
pub struct Message([u8; 32]);

impl PartialEq for Signature {
	fn eq(&self, other: &Signature) -> bool {
		for i in 0..64 {
			if self.0[i] != other.0[i] {
				return false;
			}
		}
		true
	}
}

impl PartialOrd for Signature {
	fn partial_cmp(&self, other: &Signature) -> Option<Ordering> {
		let len = self.0.len();
		for i in 0..len {
			if self.0[i] < other.0[i] {
				return Some(Ordering::Less);
			} else if self.0[i] > other.0[i] {
				return Some(Ordering::Greater);
			}
		}
		Some(Ordering::Equal)
	}
}

impl Eq for Signature {}

impl Ord for Signature {
	fn cmp(&self, other: &Self) -> Ordering {
		let len = self.0.len();
		for i in 0..len {
			if self.0[i] < other.0[i] {
				return Ordering::Less;
			} else if self.0[i] > other.0[i] {
				return Ordering::Greater;
			}
		}
		Ordering::Equal
	}
}

impl Hash for Signature {
	fn hash<H>(&self, hasher: &mut H)
	where
		H: Hasher,
	{
		self.0.hash(hasher);
	}
}

impl Display for Signature {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		let b = bytes_to_hex_64(&self.0);
		for i in 0..128 {
			writef!(f, "{}", b[i] as char)?;
		}
		Ok(())
	}
}

impl AsRaw<Self> for Signature {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
}
impl AsRawMut<Self> for Signature {
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl Signature {
	pub fn new() -> Self {
		Self([0u8; 64])
	}
}

#[cfg(test)]
mod test_debug {
	use super::*;
	impl Debug for Message {
		fn fmt(&self, f: &mut CoreFormatter<'_>) -> FmtResult {
			write!(f, "{:?}", self.0)
		}
	}
}

impl AsRef<[u8]> for Message {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl AsRaw<Self> for Message {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
}
impl AsRawMut<Self> for Message {
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl Message {
	pub fn new(v: [u8; 32]) -> Self {
		Self(v)
	}

	pub fn zero() -> Self {
		Self([0u8; 32])
	}
}
