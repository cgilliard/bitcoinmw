use prelude::*;

mod sealed {
	use prelude::*;
	pub trait Sealed {}
	impl<T: Clone> Sealed for T {}
	impl Sealed for str {}
	impl<T: Clone> Sealed for [T] {}
	pub struct Private;
}

use std::clone::sealed::{Private, Sealed};

// This trait is implemented by any type that implements [`std::clone::Clone`].
pub trait DynClone: Sealed {
	// Not public API
	#[doc(hidden)]
	fn __clone_box(&self, _: Private) -> Result<*mut ()>;
}

pub fn clone_box<T>(t: &T) -> Result<Box<T>>
where
	T: ?Sized + DynClone,
{
	let mut fat_ptr = t as *const T;
	unsafe {
		let data_ptr = &mut fat_ptr as *mut *const T as *mut *mut ();
		assert_eq!(*data_ptr as *const (), t as *const T as *const ());
		*data_ptr = <T as DynClone>::__clone_box(t, Private)?;
	}
	Ok(Box::from_raw(Ptr::new(fat_ptr as *mut T)))
}

impl<T> DynClone for T
where
	T: Clone,
{
	fn __clone_box(&self, _: Private) -> Result<*mut ()> {
		Ok(Box::<T>::into_raw(&Box::new(self.clone())?) as *mut ())
	}
}

pub trait TryClone {
	fn try_clone(&self) -> Result<Self>
	where
		Self: Sized;
}

impl<T: Clone> TryClone for T {
	fn try_clone(&self) -> Result<Self> {
		Ok(self.clone())
	}
}
