#[macro_export]
macro_rules! aadd {
	($a:expr, $v:expr) => {{
		use ffi::atomic_fetch_add_u64;
		unsafe { atomic_fetch_add_u64($a, $v) }
	}};
}

#[macro_export]
macro_rules! asub {
	($a:expr, $v:expr) => {{
		use ffi::atomic_fetch_sub_u64;
		unsafe { atomic_fetch_sub_u64($a, $v) }
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
		unsafe { atomic_store_u64($a, $v) }
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
macro_rules! box_dyn {
	($value:expr, $trait:path) => {{
		let mut boxed = Box::new($value).unwrap();
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
			unsafe {
				let rptr = alloc(total_size) as *mut u8;
				if rptr.is_null() {
					panic!("Allocation failed");
				}
				let mut write_ptr = rptr;
				for _ in 0..count {
					write(write_ptr as *mut _, $value);
					write_ptr = write_ptr.add(elem_size);
				}
				rptr as *mut _
			}
		};
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
			Err(Error::new(IllegalState))
		} else {
			unsafe {
				let rptr = alloc(total_size) as *mut u8;
				if rptr.is_null() {
					Err(Error::new(Alloc))
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
macro_rules! vec {
                ($($elem:expr),*) => {
                    {
                                let mut vec = Vec::new();
                                let mut err: Error = Error::new(Unknown);
                                $(
                                        if err.kind == ErrorKind::Unknown {
                                                match vec.push($elem) {
                                                        Ok(_) => {},
                                                        Err(e) => err = e,
                                                }
                                        }
                                )*
                                if err.kind != ErrorKind::Unknown {
                                        Err(err)
                                } else {
                                        Ok(vec)
                                }
                    }
                };
}
