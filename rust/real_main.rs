use ffi::*;

#[no_mangle]
pub extern "C" fn real_main(_argc: i32, _argv: *const *const u8) -> i32 {
	let ctx = unsafe { secp256k1_context_create(SECP256K1_START_SIGN) };
	unsafe {
		secp256k1_context_destroy(ctx);
	}

	0
}

#[cfg(test)]
mod test {
	use super::*;
	use core::ptr::null;

	#[test]
	fn test_real_main() {
		assert_eq!(real_main(0, null()), 0);
	}
}
