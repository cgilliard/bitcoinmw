#[no_mangle]
pub extern "C" fn real_main(_argc: i32, _argv: *const *const u8) -> i32 {
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
