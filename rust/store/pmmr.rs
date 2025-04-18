use crypto::Ctx;
use prelude::*;

pub struct Pmmr {}

impl Pmmr {
	// create a new instance of this pmmr with the specified lmdb instance and prefix
	pub fn new(db: Lmdb, prefix: &str) -> Result<Self, Error> {
		Ok(Self {})
	}

	// return a writable instance of the pmmr view (can be used to append, hard_rewind, and
	// prune.
	pub fn write(&mut self) -> Result<Self, Error> {
		Ok(Self {})
	}

	// return a read only instance of the pmmr view (can only be used to soft_rewind, get
	// sync peaks and sync_chunks.
	pub fn read(&self) -> Result<Self, Error> {
		Ok(Self {})
	}

	// hard rewind the pmmr - destructive rewind of the backend (reorgs)
	pub fn hard_rewind(&mut self) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	// soft rewind the pmmr - non-destructive rewind of the view only. The backend is not
	// modified and only the view's own branch nodes are modified (using COW).
	pub fn soft_rewind(&mut self) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	// append new outputs to the pmmr backend
	pub fn append(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	// prune stale spent outputs from the pmmr backend
	pub fn prune(&mut self, ctx: &mut Ctx, data: &[u8]) -> Result<(), Error> {
		Err(Error::new(Todo))
	}

	// get the sync peaks (top 4 submountains of the largest mountain, top 2 submountains of
	// the second and third mountains it they exist, and the peak of the other mountains if
	// they exist).
	pub fn sync_peaks(&self) -> Result<Vec<[u8; 32]>, Error> {
		Err(Error::new(Todo))
	}

	// get the sync chunk associated with the sync_peaks for this pmmr instance.
	pub fn sync_chunk(&self, index: usize) -> Result<Vec<u8>, Error> {
		Err(Error::new(Todo))
	}
}
