use prelude::*;

fn exec() -> Result<()> {
	let s = String::new("0123456789")?;
	println!("s='{}'", &s[..]);
	let mut v = vec![1, 2, 3, 4, 5, 6, 7, 8]?;
	v[3] = 9;
	println!("v={}", &v[5..]);
	Ok(())
}
#[no_mangle]
pub extern "C" fn real_main(argc: i32, _argv: *const *const u8) -> i32 {
	if argc != 0 {
		match exec() {
			Ok(_) => {}
			Err(e) => println!("Exec err: {}", e),
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
