use bible::Bible;
use prelude::*;

errors!(CapacityExceeded, ArrayIndexOutOfBounds, Timeout);

fn test_err1(x: u32) -> ResultGen<()> {
	if x == 1 {
		err!(Timeout)
	} else {
		Ok(())
	}
}

fn test_err2(x: u32) -> ResultGen<()> {
	if x == 1 {
		Ok(())
	} else {
		err!(CapacityExceeded)
	}
}

fn test_err3(x: u32) -> ResultGen<()> {
	if x == 1 {
		return err!(ArrayIndexOutOfBounds);
	}
	Ok(())
}

#[no_mangle]
pub extern "C" fn real_main(argc: i32, _argv: *const *const u8) -> i32 {
	let bible = Bible::new();
	let verse = bible.find_mod(0);
	if argc > 0 {
		println!("{}", verse);
		match test_err1(1) {
			Ok(_) => {}
			Err(e) => println!("e1={}", e),
		}
		match test_err2(1) {
			Ok(_) => {}
			Err(e) => println!("e2={}", e),
		}
		match test_err3(2) {
			Ok(_) => {}
			Err(e) => println!("e3={}", e),
		}
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
