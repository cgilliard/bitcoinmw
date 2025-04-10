use core::ptr::copy_nonoverlapping;
use core::sync::atomic::{compiler_fence, Ordering};
use crypto::constants::SECP256K1_EC_COMPRESSED;
use crypto::constants::ZERO_KEY;
use crypto::ctx::Ctx;
use crypto::ffi::*;
use prelude::*;

#[repr(C)]
pub struct Message(pub(crate) [u8; 32]);
#[repr(C)]
#[derive(Clone)]
pub struct PublicKey(pub(crate) [u8; 33]);
#[repr(C)]
#[derive(Clone)]
pub struct SecretKey(pub(crate) [u8; 32]);
#[repr(C)]
#[derive(Clone)]
pub struct Signature(pub(crate) [u8; 64]);
#[repr(C)]
pub struct PublicKeyUncompressed(pub(crate) [u8; 64]);

impl Message {
	pub(crate) fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
}

impl PublicKeyUncompressed {
	pub(crate) fn as_mut_ptr(&mut self) -> *mut PublicKeyUncompressed {
		self.0.as_mut_ptr() as *mut PublicKeyUncompressed
	}

	pub(crate) fn as_ptr(&self) -> *const PublicKeyUncompressed {
		self.0.as_ptr() as *const PublicKeyUncompressed
	}
}

impl Drop for SecretKey {
	fn drop(&mut self) {
		unsafe {
			copy_nonoverlapping(ZERO_KEY.as_ptr(), self.0.as_mut_ptr(), 32);
			compiler_fence(Ordering::SeqCst);
		}
	}
}

impl SecretKey {
	pub fn new(ctx: &mut Ctx) -> Self {
		let mut v = [0u8; 32];
		loop {
			unsafe {
				ctx.rand.gen(&mut v);
				let valid = secp256k1_ec_seckey_verify(ctx.secp, v.as_ptr() as *const SecretKey);
				if valid == 1 {
					break;
				}
			}
		}
		Self(v)
	}

	pub(crate) fn as_ptr(&self) -> *const SecretKey {
		self.0.as_ptr() as *const SecretKey
	}
}

impl Signature {
	pub fn new() -> Self {
		Self([0u8; 64])
	}
	pub(crate) fn as_mut_ptr(&mut self) -> *mut Self {
		self.0.as_mut_ptr() as *mut Self
	}

	pub(crate) fn as_ptr(&self) -> *const Self {
		self.0.as_ptr() as *const Self
	}
}

impl PublicKey {
	pub fn from(ctx: &Ctx, secret_key: &SecretKey) -> Result<Self, Error> {
		let mut v = Self([0u8; 33]);
		let mut uncomp = PublicKeyUncompressed([0u8; 64]);

		unsafe {
			if secp256k1_ec_pubkey_create(ctx.secp, uncomp.as_mut_ptr(), secret_key.as_ptr()) == 0 {
				return Err(Error::new(PubkeyCreate));
			}

			let mut len = 33usize;
			let serialize_result = secp256k1_ec_pubkey_serialize(
				ctx.secp,
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
				ctx.secp,
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
			if secp256k1_ec_pubkey_parse(ctx.secp, ret.as_mut_ptr(), self.as_ptr(), self.0.len())
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
				ctx.secp,
				uncomp_self.as_mut_ptr(),
				self.as_ptr(),
				self.0.len(),
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Uncompress other
			if secp256k1_ec_pubkey_parse(
				ctx.secp,
				uncomp_other.as_mut_ptr(),
				other.as_ptr(),
				other.0.len(),
			) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Combine uncompressed keys
			let pubkeys = [uncomp_self.as_ptr(), uncomp_other.as_ptr()];
			if secp256k1_ec_pubkey_combine(ctx.secp, result.as_mut_ptr(), pubkeys.as_ptr(), 2) != 1
			{
				return Err(Error::new(InvalidPublicKey));
			}

			// Recompress result
			let mut compressed = Self([0u8; 33]);
			let mut len = 33usize;
			if secp256k1_ec_pubkey_serialize(
				ctx.secp,
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
