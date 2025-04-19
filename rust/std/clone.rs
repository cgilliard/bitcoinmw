use prelude::*;

pub trait TryClone {
	fn try_clone(&self) -> Result<Self, Error>
	where
		Self: Sized;
}
