#![allow(dead_code)]

/// Flag for context to enable no precomputation
pub const SECP256K1_START_NONE: u32 = (1 << 0) | 0;
/// Flag for context to enable verification precomputation
pub const SECP256K1_START_VERIFY: u32 = (1 << 0) | (1 << 8);
/// Flag for context to enable signing precomputation
pub const SECP256K1_START_SIGN: u32 = (1 << 0) | (1 << 9);
/// Flag for keys to indicate uncompressed serialization format
pub const SECP256K1_SER_UNCOMPRESSED: u32 = (1 << 1) | 0;
/// Flag for keys to indicate compressed serialization format
pub const SECP256K1_SER_COMPRESSED: u32 = (1 << 1) | (1 << 8);

extern "C" {
	pub fn write(fd: i32, s: *const u8, len: i32);
	pub fn sleep(millis: u64);
	pub fn secp256k1_context_create(flags: u32) -> *mut u8;
	pub fn secp256k1_context_destroy(ctx: *mut u8);
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_ffi_basic() {
		let ctx = unsafe { secp256k1_context_create(SECP256K1_START_SIGN) };
		unsafe {
			secp256k1_context_destroy(ctx);
		}
		assert_eq!(1, 1);
	}
}
