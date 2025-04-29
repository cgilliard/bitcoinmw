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
        use std::misc::simple_hash;
        define_errors_inner!(@count 0, simple_hash(file!(), line!()), $($error),*);
        #[cfg(test)]
        mod unique_test_mod {
            #[test]
            fn print_hash_value() {
                #[cfg(printhashes)]
                {
                    use std::misc::simple_hash;
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
macro_rules! box_dyn {
	($value:expr, $trait:path) => {{
		let mut boxed = match Box::new($value) {
			Ok(b) => b,
			Err(e) => exit!("box_dyn failed due to error: {}", e),
		};
		let ptr = boxed.ptr.raw();
		boxed.leak();
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
				boxed.leak();
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
		use std::ffi::alloc;
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
		//print!("Panic[@{}:{}]: ", file!(), line!());
		//println!($fmt, $($t),*);

		#[allow(unused_unsafe)]
		unsafe {
			use std::ffi::exit;
			exit(-1);
		}
		loop {}
	}};
}
