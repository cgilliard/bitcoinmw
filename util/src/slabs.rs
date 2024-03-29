// Copyright (c) 2023-2024, The BitcoinMW Developers
// Some code and concepts from:
// * Grin: https://github.com/mimblewimble/grin
// * Arti: https://gitlab.torproject.org/tpo/core/arti
// * BitcoinMW: https://github.com/bitcoinmw/bitcoinmw
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::misc::{set_max, slice_to_usize, usize_to_slice};
use crate::types::SlabAllocatorImpl;
use crate::{Array, Slab, SlabAllocator, SlabAllocatorConfig, SlabMut, UtilBuilder};
use bmw_err::{err, Error};
use bmw_log::*;
use std::cell::UnsafeCell;

info!();

impl Default for SlabAllocatorConfig {
	fn default() -> Self {
		Self {
			slab_size: 256,
			slab_count: 40 * 1024,
		}
	}
}

thread_local! {
		#[doc(hidden)]
		pub static GLOBAL_SLAB_ALLOCATOR: UnsafeCell<Box<dyn SlabAllocator>> = UtilBuilder::build_slabs_unsafe();
}

impl<'a> SlabMut<'a> {
	/// get an immutable reference to the data held in this slab.
	pub fn get(&self) -> &[u8] {
		&self.data
	}

	/// get a mutable reference to the data held in this slab.
	pub fn get_mut(&mut self) -> &mut [u8] {
		&mut self.data
	}

	/// get the id of this slab. Each slab has in an instance of [`crate::SlabAllocator`]
	/// has a unique id.
	pub fn id(&self) -> usize {
		self.id
	}
}

impl<'a> Slab<'a> {
	/// get a mutable reference to the data held in this slab.
	pub fn get(&self) -> &[u8] {
		&self.data
	}

	/// get the id of this slab. Each slab in an instance of [`crate::SlabAllocator`]
	/// has a unique id.
	pub fn id(&self) -> usize {
		self.id
	}
}

