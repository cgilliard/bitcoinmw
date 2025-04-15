use core::ptr::copy_nonoverlapping;
use core::sync::atomic::{compiler_fence, Ordering::SeqCst};
use crypto::constants::SECP256K1_EC_COMPRESSED;
use crypto::constants::ZERO_KEY;
use crypto::ctx::Ctx;
use crypto::ffi::*;
use prelude::*;
use std::misc::bytes_to_hex_64;

#[repr(C)]
pub struct Message([u8; 32]);
#[repr(C)]
#[derive(Clone)]
pub struct PublicKey([u8; 33]);
#[repr(C)]
#[derive(Clone)]
pub struct SecretKey([u8; 32]);
#[repr(C)]
#[derive(Clone)]
pub struct Signature([u8; 64]);
#[repr(C)]
pub struct PublicKeyUncompressed([u8; 64]);

impl Display for Signature {
	fn format(&self, f: &mut Formatter) -> Result<(), Error> {
		let b = bytes_to_hex_64(&self.0);
		for i in 0..128 {
			writeb!(f, "{}", b[i] as char)?;
		}
		Ok(())
	}
}

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

impl Message {
	pub fn new(v: [u8; 32]) -> Self {
		Self(v)
	}
	pub fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
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

impl Drop for SecretKey {
	fn drop(&mut self) {
		unsafe {
			copy_nonoverlapping(ZERO_KEY.as_ptr(), self.0.as_mut_ptr(), 32);
			compiler_fence(SeqCst);
		}
	}
}

impl SecretKey {
	pub fn new(v: [u8; 32]) -> Self {
		Self(v)
	}

	pub fn gen(ctx: &Ctx) -> Self {
		let mut v = [0u8; 32];
		loop {
			unsafe {
				ctx.rand().gen(&mut v);
				let valid = secp256k1_ec_seckey_verify(ctx.secp(), v.as_ptr() as *const SecretKey);
				if valid == 1 {
					break;
				}
			}
		}
		Self(v)
	}

	pub fn negate(&mut self, ctx: &mut Ctx) -> Result<(), Error> {
		unsafe {
			if secp256k1_ec_privkey_negate(ctx.secp(), self.as_mut_ptr()) == 0 {
				Err(Error::new(InvalidSecretKey))
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
}

impl Signature {
	pub fn new() -> Self {
		Self([0u8; 64])
	}
	pub fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}

	pub fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}

	pub fn as_ref(&self) -> &[u8; 64] {
		&self.0
	}
}

impl PublicKey {
	pub fn from(ctx: &Ctx, secret_key: &SecretKey) -> Result<Self, Error> {
		let mut v = Self([0u8; 33]);
		let mut uncomp = PublicKeyUncompressed([0u8; 64]);

		unsafe {
			if secp256k1_ec_pubkey_create(ctx.secp(), uncomp.as_mut_ptr(), secret_key.as_ptr()) == 0
			{
				return Err(Error::new(PubkeyCreate));
			}

			let mut len = 33usize;
			let serialize_result = secp256k1_ec_pubkey_serialize(
				ctx.secp(),
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
				ctx.secp(),
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

	pub fn decompress(&self, ctx: &Ctx) -> Result<PublicKeyUncompressed, Error> {
		let mut ret = PublicKeyUncompressed([0u8; 64]);
		unsafe {
			if secp256k1_ec_pubkey_parse(ctx.secp(), ret.as_mut_ptr(), self.as_ptr(), self.0.len())
				!= 1
			{
				return Err(Error::new(InvalidPublicKey));
			}
		}
		Ok(ret)
	}

	pub fn add(&self, ctx: &Ctx, other: &PublicKey) -> Result<Self, Error> {
		let mut result = PublicKeyUncompressed([0u8; 64]);
		let mut uncomp_self = PublicKeyUncompressed([0u8; 64]);
		let mut uncomp_other = PublicKeyUncompressed([0u8; 64]);

		// Uncompress self
		unsafe {
			if secp256k1_ec_pubkey_parse(
				ctx.secp(),
				uncomp_self.as_mut_ptr(),
				self.as_ptr(),
				self.0.len(),
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Uncompress other
			if secp256k1_ec_pubkey_parse(
				ctx.secp(),
				uncomp_other.as_mut_ptr(),
				other.as_ptr(),
				other.0.len(),
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Combine uncompressed keys
			let pubkeys = [uncomp_self.as_ptr(), uncomp_other.as_ptr()];
			if secp256k1_ec_pubkey_combine(ctx.secp(), result.as_mut_ptr(), pubkeys.as_ptr(), 2)
				!= 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Recompress result
			let mut compressed = Self([0u8; 33]);
			let mut len = 33usize;
			if secp256k1_ec_pubkey_serialize(
				ctx.secp(),
				compressed.as_mut_ptr(),
				&mut len,
				result.as_ptr(),
				SECP256K1_EC_COMPRESSED,
			) != 1
			{
				return Err(Error::new(Serialization));
			}
			Ok(compressed)
		}
	}

	fn as_mut_ptr(&mut self) -> *mut PublicKey {
		self.0.as_mut_ptr() as *mut PublicKey
	}

	fn as_ptr(&self) -> *const PublicKey {
		self.0.as_ptr() as *const PublicKey
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_negate() -> Result<(), Error> {
		let mut ctx = Ctx::new()?;
		let sk1 = SecretKey::gen(&mut ctx);
		let mut sk2 = sk1.clone();
		assert!(sk1.0 == sk2.0);
		sk2.negate(&mut ctx)?;
		assert!(sk1.0 != sk2.0);
		sk2.negate(&mut ctx)?;
		assert!(sk1.0 == sk2.0);
		Ok(())
	}
}
