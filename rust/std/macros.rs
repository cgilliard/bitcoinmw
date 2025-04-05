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