impl SlabAllocator for SlabAllocatorImpl {
	fn is_init(&self) -> bool {
		self.config.is_some()
	}
	fn allocate<'a>(&'a mut self) -> Result<SlabMut<'a>, Error> {
		if self.config.is_none() {
			return Err(err!(ErrKind::IllegalState, "not initialized"));
		}
		let config = self.config.as_ref().unwrap();
		debug!("allocate:self.config={:?}", config)?;
		if self.first_free == self.max_value {
			return Err(err!(ErrKind::CapacityExceeded, "no more slabs available"));
		}

		let id = self.first_free;
		debug!("slab allocate id = {}", id)?;
		let offset = (self.ptr_size + config.slab_size) * id;
		self.first_free = slice_to_usize(&self.data.as_slice()[offset..offset + self.ptr_size])?;
		debug!("new firstfree={}", self.first_free)?;
		let offset = offset + self.ptr_size;
		// mark it as not free we use max_value - 1 because max_value is used to
		// terminate the free list
		let mut invalid_ptr = [0u8; 8];
		usize_to_slice(self.max_value - 1, &mut invalid_ptr[0..self.ptr_size])?;

		self.data.as_mut()[(self.ptr_size + config.slab_size) * id
			..(self.ptr_size + config.slab_size) * id + self.ptr_size]
			.clone_from_slice(&invalid_ptr[0..self.ptr_size]);
		let data = &mut self.data.as_mut()[offset..offset + config.slab_size as usize];
		self.free_count = self.free_count.saturating_sub(1);

		Ok(SlabMut { data, id })
	}
	fn free(&mut self, id: usize) -> Result<(), Error> {
		debug!("slabs free id ={}", id)?;
		match &self.config {
			Some(config) => {
				if id >= config.slab_count {
					let fmt = format!("slab.id = {}, total slabs = {}", id, config.slab_count);
					return Err(err!(ErrKind::ArrayIndexOutOfBounds, fmt));
				}
				let offset = (self.ptr_size + config.slab_size) * id;
				debug!("first_free={}", self.first_free)?;

				// check that it's currently allocated
				let slab_entry =
					slice_to_usize(&self.data.as_slice()[offset..offset + self.ptr_size])?;

				if slab_entry != self.max_value - 1 {
					debug!("double free")?;
					// double free error
					let fmt = format!("slab.id = {} has been freed when not allocated", id);
					return Err(err!(ErrKind::IllegalState, fmt));
				}

				// update free list
				let mut first_free_slice = [0u8; 8];
				usize_to_slice(self.first_free, &mut first_free_slice[0..self.ptr_size])?;
				debug!("free:self.config={:?},id={}", config, id)?;
				self.data.as_mut()[offset..offset + self.ptr_size]
					.clone_from_slice(&first_free_slice[0..self.ptr_size]);
				self.first_free = id;
				debug!("update firstfree to {}", self.first_free)?;
				self.free_count += 1;
				Ok(())
			}
			None => {
				let text = "slab allocator has not been initialized";
				let e = err!(ErrKind::IllegalState, text);
				Err(e)
			}
		}
	}
	fn get<'a>(&'a self, id: usize) -> Result<Slab<'a>, Error> {
		if self.config.is_none() {
			return Err(err!(ErrKind::IllegalState, "not initialized"));
		}
		let config = self.config.as_ref().unwrap();
		if id >= config.slab_count {
			let fmt = format!("slab.id = {}, total slabs = {}", id, config.slab_count);
			return Err(err!(ErrKind::ArrayIndexOutOfBounds, fmt));
		}
		debug!("get:self.config={:?},id={}", config, id)?;
		// calculate offset of this slab
		let offset = self.ptr_size + ((self.ptr_size + config.slab_size) * id);
		// get data
		let data = &self.data.as_slice()[offset..offset + config.slab_size];
		Ok(Slab { data, id })
	}
	fn get_mut<'a>(&'a mut self, id: usize) -> Result<SlabMut<'a>, Error> {
		if self.config.is_none() {
			return Err(err!(ErrKind::IllegalState, "not initialized"));
		}
		let config = self.config.as_ref().unwrap();
		if id >= config.slab_count {
			let fmt = format!("slab.id = {}, total slabs = {}", id, config.slab_count);
			return Err(err!(ErrKind::ArrayIndexOutOfBounds, fmt));
		}
		debug!("get_mut:self.config={:?},id={}", config, id)?;
		// calculate offset of this slab
		let offset = self.ptr_size + ((self.ptr_size + config.slab_size) * id);
		// get data
		let data = &mut self.data.as_mut()[offset..offset + config.slab_size as usize];
		Ok(SlabMut { data, id })
	}

	fn free_count(&self) -> Result<usize, Error> {
		if self.config.is_none() {
			return Err(err!(ErrKind::IllegalState, "not initialized"));
		}
		let config = self.config.as_ref().unwrap();
		debug!("free_count:self.config={:?}", config)?;
		Ok(self.free_count)
	}

	fn slab_size(&self) -> Result<usize, Error> {
		debug!("slab_size:self.config={:?}", self.config)?;
		match &self.config {
			Some(config) => Ok(config.slab_size),
			None => {
				let text = "slab allocator has not been initialized";
				let e = err!(ErrKind::IllegalState, text);
				Err(e)
			}
		}
	}

	fn slab_count(&self) -> Result<usize, Error> {
		debug!("slab_size:self.config={:?}", self.config)?;
		match &self.config {
			Some(config) => Ok(config.slab_count),
			None => {
				let text = "slab allocator has not been initialized";
				let e = err!(ErrKind::IllegalState, text);
				Err(e)
			}
		}
	}

	fn init(&mut self, config: SlabAllocatorConfig) -> Result<(), Error> {
		match &self.config {
			Some(_) => {
				let text = "slab allocator has already been initialized";
				let e = err!(ErrKind::IllegalState, text);
				Err(e)
			}
			None => {
				if config.slab_size < 8 {
					let text = "slab_size must be at least 8 bytes";
					let e = err!(ErrKind::IllegalArgument, text);
					return Err(e);
				}
				if config.slab_count == 0 {
					let text = "slab_count must be greater than 0";
					let e = err!(ErrKind::IllegalArgument, text);
					return Err(e);
				}

				// build data array
				let mut data = UtilBuilder::build_array(
					config.slab_count * (config.slab_size + self.ptr_size),
					&0u8,
				)?;

				// calculate the pointer size and max_value
				self.ptr_size = 0;
				let mut x = config.slab_count + 2; // two more,
								   // one for termination
								   // pointer and one for free status
				loop {
					if x == 0 {
						break;
					}
					x >>= 8;
					self.ptr_size += 1;
				}
				let mut ptr = [0u8; 8];
				set_max(&mut ptr[0..self.ptr_size]);
				self.max_value = slice_to_usize(&ptr[0..self.ptr_size])?;

				// build free list
				Self::build_free_list(
					&mut data,
					config.slab_count,
					config.slab_size,
					self.ptr_size,
					self.max_value,
				)?;
				self.data = data;
				self.free_count = config.slab_count;
				self.config = Some(config);
				self.first_free = 0;
				Ok(())
			}
		}
	}
}

impl SlabAllocatorImpl {
	pub(crate) fn new() -> Self {
		let data = UtilBuilder::build_array(1, &0u8).unwrap();
		Self {
			config: None,
			free_count: 0,
			data,
			first_free: 0,
			ptr_size: 8,
			max_value: 0,
		}
	}

	fn build_free_list(
		data: &mut Array<u8>,
		slab_count: usize,
		slab_size: usize,
		ptr_size: usize,
		max_value: usize,
	) -> Result<(), Error> {
		// Initially all slabs are in the free list. Create a linked list of slabs.
		for i in 0..slab_count {
			let mut next_bytes = [0u8; 8];
			if i < slab_count - 1 {
				usize_to_slice(i + 1, &mut next_bytes[0..ptr_size])?;
			} else {
				usize_to_slice(max_value, &mut next_bytes[0..ptr_size])?;
			}

			let offset_next = i * (ptr_size + slab_size);
			data.as_mut()[offset_next..offset_next + ptr_size]
				.clone_from_slice(&next_bytes[0..ptr_size]);
		}
		Ok(())
	}
}
