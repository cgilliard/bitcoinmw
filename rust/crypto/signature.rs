use prelude::*;
use std::misc::bytes_to_hex_64;

#[repr(C)]
#[derive(Clone)]
pub struct Signature([u8; 64]);

#[repr(C)]
pub struct Message([u8; 32]);

impl Display for Signature {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
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
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl Signature {
	pub fn new() -> Self {
		Self([0u8; 64])
	}
}

impl AsRaw<Self> for Message {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl Message {
	pub fn new(v: [u8; 32]) -> Self {
		Self(v)
	}
}
