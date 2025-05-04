#[macro_export]
macro_rules! err {
	($e:expr) => {{
		let mut e = $e;
		e.set_bt(Backtrace::new());
		Err(e)
	}};
}

#[macro_export]
macro_rules! errors {
    ($($error:ident),*) => {
        use misc::simple_hash;
        define_errors_inner!(@count 0, simple_hash(file!(), line!()), $($error),*);
        #[cfg(test)]
        mod unique_test_mod {
            #[test]
            fn print_hash_value() {
                #[cfg(printhashes)]
                {
                    use misc::simple_hash;
                    use prelude::*;
                    println!("hash value = {}", simple_hash(file!(), line!()));
                }
            }
        }
    };
}

#[macro_export]
macro_rules! define_errors_inner {
    (@count $index:expr, $file_hash:expr, $head:ident $(, $tail:ident)*) => {
        #[allow(non_upper_case_globals)]
        pub const $head: Error = Error::new(
            $file_hash + $index,
            || -> &'static str { stringify!($head) },
            Backtrace::init()
        );
        define_errors_inner!(@count $index + 1, $file_hash, $($tail),*);
    };
    (@count $index:expr, $file_hash:expr,) => {};
}

#[macro_export]
macro_rules! nop {
	() => {{
		loop {}
	}};
}

#[macro_export]
macro_rules! box_dyn {
	($value:expr, $trait:path) => {{
		let mut boxed = match Box::new($value) {
			Ok(b) => b,
			Err(e) => exit!("box_dyn failed due to error: {}", e),
		};
		let ptr = boxed.ptr.raw();
		unsafe {
			boxed.leak();
		}
		Box {
			ptr: Ptr::new(ptr as *mut dyn $trait),
		}
	}};
}

#[macro_export]
macro_rules! try_box_dyn {
	($value:expr, $trait:path) => {{
		match Box::new($value) {
			Ok(mut boxed) => {
				let ptr = boxed.ptr.raw();
				unsafe {
					boxed.leak();
				}
				Ok(Box {
					ptr: Ptr::new(ptr as *mut dyn $trait),
				})
			}
			Err(e) => Err(e),
		}
	}};
}

#[macro_export]
macro_rules! box_slice {
	($value:expr, $len:expr) => {{
		use core::mem::size_of_val;
		use core::ptr::{null_mut, write};
		use core::slice::from_raw_parts_mut;
		let count = $len;
		let elem_size = size_of_val(&$value);
		let total_size = elem_size * count;
		let ptr = if total_size == 0 {
			null_mut()
		} else {
			#[allow(unused_unsafe)]
			unsafe {
				let rptr = alloc(total_size) as *mut u8;
				if rptr.is_null() {
					exit!("box_slice failed due to allocation failure!");
				}
				let mut write_ptr = rptr;
				for _ in 0..count {
					write(write_ptr as *mut _, $value);
					write_ptr = write_ptr.add(elem_size);
				}
				rptr as *mut _
			}
		};
		#[allow(unused_unsafe)]
		unsafe {
			Box {
				ptr: Ptr::new(from_raw_parts_mut(ptr as *mut _, count)),
			}
		}
	}};
}

#[macro_export]
macro_rules! try_box_slice {
	($value:expr, $len:expr) => {{
		use core::mem::size_of_val;
		use core::ptr::write;
		use core::slice::from_raw_parts_mut;
		use ffi::alloc;
		let count = $len;
		let elem_size = size_of_val(&$value);
		let total_size = elem_size * count;
		if total_size == 0 {
			err!(IllegalState)
		} else {
			#[allow(unused_unsafe)]
			unsafe {
				let rptr = alloc(total_size) as *mut u8;
				if rptr.is_null() {
					err!(Alloc)
				} else {
					let mut write_ptr = rptr;
					for _ in 0..count {
						write(write_ptr as *mut _, $value);
						write_ptr = write_ptr.add(elem_size);
					}
					Ok(Box::from_raw(Ptr::new(from_raw_parts_mut(
						rptr as *mut _,
						count,
					))))
				}
			}
		}
	}};
}

#[macro_export]
macro_rules! exit {
	($fmt:expr) => {{
		exit!("{}", $fmt);
	}};
	($fmt:expr,  $($t:expr),*) => {{
		print!("Panic[@{}:{}]: ", file!(), line!());
		println!($fmt, $($t),*);
		let bt = Backtrace::new();
		println!("{}", bt);

		#[allow(unused_unsafe)]
		unsafe {
			use ffi::exit;
			exit(-1);
		}
		loop {}
	}};
}

#[macro_export]
macro_rules! aadd {
	($a:expr, $v:expr) => {{
		use ffi::atomic_fetch_add_u64;
		#[allow(unused_unsafe)]
		unsafe {
			atomic_fetch_add_u64($a, $v)
		}
	}};
}

