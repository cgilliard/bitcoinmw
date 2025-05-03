use prelude::*;

fn do_exec() -> Result<()> {
	let mut v = vec![9, 7, 8, 2, 10]?;
	let _ = &mut v[..].quicksort();
	println!("v={}", v);
	Ok(())
}

#[no_mangle]
pub extern "C" fn real_main(_argc: i32, _argv: *const *const u8) -> i32 {
	let _ = do_exec();
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
