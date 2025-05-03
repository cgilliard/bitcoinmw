#[macro_export]
macro_rules! lock {
	() => {{
		use core::cell::UnsafeCell;
		Lock {
			state: UnsafeCell::new(0),
		}
	}};
}

#[macro_export]
macro_rules! lock_box {
	() => {{
		use util::lock::LockBox;
		LockBox::new()
	}};
}

#[macro_export]
macro_rules! sched_yield {
	() => {{
		use std::ffi::sched_yield;
		#[allow(unused_unsafe)]
		unsafe {
			sched_yield();
		}
	}};
}
