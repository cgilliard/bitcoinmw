#[allow(dead_code)]
extern "C" {
    pub fn write(fd: i32, s: *const u8, len: i32);
    pub fn sleep(millis: u64);
}

#[cfg(test)]
mod test {
    #[test]
    fn test_ffi_basic() {
        assert_eq!(2, 1);
    }
}
