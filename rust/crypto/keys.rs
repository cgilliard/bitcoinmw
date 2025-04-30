use core::convert::{AsMut, AsRef};
use core::ptr::write_bytes;
use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering::SeqCst;
use crypto::constants::SECP256K1_EC_COMPRESSED;
use crypto::ctx::Ctx;
use crypto::errors::*;
use crypto::ffi::{
	secp256k1_ec_privkey_negate, secp256k1_ec_pubkey_combine, secp256k1_ec_pubkey_create,
	secp256k1_ec_pubkey_parse, secp256k1_ec_pubkey_serialize, secp256k1_ec_seckey_verify,
};
use prelude::*;

#[repr(C)]
#[derive(Clone)]
pub struct PublicKey([u8; 33]);
#[repr(C)]
#[derive(Clone)]
pub struct SecretKey([u8; 32]);
#[repr(C)]
#[derive(Clone)]
pub struct PublicKeyUncompressed([u8; 64]);

impl Debug for SecretKey {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> FmtResult {
		#[cfg(test)]
		write!(_f, "SecretKey[..]")?;
		Ok(())
	}
}

impl PartialEq for SecretKey {
	fn eq(&self, other: &SecretKey) -> bool {
		self.0 == other.0
	}
}

impl Drop for SecretKey {
	fn drop(&mut self) {
		unsafe {
			write_bytes(self.0.as_mut_ptr(), 0, 32);
			compiler_fence(SeqCst);
		}
	}
}

impl AsRaw<Self> for SecretKey {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
}
impl AsRawMut<Self> for SecretKey {
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl AsMut<[u8]> for SecretKey {
	fn as_mut(&mut self) -> &mut [u8] {
		&mut self.0
	}
}

impl AsRef<[u8]> for SecretKey {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

impl SecretKey {
	pub fn zero() -> Self {
		Self([0u8; 32])
	}

	pub fn gen(ctx: &Ctx) -> Self {
		let mut v = Self::zero();
		loop {
			ctx.gen(&mut v.0);
			if unsafe { secp256k1_ec_seckey_verify(ctx.as_ptr(), v.as_ptr()) == 1 } {
				break;
			}
		}
		v
	}

	pub fn negate(&mut self, ctx: &Ctx) -> Result<()> {
		if unsafe { secp256k1_ec_privkey_negate(ctx.as_ptr(), self.as_mut_ptr()) == 0 } {
			err!(OperationFailed)
		} else {
			Ok(())
		}
	}

	pub fn validate(&self, ctx: &Ctx) -> Result<()> {
		if unsafe { secp256k1_ec_seckey_verify(ctx.as_ptr(), self.as_ptr()) != 1 } {
			err!(OperationFailed)
		} else {
			Ok(())
		}
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

impl AsRaw<Self> for PublicKeyUncompressed {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
}

impl AsRawMut<Self> for PublicKeyUncompressed {
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl PublicKeyUncompressed {
	pub const fn new(v: [u8; 64]) -> Self {
		Self(v)
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

impl AsRaw<Self> for PublicKey {
	fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
}

impl AsRawMut<Self> for PublicKey {
	fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}
}

impl PublicKey {
	pub fn from(ctx: &Ctx, secret_key: &SecretKey) -> Result<Self> {
		let mut v = Self([0u8; 33]);
		let mut uncomp = PublicKeyUncompressed([0u8; 64]);

		unsafe {
			if secp256k1_ec_pubkey_create(ctx.as_ptr(), uncomp.as_mut_ptr(), secret_key.as_ptr())
				== 0
			{
				return err!(OperationFailed);
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
				err!(Serialization)
			} else {
				Ok(v)
			}
		}
	}

	pub fn compress(ctx: &Ctx, key: PublicKeyUncompressed) -> Result<Self> {
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
			err!(Serialization)
		} else {
			Ok(v)
		}
	}

	pub fn decompress(&self, ctx: &Ctx) -> Result<PublicKeyUncompressed> {
		let mut ret = PublicKeyUncompressed([0u8; 64]);
		unsafe {
			if secp256k1_ec_pubkey_parse(
				ctx.as_ptr(),
				ret.as_mut_ptr(),
				self.as_ptr(),
				self.0.len(),
			) != 1
			{
				return err!(OperationFailed);
			}
		}
		Ok(ret)
	}

	pub fn combine(&self, ctx: &Ctx, other: &PublicKey) -> Result<Self> {
		let mut result = PublicKeyUncompressed([0u8; 64]);
		let mut uncomp_self = PublicKeyUncompressed([0u8; 64]);
		let mut uncomp_other = PublicKeyUncompressed([0u8; 64]);

		unsafe {
			// Uncompress self
			if secp256k1_ec_pubkey_parse(
				ctx.as_ptr(),
				uncomp_self.as_mut_ptr(),
				self.as_ptr(),
				self.0.len(),
			) != 1
			{
				return err!(IllegalArgument);
			}

			// Uncompress other
			if secp256k1_ec_pubkey_parse(
				ctx.as_ptr(),
				uncomp_other.as_mut_ptr(),
				other.as_ptr(),
				other.0.len(),
			) != 1
			{
				return err!(IllegalArgument);
			}

			// Combine uncompressed keys
			let pubkeys = [uncomp_self.as_ptr(), uncomp_other.as_ptr()];
			if secp256k1_ec_pubkey_combine(ctx.as_ptr(), result.as_mut_ptr(), pubkeys.as_ptr(), 2)
				!= 1
			{
				return err!(OperationFailed);
			}

			// Recompress result
			let mut compressed = Self([0u8; 33]);
			let mut len = 33usize;
			if secp256k1_ec_pubkey_serialize(
				ctx.as_ptr(),
				compressed.as_mut_ptr(),
				&mut len,
				result.as_ptr(),
				SECP256K1_EC_COMPRESSED,
			) != 1
			{
				return err!(Serialization);
			}
			Ok(compressed)
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_secret_key() -> Result<()> {
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
