use core::ptr::write_volatile;
use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering::SeqCst;
use crypto::constants::SECP256K1_EC_COMPRESSED;
use crypto::ctx::Ctx;
use crypto::ffi::{
	secp256k1_ec_privkey_negate, secp256k1_ec_pubkey_create, secp256k1_ec_pubkey_serialize,
	secp256k1_ec_seckey_verify,
};
use prelude::*;

#[repr(C)]
#[derive(Clone)]
pub struct PublicKey([u8; 33]);
#[repr(C)]
#[derive(Clone, PartialEq)]
pub struct SecretKey([u8; 32]);
#[repr(C)]
#[derive(Clone)]
pub struct PublicKeyUncompressed([u8; 64]);

// mrustc does not support Debug for thee types, but we need it
// in tests to do assertions
#[cfg(test)]
impl Debug for SecretKey {
	fn fmt(&self, f: &mut crate::core::fmt::Formatter<'_>) -> Result<(), crate::core::fmt::Error> {
		write!(f, "{:?}", self.0)
	}
}

impl Drop for SecretKey {
	fn drop(&mut self) {
		for b in self.0.iter_mut() {
			unsafe {
				write_volatile(b, 0);
			}
		}
		compiler_fence(SeqCst);
	}
}

impl SecretKey {
	pub fn zero() -> Self {
		Self([0u8; 32])
	}

	pub fn gen(ctx: &Ctx) -> Self {
		let mut v = [0u8; 32];
		loop {
			unsafe {
				ctx.gen(&mut v);
				let valid =
					secp256k1_ec_seckey_verify(ctx.as_ptr(), v.as_ptr() as *const SecretKey);
				if valid == 1 {
					break;
				}
			}
		}
		Self(v)
	}

	pub fn negate(&mut self, ctx: &Ctx) -> Result<(), Error> {
		unsafe {
			if secp256k1_ec_privkey_negate(ctx.as_ptr(), self.as_mut_ptr()) == 0 {
				Err(Error::new(OperationFailed))
			} else {
				Ok(())
			}
		}
	}

	pub fn as_mut_ptr(&mut self) -> *mut SecretKey {
		self.0.as_mut_ptr() as *mut SecretKey
	}

	pub fn as_ptr(&self) -> *const SecretKey {
		self.0.as_ptr() as *const SecretKey
	}

	pub fn as_ref(&self) -> &[u8; 32] {
		&self.0
	}

	pub fn as_mut_ref(&mut self) -> &mut [u8; 32] {
		&mut self.0
	}
}

// mrustc compatability since there is implementation for a 64 byte array
impl PartialEq for PublicKeyUncompressed {
	fn eq(&self, other: &Self) -> bool {
		for i in 0..self.0.len() {
			if self.0[i] != other.0[i] {
				return false;
			}
		}
		true
	}
}

impl PublicKeyUncompressed {
	pub const fn new(v: [u8; 64]) -> Self {
		Self(v)
	}

	pub fn as_mut_ptr(&mut self) -> *mut PublicKeyUncompressed {
		self.0.as_mut_ptr() as *mut PublicKeyUncompressed
	}

	pub fn as_ptr(&self) -> *const PublicKeyUncompressed {
		self.0.as_ptr() as *const PublicKeyUncompressed
	}
}

// mrustc compatability since there is implementation for a 33 byte array
impl PartialEq for PublicKey {
	fn eq(&self, other: &Self) -> bool {
		for i in 0..self.0.len() {
			if self.0[i] != other.0[i] {
				return false;
			}
		}
		true
	}
}

impl PublicKey {
	pub fn from(ctx: &Ctx, secret_key: &SecretKey) -> Result<Self, Error> {
		let mut v = Self([0u8; 33]);
		let mut uncomp = PublicKeyUncompressed([0u8; 64]);

		unsafe {
			if secp256k1_ec_pubkey_create(ctx.as_ptr(), uncomp.as_mut_ptr(), secret_key.as_ptr())
				== 0
			{
				return Err(Error::new(OperationFailed));
			}

			let mut len = 33usize;
			let serialize_result = secp256k1_ec_pubkey_serialize(
				ctx.as_ptr(),
				v.as_mut_ptr(),
				&mut len,
				uncomp.as_ptr(),
				SECP256K1_EC_COMPRESSED,
			);
			if serialize_result == 0 {
				Err(Error::new(Serialization))
			} else {
				Ok(v)
			}
		}
	}

	pub fn compress(ctx: &Ctx, key: PublicKeyUncompressed) -> Result<Self, Error> {
		let mut v = Self([0u8; 33]);
		let mut len = 33usize;
		let serialize_result = unsafe {
			secp256k1_ec_pubkey_serialize(
				ctx.as_ptr(),
				v.as_mut_ptr(),
				&mut len,
				key.as_ptr(),
				SECP256K1_EC_COMPRESSED,
			)
		};
		if serialize_result == 0 {
			Err(Error::new(Serialization))
		} else {
			Ok(v)
		}
	}

	pub fn as_ptr(&self) -> *const PublicKey {
		self.0.as_ptr() as *const PublicKey
	}

	pub fn as_mut_ptr(&mut self) -> *mut PublicKey {
		self.0.as_mut_ptr() as *mut PublicKey
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_secret_key() -> Result<(), Error> {
		let ctx = Ctx::new()?;
		let skey1 = SecretKey::zero();
		let skey2 = SecretKey::gen(&ctx);
		let skey3 = SecretKey::zero();
		let skey4 = SecretKey::gen(&ctx);

		assert_ne!(skey1, skey2);
		assert_eq!(skey1, skey3);
		assert_ne!(skey2, skey4);
		Ok(())
	}
}
