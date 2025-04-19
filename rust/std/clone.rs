use prelude::*;

pub trait TryClone {
	fn try_clone(&self) -> Result<Self, Error>
	where
		Self: Sized;
}

impl<T: Clone> TryClone for T {
	fn try_clone(&self) -> Result<Self, Error> {
		Ok(self.clone())
	}
}
