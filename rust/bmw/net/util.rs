use core::ptr::copy_nonoverlapping;
use core::str::from_utf8_unchecked;
use net::constants::WEBSOCKET_MAGIC_STRING;
use net::ffi::{sha1, Base64encode};
use prelude::*;

pub fn websocket_accept_key(sec_key: &str) -> Result<String> {
	let sec_key = sec_key.as_bytes();
	let mut sha1_result: [u8; 20] = [0; 20];
	let mut combined: [u8; 60] = [0; 60];

	unsafe {
		copy_nonoverlapping(sec_key.as_ptr(), combined.as_mut_ptr(), sec_key.len());

		copy_nonoverlapping(
			WEBSOCKET_MAGIC_STRING.as_ptr(),
			combined.as_mut_ptr().add(sec_key.len()),
			WEBSOCKET_MAGIC_STRING.len(),
		);
		sha1(combined.as_ptr(), combined.len(), sha1_result.as_mut_ptr());

		let mut accept_key: [u8; 28] = [0; 28];
		Base64encode(accept_key.as_mut_ptr(), sha1_result.as_mut_ptr(), 20);

		String::new(from_utf8_unchecked(&accept_key))
	}
}
