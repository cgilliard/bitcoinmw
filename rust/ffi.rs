#[allow(dead_code)]
extern "C" {
    pub fn write(fd: i32, s: *const u8, len: i32);
    pub fn sleep(millis: u64);
}
