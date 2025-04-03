use ffi::{sleep, write};

#[no_mangle]
pub extern "C" fn real_main(_argc: i32, _argv: *const *const u8) -> i32 {
    unsafe {
        write(2, "Running bitcoinmw...\n".as_ptr(), 21);
        sleep(60);
    }
    0
}
