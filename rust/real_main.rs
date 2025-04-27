use bible::Bible;
use prelude::*;

errors!(CapacityExceeded, ArrayIndexOutOfBounds, Timeout);

fn test_err() -> ResultGen<()> {
	Err(err!(Timeout))
}

#[no_mangle]
pub extern "C" fn real_main(argc: i32, _argv: *const *const u8) -> i32 {
	let bible = Bible::new();
	let verse = bible.find_mod(0);
	if argc > 0 {
		println!("{}", verse);
		match test_err() {
			Ok(_) => {}
			Err(e1) => println!("e1={}", e1),
		};
		let e2 = err!(CapacityExceeded);
		println!("e2={}", e2);
		let e3 = err!(ArrayIndexOutOfBounds);
		println!("e3={}", e3);
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
