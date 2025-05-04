extern crate base;
extern crate macros;
use self::base::Error;
use self::macros::Dummy;
use core::cmp::PartialEq;

#[derive(Dummy)]
struct MyStruct {
	x: u32,
	y: u64,
}

#[no_mangle]
pub extern "C" fn real_main(_argc: i32, _argv: *const *const u8) -> i32 {
	let _err = Error { code: 2 };
	let _ms = MyStruct { x: 1, y: 2 };
	let _v = _ms.x;
	let _y = _ms.y;
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
