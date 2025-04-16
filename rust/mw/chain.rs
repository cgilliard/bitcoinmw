#![allow(dead_code)]

use prelude::*;

pub struct Chain {}

impl Chain {
	pub fn new() -> Result<Self, Error> {
		Err(Error::new(Todo))
	}
}
