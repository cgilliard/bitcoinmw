use crypto::Ctx;
use prelude::*;

pub struct PmmrBackend {}

impl PmmrBackend {
	pub fn new() -> Self {
		Self {}
	}

	pub fn append(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<u64, Error> {
		Err(Error::new(Todo))
	}

	pub fn prune(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	pub fn truncate(&mut self, pos: u64) -> Result<(), Error> {
		Err(Error::new(Todo))
	}
}
