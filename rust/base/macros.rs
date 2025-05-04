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
