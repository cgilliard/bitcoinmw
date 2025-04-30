/*use crypto::cpsrng::Cpsrng;
use prelude::*;
use std::ffi::getmicros;
use std::misc::from_le_bytes_u64;

fn exec() -> Result<()> {
	let start = unsafe { getmicros() };
	let mut v = Vec::new();
	let cpsrng = Cpsrng::new()?;
	for _i in 0..100_000 {
		let mut bytes = [0u8; 8];
		cpsrng.gen(&mut bytes);
		let vx = from_le_bytes_u64(&bytes)?;
		v.push(vx)?;
	}
	let diff = unsafe { getmicros() } - start;
	println!("len={},diff={}", v.len(), diff);
	Ok(())
}
*/

#[no_mangle]
pub extern "C" fn real_main(argc: i32, _argv: *const *const u8) -> i32 {
	if argc != 0 {
		/*
		match exec() {
			Ok(_) => {}
			Err(e) => println!("Exec err: {}", e),
		}
			*/
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
