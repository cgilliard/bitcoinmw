pub trait AsRaw<T> {
	fn as_ptr(&self) -> *const T;
}

pub trait AsRawMut<T> {
	fn as_mut_ptr(&mut self) -> *mut T;
}
