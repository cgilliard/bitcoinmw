use bible::Bible;
use prelude::*;

#[no_mangle]
pub extern "C" fn real_main(argc: i32, _argv: *const *const u8) -> i32 {
	let bible = Bible::new();
	let verse = bible.find_mod(0);
	if argc > 0 {
		println!("{}", verse);
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