#[macro_export]
macro_rules! asub {
	($a:expr, $v:expr) => {{
		use ffi::atomic_fetch_sub_u64;
		#[allow(unused_unsafe)]
		unsafe {
			atomic_fetch_sub_u64($a, $v)
		}
	}};
}

#[macro_export]
macro_rules! aload {
	($a:expr) => {{
		use ffi::atomic_load_u64;
		#[allow(unused_unsafe)]
		unsafe {
			atomic_load_u64($a)
		}
	}};
}

#[macro_export]
macro_rules! astore {
	($a:expr, $v:expr) => {{
		use ffi::atomic_store_u64;
		#[allow(unused_unsafe)]
		unsafe {
			atomic_store_u64($a, $v)
		}
	}};
}

#[macro_export]
macro_rules! cas {
	($v:expr, $expect:expr, $desired:expr) => {{
		use ffi::cas_release;
		#[allow(unused_unsafe)]
		unsafe {
			cas_release($v, $expect, $desired)
		}
	}};
}

#[macro_export]
macro_rules! vec {
        ($($elem:expr),*) => {{
                let mut vec = Vec::new();
                let mut err: Error = Error::new(
                        Unknown.code(),
                        || -> &'static str { "Unknown" },
                        Backtrace::init(),
                );

                $(
                                if err == Unknown {
                                                match vec.push($elem) {
                                                                Ok(_) => {},
                                                                Err(e) => err = e,
                                                }
                                }
                )*
                if err != Unknown {
                        Err(err)
                } else {
                        Ok(vec)
                }
        }};
}

#[macro_export]
macro_rules! writef {
    ($f:expr, $fmt:expr) => {{
        writef!($f, "{}", $fmt)
    }};
    ($f:expr, $fmt:expr, $($t:expr),*) => {{
        use core::str::from_utf8_unchecked;
        use misc::subslice;

        let mut err = Error::new(Unknown.code(), || { "Unknown" }, Backtrace::init());
        let fmt_str = $fmt;
        let mut cur = 0;
        $(
            match fmt_str.findn("{}", cur) {
                Some(index) => {
                    if index > cur {
                        match subslice(fmt_str.as_bytes(), cur, index - cur) {
                            Ok(bytes) => {
                                #[allow(unused_unsafe)]
                                let s = unsafe { from_utf8_unchecked(bytes) };
                                match $f.write_str(s, s.len()) {
                                    Ok(_) => {},
                                    Err(e) => err = e,
                                }
                            }
                            Err(e) => err = e,
                        }
                    }
                    cur = index + 2;
                    match $t.format($f) {
                        Ok(_) => {},
                        Err(e) => err = e,
                    }
                }
                None => {},
            }
        )*
        if cur < fmt_str.len() {
            match subslice(fmt_str.as_bytes(), cur, fmt_str.len() - cur) {
                Ok(bytes) => {
                    #[allow(unused_unsafe)]
                    let s = unsafe { from_utf8_unchecked(bytes) };
                    match $f.write_str(s, s.len()) {
                        Ok(_) => {},
                        Err(e) => err = e,
                    }
                }
                Err(e) => err = e,
            }
        }
        if err == Unknown {
            Ok(())
        } else {
            Err(err)
        }
    }};
}

#[macro_export]
macro_rules! format {
        ($fmt:expr) => {{
                format!("{}", $fmt)
        }};
        ($fmt:expr, $($t:expr),*) => {{
                let mut formatter = Formatter::new();
                match writef!(&mut formatter, $fmt, $($t),*) {
                    Ok(_) => String::new(formatter.as_str()),
                    Err(e) => Err(e)
                }
        }};
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => {{
            use ffi::write;
            #[allow(unused_unsafe)]
            unsafe {
                    write(2, $fmt.as_ptr(), $fmt.len());
                    write(2, "\n".as_ptr(), 1);
            }
    }};
    ($fmt:expr, $($t:expr),*) => {{
        match format!($fmt, $($t),*) {
            Ok(line) => {
                use ffi::write;
                #[allow(unused_unsafe)]
                unsafe {
                        write(2, line.as_str().as_ptr(), line.len());
                        write(2, "\n".as_ptr(), 1);
                }
            },
            Err(_e) => {},
        }
    }};
}

#[macro_export]
macro_rules! print {
    ($fmt:expr) => {{
        use ffi::write;
        #[allow(unused_unsafe)]
        unsafe { write(2, $fmt.as_ptr(), $fmt.len()); }
    }};
    ($fmt:expr, $($t:expr),*) => {{
        use ffi::write;
        match format!($fmt, $($t),*) {
            Ok(line) => {
                #[allow(unused_unsafe)]
                unsafe { write(2, line.as_str().as_ptr(), line.len()); }
            },
            Err(_e) => {},
        }
    }};
}
