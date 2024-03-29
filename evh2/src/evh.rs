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

#[cfg(target_os = "linux")]
use crate::linux::*;
#[cfg(target_os = "macos")]
use crate::mac::*;
#[cfg(target_os = "windows")]
use crate::win::*;

use crate::constants::*;
use crate::types::{
	ConnectionType, ConnectionVariant, Event, EventHandlerCallbacks, EventHandlerConfig,
	EventHandlerContext, EventHandlerImpl, EventHandlerState, EventIn, EventType, EventTypeIn,
	GlobalStats, UserContextImpl, Wakeup, WriteHandle, WriteState,
};
use crate::{Connection, EventHandler, EvhStats, UserContext};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{ConfigBuilder, ConfigOption};
use bmw_deps::errno::{errno, set_errno, Errno};
use bmw_deps::rand::random;
use bmw_err::*;
use bmw_log::*;
use bmw_util::*;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::time::{SystemTime, UNIX_EPOCH};

info!();

impl Wakeup {
	pub(crate) fn new() -> Result<Self, Error> {
		set_errno(Errno(0));
		let (reader, writer) = wakeup_impl()?;
		let requested = lock_box!(false)?;
		let needed = lock_box!(false)?;
		let id = random();
		Ok(Self {
			id,
			reader,
			writer,
			requested,
			needed,
		})
	}

	pub(crate) fn wakeup(&mut self) -> Result<(), Error> {
		let mut requested = self.requested.wlock()?;
		let needed = self.needed.rlock()?;
		let need_wakeup = **needed.guard()? && !(**requested.guard()?);
		**requested.guard()? = true;
		if need_wakeup {
			debug!("wakeup writing to {}", self.writer)?;
			let len = write_impl(self.writer, &[0u8; 1])?;
			debug!("len={},errno={}", len, errno())?;
		}
		Ok(())
	}

	pub(crate) fn pre_block(&mut self) -> Result<(bool, RwLockReadGuardWrapper<bool>), Error> {
		let requested = self.requested.rlock()?;
		{
			let mut needed = self.needed.wlock()?;
			**needed.guard()? = true;
		}
		let lock_guard = self.needed.rlock()?;
		let is_requested = **requested.guard()?;
		Ok((is_requested, lock_guard))
	}

	pub(crate) fn post_block(&mut self) -> Result<(), Error> {
		let mut requested = self.requested.wlock()?;
		let mut needed = self.needed.wlock()?;

		**requested.guard()? = false;
		**needed.guard()? = false;
		Ok(())
	}
}

impl UserContext for &mut UserContextImpl {
	fn clone_next_chunk(
		&mut self,
		connection: &mut Connection,
		buf: &mut [u8],
	) -> Result<usize, Error> {
		let last_slab = connection.get_last_slab();
		let slab_offset = connection.get_slab_offset();

		if self.slab_cur >= u32::MAX as usize {
			Ok(0)
		} else {
			let slab = self.read_slabs.get(self.slab_cur)?;
			let slab = slab.get();
			let start_ptr = slab.len().saturating_sub(4);

			let mut offset = if self.slab_cur == last_slab {
				slab_offset
			} else {
				start_ptr
			};

			let buf_len = buf.len();
			if buf_len < offset {
				offset = buf_len;
			}
			buf[0..offset].clone_from_slice(&slab[0..offset]);
			self.slab_cur =
				u32::from_be_bytes(try_into!(&slab[start_ptr..start_ptr + 4])?) as usize;
			Ok(offset)
		}
	}
	fn cur_slab_id(&self) -> usize {
		self.slab_cur
	}
	fn clear_all(&mut self, connection: &mut Connection) -> Result<(), Error> {
		self.clear_through(connection.get_last_slab(), connection)
	}
	fn clear_through(&mut self, slab_id: usize, connection: &mut Connection) -> Result<(), Error> {
		debug!("clear_through for {}", connection.handle())?;
		let mut cur = connection.get_first_slab();
		loop {
			if cur >= u32::MAX as usize {
				connection.set_first_slab(u32::MAX as usize);
				connection.set_last_slab(u32::MAX as usize);
				break;
			}
			let slab = self.read_slabs.get(cur)?;
			let slab = slab.get();
			let len = slab.len();
			let start = len.saturating_sub(4);
			let next = u32::from_be_bytes(try_into!(&slab[start..start + 4])?) as usize;
			debug!("free slab {}", cur)?;
			self.read_slabs.free(cur)?;

			connection.set_first_slab(next);
			if connection.get_first_slab() >= u32::MAX as usize {
				connection.set_last_slab(connection.get_first_slab());
			}

			if cur == slab_id {
				debug!("breaking because cur = {}, slab_id = {}", cur, slab_id)?;
				break;
			}

			cur = next;
		}

		debug!(
			"clear through complete first_slab={},last_slab={}",
			connection.get_first_slab(),
			connection.get_last_slab()
		)?;
		Ok(())
	}

	fn get_user_data(&mut self) -> &mut Option<Box<dyn Any + Send + Sync>> {
		&mut self.user_data
	}

	fn set_user_data(&mut self, user_data: Box<dyn Any + Send + Sync>) {
		self.user_data = Some(user_data);
	}
}

impl WriteState {
	fn new() -> Self {
		Self {
			flags: 0,
			write_buffer: vec![],
		}
	}

	pub(crate) fn set_flag(&mut self, flag: u8) {
		self.flags |= flag;
	}

	pub(crate) fn unset_flag(&mut self, flag: u8) {
		self.flags &= !flag;
	}

	pub(crate) fn is_set(&self, flag: u8) -> bool {
		self.flags & flag != 0
	}
}

impl WriteHandle {
	pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
		let data_len = data.len();
		let wlen = {
			let write_state = self.write_state.rlock()?;
			let guard = write_state.guard()?;

			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				let text = format!("write on a closed handle: {}", self.handle);
				return Err(err!(ErrKind::IO, text));
			} else if (**guard).is_set(WRITE_STATE_FLAG_PENDING) {
				0
			} else {
				write_impl(self.handle, data)?
			}
		};

		if wlen < 0 {
			let text = format!(
				"An i/o error occurred while trying to write to handle {}: {}",
				self.handle,
				errno()
			);
			return Err(err!(ErrKind::IO, text));
		}

		let wlen: usize = try_into!(wlen)?;

		if wlen < data_len {
			self.queue_data(&data[wlen..])?;
		}
		Ok(())
	}
	pub fn close(&mut self) -> Result<(), Error> {
		{
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard()?;
			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				let text = format!(
					"try to close a handle that is already closed: {}",
					self.handle
				);
				return Err(err!(ErrKind::IO, text));
			}

			(**guard).set_flag(WRITE_STATE_FLAG_CLOSE);
		}

		{
			wlock!(self.state).write_queue.push_back(self.id);
		}

		self.wakeup.wakeup()?;
		Ok(())
	}
	pub fn trigger_on_read(&mut self) -> Result<(), Error> {
		debug!("trigger on read {} ", self.handle)?;
		{
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard()?;
			if (**guard).is_set(WRITE_STATE_FLAG_CLOSE) {
				let text = format!("trigger_on_read on a closed handle: {}", self.handle);
				return Err(err!(ErrKind::IO, text));
			}

			(**guard).set_flag(WRITE_STATE_FLAG_TRIGGER_ON_READ);
		}
		{
			wlock!(self.state).write_queue.push_back(self.id);
		}
		self.wakeup.wakeup()?;

		Ok(())
	}

	fn is_set(&self, flag: u8) -> Result<bool, Error> {
		let write_state = self.write_state.rlock()?;
		let guard = write_state.guard()?;
		Ok((**guard).is_set(flag))
	}

	fn unset_flag(&mut self, flag: u8) -> Result<(), Error> {
		let mut write_state = self.write_state.wlock()?;
		let guard = write_state.guard()?;
		(**guard).unset_flag(flag);
		Ok(())
	}

	fn write_state(&mut self) -> Result<&mut Box<dyn LockBox<WriteState>>, Error> {
		Ok(&mut self.write_state)
	}
	fn new(connection_impl: &Connection) -> Result<Self, Error> {
		let wakeup = match &connection_impl.wakeup {
			Some(wakeup) => wakeup.clone(),
			None => {
				return Err(err!(
					ErrKind::IllegalState,
					"cannot create a write handle on a connection that has no wakeup set"
				));
			}
		};
		let state = match &connection_impl.state {
			Some(state) => state.clone(),
			None => {
				return Err(err!(
					ErrKind::IllegalState,
					"cannot create a write handle on a connection that has no state set"
				));
			}
		};
		Ok(Self {
			handle: connection_impl.handle,
			id: connection_impl.id,
			write_state: connection_impl.write_state.clone(),
			wakeup,
			state,
		})
	}
	fn queue_data(&mut self, data: &[u8]) -> Result<(), Error> {
		{
			let mut write_state = self.write_state.wlock()?;
			let guard = write_state.guard()?;
			(**guard).set_flag(WRITE_STATE_FLAG_PENDING);
			(**guard).write_buffer.extend(data);
		}

		{
			wlock!(self.state).write_queue.push_back(self.id);
		}

		self.wakeup.wakeup()?;
		Ok(())
	}
}

impl Connection {
	pub fn id(&self) -> u128 {
		self.id
	}
	pub fn write_handle(&self) -> Result<WriteHandle, Error> {
		let wh = WriteHandle::new(self)?;
		Ok(wh)
	}
	pub(crate) fn new(
		handle: Handle,
		wakeup: Option<Wakeup>,
		state: Option<Box<dyn LockBox<EventHandlerState>>>,
		ctype: ConnectionType,
	) -> Result<Self, Error> {
		Ok(Self {
			handle,
			id: random(),
			first_slab: usize::MAX,
			last_slab: usize::MAX,
			slab_offset: 0,
			write_state: lock_box!(WriteState::new())?,
			wakeup,
			state,
			tx: None,
			ctype,
		})
	}
	pub(crate) fn handle(&self) -> Handle {
		self.handle
	}
	fn set_state(&mut self, state: Box<dyn LockBox<EventHandlerState>>) -> Result<(), Error> {
		self.state = Some(state);
		Ok(())
	}

	fn set_wakeup(&mut self, wakeup: Wakeup) -> Result<(), Error> {
		self.wakeup = Some(wakeup);
		Ok(())
	}

	fn set_tx(&mut self, tx: SyncSender<()>) {
		self.tx = Some(tx);
	}
	fn get_tx(&mut self) -> Option<&mut SyncSender<()>> {
		self.tx.as_mut()
	}
	fn get_slab_offset(&self) -> usize {
		self.slab_offset
	}
	fn get_first_slab(&self) -> usize {
		self.first_slab
	}
	fn get_last_slab(&self) -> usize {
		self.last_slab
	}
	fn set_slab_offset(&mut self, slab_offset: usize) {
		self.slab_offset = slab_offset;
	}
	fn set_first_slab(&mut self, first_slab: usize) {
		self.first_slab = first_slab;
	}
	fn set_last_slab(&mut self, last_slab: usize) {
		self.last_slab = last_slab;
	}
}

impl EventHandlerState {
	pub(crate) fn new() -> Result<Self, Error> {
		Ok(Self {
			nconnections: VecDeque::new(),
			write_queue: VecDeque::new(),
			stop: false,
		})
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic> Drop
	for EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext + '_>, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	fn drop(&mut self) {
		let _ = debug!("drop evh");
		match self.stop() {
			Ok(_) => {}
			Err(e) => {
				let _ = error!("Error occurred while dropping: {}", e);
			}
		}
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	for EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext + '_>, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	fn start(&mut self) -> Result<(), Error> {
		self.start_impl()
	}
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error> {
		self.callbacks.on_read = Some(Box::pin(on_read));
		Ok(())
	}
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error> {
		self.callbacks.on_accept = Some(Box::pin(on_accept));
		Ok(())
	}
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error> {
		self.callbacks.on_close = Some(Box::pin(on_close));
		Ok(())
	}
	fn set_on_housekeeper(&mut self, on_housekeeper: OnHousekeeper) -> Result<(), Error> {
		self.callbacks.on_housekeeper = Some(Box::pin(on_housekeeper));
		Ok(())
	}
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error> {
		self.callbacks.on_panic = Some(Box::pin(on_panic));
		Ok(())
	}
	fn add_server_connection(&mut self, mut connection: Connection) -> Result<(), Error> {
		match connection.ctype {
			ConnectionType::Server => {}
			_ => {
				return Err(err!(
					ErrKind::IllegalArgument,
					"trying to add a non-server connection as a server!"
				))
			}
		}
		let (tx, rx) = sync_channel(1);
		connection.set_tx(tx);

		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;
		debug!(
			"adding server handle = {}, tid = {}",
			connection.handle(),
			tid
		)?;

		{
			let mut state = self.state[tid].wlock()?;
			let guard = state.guard()?;
			(**guard)
				.nconnections
				.push_back(ConnectionVariant::ServerConnection(connection));
		}

		debug!("about to wakeup")?;

		self.wakeups[tid].wakeup()?;

		debug!("add complete")?;

		rx.recv()?;

		Ok(())
	}
	fn add_client_connection(&mut self, mut connection: Connection) -> Result<WriteHandle, Error> {
		match connection.ctype {
			ConnectionType::Client => {}
			_ => {
				return Err(err!(
					ErrKind::IllegalArgument,
					"trying to add a non-server connection as a server!"
				))
			}
		}
		let (tx, rx) = sync_channel(1);
		connection.set_tx(tx);

		let handle = connection.handle();
		let tid: usize = try_into!(handle % self.config.threads as Handle)?;

		connection.set_state(self.state[tid].clone())?;
		connection.set_wakeup(self.wakeups[tid].clone())?;
		let ret = connection.write_handle()?;

		debug!(
			"adding client handle = {}, tid = {}, id = {}",
			connection.handle(),
			tid,
			connection.id(),
		)?;

		{
			let mut state = self.state[tid].wlock()?;
			let guard = state.guard()?;
			(**guard)
				.nconnections
				.push_back(ConnectionVariant::ClientConnection(connection));
		}

		debug!("push client connection to tid = {}", tid)?;

		self.wakeups[tid].wakeup()?;

		rx.recv()?;
		Ok(ret)
	}

	fn wait_for_stats(&mut self) -> Result<EvhStats, Error> {
		self.wait_for_stats()
	}
}

impl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
	EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(&mut Connection, &mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnHousekeeper: FnMut(&mut Box<dyn UserContext + '_>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnPanic: FnMut(&mut Box<dyn UserContext + '_>, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = Self::build_config(configs)?;
		let mut state = array!(config.threads, &lock_box!(EventHandlerState::new()?)?)?;

		let w = Wakeup::new()?;
		let mut wakeups = array!(config.threads, &w)?;

		for i in 0..config.threads {
			state[i] = lock_box!(EventHandlerState::new()?)?;
			wakeups[i] = Wakeup::new()?;
		}

		let global_stats = GlobalStats {
			stats: EvhStats::new(),
			update_counter: 0,
			tx: None,
		};
		let stats = lock_box!(global_stats)?;

		Ok(Self {
			callbacks: EventHandlerCallbacks {
				on_read: None,
				on_accept: None,
				on_close: None,
				on_panic: None,
				on_housekeeper: None,
			},
			config,
			state,
			wakeups,
			stats,
			stopper: None,
		})
	}

	fn wait_for_stats(&mut self) -> Result<EvhStats, Error> {
		let mut ret = EvhStats::new();
		let (tx, rx) = sync_channel(1);
		{
			let mut stats = self.stats.wlock()?;
			let guard = stats.guard()?;
			(**guard).tx = Some(tx);
		}

		rx.recv()?;

		{
			let mut stats = self.stats.wlock()?;
			let guard = stats.guard()?;

			let _ = std::mem::replace(&mut ret, (**guard).stats.clone());
			(**guard).stats.reset();
		}

		Ok(ret)
	}

	fn start_impl(&mut self) -> Result<(), Error> {
		let mut tp = thread_pool!(MinSize(self.config.threads))?;
		let mut executor = lock_box!(tp.executor()?)?;
		let mut executor_clone = executor.clone();
		self.stopper = Some(tp.stopper()?);

		let config = self.config.clone();
		let callbacks = self.callbacks.clone();
		let state = self.state.clone();
		let wakeups = self.wakeups.clone();

		let read_slabs = slab_allocator!(
			SlabSize(config.read_slab_size),
			SlabCount(config.read_slab_count)
		)?;
		let user_context = UserContextImpl {
			read_slabs,
			user_data: None,
			slab_cur: usize::MAX,
		};
		let mut ctx_arr = array!(
			config.threads,
			&lock_box!(EventHandlerContext::new(
				wakeups.clone(),
				0,
				self.stats.clone()
			)?)?
		)?;
		let mut user_context_arr = array!(config.threads, &lock_box!(user_context)?)?;

		for i in 0..config.threads {
			let mut evhc = EventHandlerContext::new(wakeups.clone(), i, self.stats.clone())?;
			let wakeup_reader = wakeups[i].reader;
			let evt = EventIn::new(wakeup_reader, EventTypeIn::Read);
			evhc.in_events.push(evt);

			ctx_arr[i] = lock_box!(evhc)?;
			let read_slabs = slab_allocator!(
				SlabSize(config.read_slab_size),
				SlabCount(config.read_slab_count)
			)?;
			let user_context = UserContextImpl {
				read_slabs,
				user_data: None,
				slab_cur: usize::MAX,
			};
			user_context_arr[i] = lock_box!(user_context)?;

			wlock!(self.state[i])
				.nconnections
				.push_back(ConnectionVariant::Wakeup(wakeups[i].clone()));
		}

		let user_context_arr_clone = user_context_arr.clone();
		let ctx_arr_clone = ctx_arr.clone();

		tp.set_on_panic(move |id, e| -> Result<(), Error> {
			let e = e.downcast_ref::<&str>().unwrap_or(&"unknown error");
			error!("on panic: [thread_id={}]: {}", id, e)?;
			let config = config.clone();
			let callbacks = callbacks.clone();
			let state = state.clone();
			let user_ctx_arr_clone = user_context_arr_clone.clone();
			let ctx_arr = ctx_arr_clone.clone();

			wlock!(executor).execute(
				async move {
					match EventHandlerImpl::execute_thread(
						config,
						callbacks,
						state,
						ctx_arr,
						user_ctx_arr_clone,
						try_into!(id)?,
						true,
					) {
						Ok(_) => {}
						Err(e) => {
							fatal!("Execute thread had an unexpected error: {}", e)?;
						}
					}
					Ok(())
				},
				try_into!(id)?,
			)?;
			Ok(())
		})?;

		tp.start()?;

		{
			let mut executor = executor_clone.wlock()?;
			let guard = executor.guard()?;
			(**guard) = tp.executor()?;
		}

		for i in 0..self.config.threads {
			let config = self.config.clone();
			let callbacks = self.callbacks.clone();
			let state = self.state.clone();
			let ctx_arr = ctx_arr.clone();
			let user_context_arr = user_context_arr.clone();
			execute!(tp, try_into!(i)?, {
				match Self::execute_thread(
					config,
					callbacks,
					state,
					ctx_arr.clone(),
					user_context_arr.clone(),
					i,
					false,
				) {
					Ok(_) => {}
					Err(e) => {
						fatal!("Execute thread had an unexpected error: {}", e)?;
					}
				}
				Ok(())
			})?;
		}
		Ok(())
	}

	fn build_config(configs: Vec<ConfigOption>) -> Result<EventHandlerConfig, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(
			vec![
				CN::EvhReadSlabSize,
				CN::EvhReadSlabCount,
				CN::EvhTimeout,
				CN::EvhThreads,
				CN::EvhHouseKeeperFrequencyMillis,
				CN::EvhStatsUpdateMillis,
				CN::Debug,
			],
			vec![],
		)?;

		let threads = config.get_or_usize(&CN::EvhThreads, EVH_DEFAULT_THREADS);
		let read_slab_count =
			config.get_or_usize(&CN::EvhReadSlabCount, EVH_DEFAULT_READ_SLAB_COUNT);
		let read_slab_size = config.get_or_usize(&CN::EvhReadSlabSize, EVH_DEFAULT_READ_SLAB_SIZE);
		let debug = config.get_or_bool(&CN::Debug, false);
		let timeout = config.get_or_u16(&CN::EvhTimeout, EVH_DEFAULT_TIMEOUT);
		let housekeeping_frequency_millis = config.get_or_usize(
			&CN::EvhHouseKeeperFrequencyMillis,
			EVH_DEFAULT_HOUSEKEEPING_FREQUENCY_MILLIS,
		);
		let stats_update_frequency_millis =
			config.get_or_usize(&CN::EvhStatsUpdateMillis, EVH_DEFAULT_STATS_UPDATE_MILLIS);

		if read_slab_count == 0 {
			let text = "EvhReadSlabCount count must not be 0";
			return Err(err!(ErrKind::Configuration, text));
		}

		if read_slab_size < 25 {
			let text = "EvhReadSlabSize must be at least 25";
			return Err(err!(ErrKind::Configuration, text));
		}

		if timeout == 0 {
			let text = "EvhTimeout must not be 0";
			return Err(err!(ErrKind::Configuration, text));
		}

		if housekeeping_frequency_millis == 0 {
			let text = "EvhHouseKeeperFrequencyMillis must not be 0";
			return Err(err!(ErrKind::Configuration, text));
		}

		Ok(EventHandlerConfig {
			threads,
			debug,
			timeout,
			read_slab_size,
			read_slab_count,
			housekeeping_frequency_millis,
			stats_update_frequency_millis,
		})
	}

	fn execute_thread(
		config: EventHandlerConfig,
		mut callbacks: EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		mut state: Array<Box<dyn LockBox<EventHandlerState>>>,
		mut ctx_arr: Array<Box<dyn LockBox<EventHandlerContext>>>,
		mut user_context_arr: Array<Box<dyn LockBox<UserContextImpl>>>,
		tid: usize,
		panic_recovery: bool,
	) -> Result<(), Error> {
		debug!("execute thread {}", tid)?;

		let mut ctx = ctx_arr[tid].wlock_ignore_poison()?;
		let mut user_context = user_context_arr[tid].wlock_ignore_poison()?;
		let ctx_guard = ctx.guard()?;
		let user_context_guard = user_context.guard()?;

		let mut count = 0u128;

		if panic_recovery {
			let ret_event_itt = (**ctx_guard).ret_event_itt;
			let ret_event_count = (**ctx_guard).ret_event_count;

			let trigger_itt = (**ctx_guard).trigger_itt;
			let trigger_count = (**ctx_guard).trigger_on_read_list.len();
			warn!(
				"panic recovery with ret_event_count = {}, ret_event_itt = {}, trigger_count = {}, trigger_itt = {}",
				ret_event_count, ret_event_itt, trigger_count, trigger_itt
			)?;

			if trigger_itt < trigger_count {
				let handle = (**ctx_guard).trigger_on_read_list[trigger_itt];
				debug!("handle to close (trigger_on_read) = {}", handle)?;

				Self::process_close(
					handle,
					&mut (**ctx_guard),
					&mut callbacks,
					&mut (**user_context_guard),
				)?;

				// skip over errant event
				(**ctx_guard).trigger_itt += 1;
			} else if ret_event_itt < ret_event_count {
				// the error must have been in the on_read regular read events
				let handle = (**ctx_guard).ret_events[(**ctx_guard).ret_event_itt].handle;
				debug!("handle to close (regular) = {}", handle)?;

				Self::process_close(
					handle,
					&mut (**ctx_guard),
					&mut callbacks,
					&mut (**user_context_guard),
				)?;

				// skip over errant event
				(**ctx_guard).ret_event_itt += 1;
			} else {
				// something's wrong but continue
				warn!("panic, but no pending events. Internal panic?")?;
			}

			match Self::process_events(
				&config,
				&mut (**ctx_guard),
				&mut callbacks,
				&mut state,
				&mut (**user_context_guard),
			) {
				Ok(_) => {}
				Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
			}
		}

		match Self::process_state(
			&mut state[tid],
			&mut (**ctx_guard),
			&mut callbacks,
			&mut (**user_context_guard),
			&config,
		) {
			Ok(_) => {}
			Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
		}

		loop {
			match get_events(&config, &mut (**ctx_guard)) {
				Ok(_) => {}
				Err(e) => fatal!("get_events generated an unexpected error: {}", e)?,
			}

			(**ctx_guard).thread_stats.event_loops += 1;
			(**ctx_guard).in_events.clear();
			(**ctx_guard)
				.in_events
				.shrink_to(EVH_DEFAULT_IN_EVENTS_SIZE);

			if config.debug {
				info!("Thread loop {}", count)?;
			}

			match Self::process_state(
				&mut state[tid],
				&mut (**ctx_guard),
				&mut callbacks,
				&mut (**user_context_guard),
				&config,
			) {
				Ok(stop) => {
					if stop {
						break Ok(());
					}
				}
				Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
			}

			debug!("calling proc events")?;
			// set iterator to 0 outside function in case of thread panic
			(**ctx_guard).ret_event_itt = 0;
			(**ctx_guard).trigger_itt = 0;
			match Self::process_events(
				&config,
				&mut (**ctx_guard),
				&mut callbacks,
				&mut state,
				&mut (**user_context_guard),
			) {
				Ok(_) => {}
				Err(e) => fatal!("Process events generated an unexpected error: {}", e)?,
			}
			count += 1;
		}
	}

	fn close_handles(ctx: &mut EventHandlerContext) -> Result<(), Error> {
		for (handle, id) in &ctx.handle_hash {
			debug!("close handle = {}, id = {}", handle, id)?;
			close_impl(*handle)?;
		}
		Ok(())
	}

	fn process_housekeeper(
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		config: &EventHandlerConfig,
	) -> Result<(), Error> {
		let now: usize = try_into!(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
		if now.saturating_sub(ctx.last_housekeeping) > config.housekeeping_frequency_millis {
			Self::call_on_housekeeper(user_context, &mut callbacks.on_housekeeper)?;

			ctx.last_housekeeping = now;
		}

		if now.saturating_sub(ctx.last_stats_update) > config.stats_update_frequency_millis {
			Self::update_stats(ctx, config)?;
			ctx.last_stats_update = now;
		}
		Ok(())
	}

	fn update_stats(
		ctx: &mut EventHandlerContext,
		config: &EventHandlerConfig,
	) -> Result<(), Error> {
		{
			let mut global_stats = ctx.global_stats.wlock()?;
			let guard = global_stats.guard()?;
			(**guard).stats.incr_stats(&ctx.thread_stats);
			(**guard).update_counter += 1;
			if (**guard).update_counter >= config.threads {
				match &(**guard).tx {
					Some(tx) => {
						tx.send(())?;
					}
					None => {}
				}

				(**guard).update_counter = 0;
			}
		}

		ctx.thread_stats.reset();
		Ok(())
	}

	fn process_state(
		state: &mut Box<dyn LockBox<EventHandlerState>>,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		config: &EventHandlerConfig,
	) -> Result<bool, Error> {
		debug!("in process state tid={}", ctx.tid)?;

		Self::process_write_pending(ctx, callbacks, user_context, state)?;

		let mut state = state.wlock()?;
		let guard = state.guard()?;

		if (**guard).stop {
			debug!("stopping thread")?;
			Self::close_handles(ctx)?;
			Ok(true)
		} else {
			Self::process_housekeeper(ctx, callbacks, user_context, config)?;
			debug!("nconnections.size={}", (**guard).nconnections.len())?;
			loop {
				let next = (**guard).nconnections.pop_front();
				if next.is_none() {
					break;
				}
				let mut next = next.unwrap();
				let (handle, id) = match &mut next {
					ConnectionVariant::ServerConnection(conn) => {
						debug!("server in process state")?;
						match conn.get_tx() {
							Some(tx) => {
								// attempt to send notification
								let _ = tx.send(());
							}
							None => {}
						}
						(conn.handle(), conn.id())
					}
					ConnectionVariant::ClientConnection(conn) => {
						debug!("client in process state")?;
						match conn.get_tx() {
							Some(tx) => {
								// attempt to send notification
								let _ = tx.send(());
							}
							None => {}
						}
						(conn.handle(), conn.id())
					}
					ConnectionVariant::Connection(conn) => {
						ctx.thread_stats.accepts += 1;
						Self::call_on_accept(user_context, conn, &mut callbacks.on_accept)?;
						(conn.handle(), conn.id())
					}
					ConnectionVariant::Wakeup(wakeup) => (wakeup.reader, wakeup.id),
				};

				debug!("found handle = {}, id = {}", handle, id)?;
				ctx.id_hash.insert(id, next);
				ctx.handle_hash.insert(handle, id);
				let event_in = EventIn::new(handle, EventTypeIn::Read);
				ctx.in_events.push(event_in);
			}

			Ok(false)
		}
	}

	fn process_write_pending(
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
		state: &mut Box<dyn LockBox<EventHandlerState>>,
	) -> Result<(), Error> {
		debug!("in process write pending")?;
		let mut ids = vec![];
		{
			let mut state = state.wlock()?;
			let guard = state.guard()?;
			debug!("write_queue.len={}", (**guard).write_queue.len())?;
			loop {
				match (**guard).write_queue.pop_front() {
					Some(id) => {
						debug!("popped id = {}", id)?;
						ids.push(id);
					}
					None => break,
				}
			}
		}

		ctx.trigger_on_read_list.clear();
		for id in ids {
			Self::process_write_id(ctx, id, callbacks, user_context)?;
		}

		Ok(())
	}

	fn process_write_id(
		ctx: &mut EventHandlerContext,
		id: u128,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		let mut close_list = vec![];
		match ctx.id_hash.get_mut(&id) {
			Some(conn) => match conn {
				ConnectionVariant::ServerConnection(_conn) => {}
				ConnectionVariant::ClientConnection(conn) => {
					let handle = conn.handle();
					let (close, trigger_on_read, pending) = Self::write_conn(conn)?;
					if close {
						close_list.push(conn.handle());
					}
					if trigger_on_read {
						ctx.trigger_on_read_list.push(conn.handle());
					}

					if pending {
						let evt = EventIn::new(handle, EventTypeIn::Write);
						ctx.in_events.push(evt);
					}
				}
				ConnectionVariant::Connection(conn) => {
					let handle = conn.handle();
					let (close, trigger_on_read, pending) = Self::write_conn(conn)?;
					if close {
						close_list.push(conn.handle());
					}
					if trigger_on_read {
						ctx.trigger_on_read_list.push(conn.handle());
					}
					if pending {
						let evt = EventIn::new(handle, EventTypeIn::Write);
						ctx.in_events.push(evt);
					}
				}
				ConnectionVariant::Wakeup(_wakeup) => {}
			},
			None => {
				warn!("none1 in process_write_id")?;
			}
		}

		for handle in close_list {
			Self::process_close(handle, ctx, callbacks, user_context)?;
		}
		Ok(())
	}

	fn write_conn(conn: &mut Connection) -> Result<(bool, bool, bool), Error> {
		let mut write_handle = conn.write_handle()?;
		let ret1 = write_handle.is_set(WRITE_STATE_FLAG_CLOSE)?;
		let ret2 = write_handle.is_set(WRITE_STATE_FLAG_TRIGGER_ON_READ)?;
		let ret3 = write_handle.is_set(WRITE_STATE_FLAG_PENDING)?;
		if ret2 {
			write_handle.unset_flag(WRITE_STATE_FLAG_TRIGGER_ON_READ)?;
		}
		Ok((ret1, ret2, ret3))
	}

	fn process_events(
		config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		// first call the trigger on reads
		debug!("trig list = {:?}", ctx.trigger_on_read_list)?;
		//for handle in &ctx.trigger_on_read_list {
		let list_len = ctx.trigger_on_read_list.len();
		loop {
			if ctx.trigger_itt == list_len {
				break;
			}

			let handle = ctx.trigger_on_read_list[ctx.trigger_itt];
			match ctx.handle_hash.get(&handle) {
				Some(id) => match ctx.id_hash.get_mut(&id) {
					Some(conn) => match conn {
						ConnectionVariant::Connection(conn) => {
							Self::call_on_read(user_context, conn, &mut callbacks.on_read)?
						}
						ConnectionVariant::ClientConnection(conn) => {
							Self::call_on_read(user_context, conn, &mut callbacks.on_read)?
						}
						_ => warn!("unexpected Conection variant for trigger_on_read")?,
					},
					None => {
						warn!("none in trigger_on_read1")?;
					}
				},
				None => {
					warn!("none in trigger_on_read2")?;
				}
			}
			ctx.trigger_itt += 1;
		}

		// next process events
		debug!("events to process = {}", ctx.ret_event_count)?;
		loop {
			if ctx.ret_event_itt == ctx.ret_event_count {
				break;
			}
			debug!("proc event = {:?}", ctx.ret_events[ctx.ret_event_itt])?;
			if ctx.ret_events[ctx.ret_event_itt].etype == EventType::Read
				|| ctx.ret_events[ctx.ret_event_itt].etype == EventType::ReadWrite
			{
				Self::process_read_event(
					config,
					ctx,
					callbacks,
					ctx.ret_events[ctx.ret_event_itt].handle,
					state,
					user_context,
				)?;
			}
			if ctx.ret_events[ctx.ret_event_itt].etype == EventType::Write
				|| ctx.ret_events[ctx.ret_event_itt].etype == EventType::ReadWrite
			{
				Self::process_write_event(
					config,
					ctx,
					callbacks,
					ctx.ret_events[ctx.ret_event_itt].handle,
				)?;
			}
			ctx.ret_event_itt += 1;
		}

		Ok(())
	}

	fn process_read_event(
		config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		handle: Handle,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		let mut accepted = vec![];
		let mut close = false;
		let mut read_count = 0;
		debug!("process read event= {}", handle)?;
		match ctx.handle_hash.get(&handle) {
			Some(id) => match ctx.id_hash.get_mut(&id) {
				Some(conn) => match conn {
					ConnectionVariant::ServerConnection(conn) => {
						Self::process_accept(conn, &mut accepted)?;
					}
					ConnectionVariant::ClientConnection(conn) => {
						(close, read_count) =
							Self::process_read(conn, config, callbacks, user_context)?;
					}
					ConnectionVariant::Connection(conn) => {
						(close, read_count) =
							Self::process_read(conn, config, callbacks, user_context)?;
					}
					ConnectionVariant::Wakeup(_wakeup) => {
						let mut buf = [0u8; 1000];
						loop {
							let rlen = read_impl(handle, &mut buf)?;
							debug!("wakeup read rlen = {:?}", rlen)?;
							cbreak!(rlen.is_none());
						}
					}
				},
				None => {
					warn!("none1")?;
				}
			},
			None => {
				warn!("none2")?;
			}
		}
		debug!("close was {}", close)?;
		if close {
			debug!("closing handle {}", handle)?;
			Self::process_close(handle, ctx, callbacks, user_context)?;
		}
		ctx.thread_stats.reads += read_count;

		Self::process_accepted_connections(accepted, config, state, &mut ctx.wakeups)
	}

	fn process_accepted_connections(
		accepted: Vec<Handle>,
		config: &EventHandlerConfig,
		state: &mut Array<Box<dyn LockBox<EventHandlerState>>>,
		wakeups: &mut Array<Wakeup>,
	) -> Result<(), Error> {
		debug!("accepted connections = {:?}", accepted)?;
		for accept in accepted {
			let accept_usize: usize = try_into!(accept)?;
			let tid = accept_usize % config.threads;
			let connection = Connection::new(
				accept,
				Some(wakeups[tid].clone()),
				Some(state[tid].clone()),
				ConnectionType::Connection,
			)?;

			{
				let mut state = state[tid].wlock()?;
				let guard = state.guard()?;
				(**guard)
					.nconnections
					.push_back(ConnectionVariant::Connection(connection));
			}

			wakeups[tid].wakeup()?;
		}
		Ok(())
	}

	fn process_read(
		conn: &mut Connection,
		config: &EventHandlerConfig,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		user_context: &mut UserContextImpl,
	) -> Result<(bool, usize), Error> {
		debug!("in process_read")?;
		let mut close = false;
		let mut read_count = 0;
		let handle = conn.handle();
		// loop through and read as many slabs as we can
		loop {
			let last_slab = conn.get_last_slab();
			let slab_offset = conn.get_slab_offset();
			let len = config.read_slab_size;
			let read_slab_next_offset = len.saturating_sub(4);
			let mut slab = if last_slab >= u32::MAX as usize {
				let mut slab = match user_context.read_slabs.allocate() {
					Ok(slab) => slab,
					Err(e) => {
						warn!("cannot allocate any more slabs due to: {}", e)?;
						// close connection
						close = true;
						break;
					}
				};
				let id = slab.id();
				debug!(
					"----------------------allocate a slab1----------------({})",
					id
				)?;
				// initialize connection with values
				conn.set_last_slab(id);
				conn.set_first_slab(id);
				conn.set_slab_offset(0);

				// set next pointer to u32::MAX (end of chain)
				slab.get_mut()[read_slab_next_offset..read_slab_next_offset + 4]
					.clone_from_slice(&u32::MAX.to_be_bytes());
				slab
			} else if slab_offset == read_slab_next_offset {
				let slab = match user_context.read_slabs.allocate() {
					Ok(slab) => slab,
					Err(e) => {
						warn!("cannot allocate any more slabs due to: {}", e)?;
						close = true;
						break;
					}
				};
				let slab_id = slab.id();
				debug!(
					"----------------------allocate a slab2----------------({})",
					slab_id
				)?;
				user_context.read_slabs.get_mut(last_slab)?.get_mut()
					[read_slab_next_offset..read_slab_next_offset + 4]
					.clone_from_slice(&(slab_id as u32).to_be_bytes());
				conn.set_last_slab(slab_id);
				conn.set_slab_offset(0);
				let mut ret = user_context.read_slabs.get_mut(slab_id)?;

				ret.get_mut()[read_slab_next_offset..read_slab_next_offset + 4]
					.clone_from_slice(&u32::MAX.to_be_bytes());

				ret
			} else {
				user_context.read_slabs.get_mut(last_slab)?
			};
			let slab_offset = conn.get_slab_offset();
			let rlen = read_impl(
				handle,
				&mut slab.get_mut()[slab_offset..read_slab_next_offset],
			)?;

			match rlen {
				Some(rlen) => {
					if rlen > 0 {
						conn.set_slab_offset(slab_offset + rlen);
						read_count += 1;
					}

					debug!(
						"rlen={},slab_id={},slab_offset={}",
						rlen,
						slab.id(),
						slab_offset + rlen
					)?;

					if rlen == 0 {
						debug!("connection closed")?;
						close = true;
						break;
					}
				}
				None => {
					debug!("no more data to read for now")?;
					// no more to read for now
					break;
				}
			}

			Self::call_on_read(user_context, conn, &mut callbacks.on_read)?;
		}

		Ok((close, read_count))
	}

	fn call_on_housekeeper(
		user_context: &mut UserContextImpl,
		callback: &mut Option<Pin<Box<OnHousekeeper>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		match callback {
			Some(ref mut on_housekeeper) => {
				let mut user_context: Box<dyn UserContext> = Box::new(user_context);
				match on_housekeeper(&mut user_context) {
					Ok(_) => {}
					Err(e) => warn!("on_housekeeper callback generated error: {}", e)?,
				}
			}
			None => {
				warn!("no on housekeeper handler!")?;
			}
		}
		Ok(())
	}

	fn call_on_read(
		user_context: &mut UserContextImpl,
		conn: &mut Connection,
		callback: &mut Option<Pin<Box<OnRead>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = conn.get_first_slab();
		match callback {
			Some(ref mut on_read) => {
				let mut user_context: Box<dyn UserContext> = Box::new(user_context);
				match on_read(conn, &mut user_context) {
					Ok(_) => {}
					Err(e) => warn!("on_read callback generated error: {}", e)?,
				}
			}
			None => {
				warn!("no on read handler!")?;
			}
		}
		Ok(())
	}

	fn call_on_accept(
		user_context: &mut UserContextImpl,
		conn: &mut Connection,
		callback: &mut Option<Pin<Box<OnAccept>>>,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		match callback {
			Some(ref mut on_accept) => {
				let mut user_context: Box<dyn UserContext> = Box::new(user_context);
				match on_accept(conn, &mut user_context) {
					Ok(_) => {}
					Err(e) => warn!("on_accept callback generated error: {}", e)?,
				}
			}
			None => {
				warn!("no on accept handler!")?;
			}
		}
		Ok(())
	}

	fn call_on_close(
		user_context: &mut UserContextImpl,
		handle: Handle,
		callback: &mut Option<Pin<Box<OnClose>>>,
		ctx: &mut EventHandlerContext,
	) -> Result<(), Error> {
		user_context.slab_cur = usize::MAX;
		match callback {
			Some(ref mut callback) => match ctx.handle_hash.get(&handle) {
				Some(id) => match ctx.id_hash.get_mut(id) {
					Some(conn) => match conn {
						ConnectionVariant::Connection(conn) => {
							let mut user_context: Box<dyn UserContext> = Box::new(user_context);
							match callback(conn, &mut user_context) {
								Ok(_) => {}
								Err(e) => warn!("on_close callback generated error: {}", e)?,
							}
						}
						ConnectionVariant::ClientConnection(conn) => {
							let mut user_context: Box<dyn UserContext> = Box::new(user_context);
							match callback(conn, &mut user_context) {
								Ok(_) => {}
								Err(e) => warn!("on_close callback generated error: {}", e)?,
							}
						}
						ConnectionVariant::ServerConnection(_conn) => {
							warn!(
								"unexpected on_close called on server connection tid = {}",
								ctx.tid
							)?;
						}
						ConnectionVariant::Wakeup(_wakeup) => {
							warn!("unexpected on_close called on wakeup tid = {}", ctx.tid)?;
						}
					},
					None => warn!("noneA")?,
				},
				None => warn!("noneB")?,
			},
			None => {
				warn!("noneC")?;
			}
		}
		Ok(())
	}

	fn process_close(
		handle: Handle,
		ctx: &mut EventHandlerContext,
		callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		mut user_context: &mut UserContextImpl,
	) -> Result<(), Error> {
		debug!("Calling close")?;
		ctx.thread_stats.closes += 1;
		Self::call_on_close(user_context, handle, &mut callbacks.on_close, ctx)?;

		let id = ctx.handle_hash.remove(&handle).unwrap_or(u128::MAX);
		debug!("removing handle={},id={}", handle, id)?;
		match ctx.id_hash.remove(&id) {
			Some(conn) => match conn {
				ConnectionVariant::Connection(mut conn) => {
					user_context.clear_through(conn.get_last_slab(), &mut conn)?;
				}
				ConnectionVariant::ClientConnection(mut conn) => {
					user_context.clear_through(conn.get_last_slab(), &mut conn)?;
				}
				ConnectionVariant::ServerConnection(_conn) => {
					warn!(
						"unexpected process_close called on server_connection tid = {}",
						ctx.tid
					)?;
				}
				ConnectionVariant::Wakeup(_wakeup) => {
					warn!(
						"unexpected process_close called on wakeup tid = {}",
						ctx.tid
					)?;
				}
			},
			None => warn!("expected a connection")?,
		}
		close_impl_ctx(handle, ctx)?;
		debug!("id hash rem")?;
		Ok(())
	}

	fn process_accept(conn: &Connection, accepted: &mut Vec<Handle>) -> Result<(), Error> {
		let handle = conn.handle();
		let id = conn.id();
		debug!("process read event on handle={},id={}", handle, id)?;
		loop {
			match accept_impl(handle) {
				Ok(next) => {
					cbreak!(next.is_none());
					accepted.push(next.unwrap());
				}
				Err(e) => {
					warn!("accept generated error: {}", e)?;
					break;
				}
			}
		}
		Ok(())
	}

	fn process_write_event(
		_config: &EventHandlerConfig,
		ctx: &mut EventHandlerContext,
		_callbacks: &mut EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
		handle: Handle,
	) -> Result<(), Error> {
		let mut close = false;
		let mut write_count = 0;
		match ctx.handle_hash.get(&handle) {
			Some(id) => match ctx.id_hash.get_mut(id) {
				Some(conn) => match conn {
					ConnectionVariant::Connection(conn) => {
						(close, write_count) = Self::write_loop(conn)?;
					}
					ConnectionVariant::ClientConnection(conn) => {
						(close, write_count) = Self::write_loop(conn)?;
					}
					_ => todo!(),
				},
				None => {
					warn!("id hash lookup failed for id: {}, handle: {}", id, handle)?;
				}
			},
			None => {
				warn!("handle lookup failed for  handle: {}", handle)?;
			}
		}

		if close {
			close_impl_ctx(handle, ctx)?;
		}

		ctx.thread_stats.delay_writes += write_count;
		Ok(())
	}

	fn write_loop(conn: &mut Connection) -> Result<(bool, usize), Error> {
		let mut write_count = 0;
		let mut wh = conn.write_handle()?;
		let write_state = wh.write_state()?;
		let mut write_state = write_state.wlock()?;
		let guard = write_state.guard()?;
		let mut close = false;
		let mut rem = true;
		loop {
			let len = (**guard).write_buffer.len();
			if len == 0 {
				rem = false;
				break;
			}
			let wlen = write_impl(conn.handle(), &(**guard).write_buffer)?;
			if wlen < 0 {
				let err = errno().0;
				if err != EAGAIN && err != ETEMPUNAVAILABLE && err != WINNONBLOCKING {
					close = true;
				}
				break;
			} else {
				let wlen: usize = try_into!(wlen)?;
				if wlen > 0 {
					write_count += 1;
				}

				(**guard).write_buffer.drain(0..wlen);
				(**guard).write_buffer.shrink_to_fit();
			}
		}

		if !rem {
			(**guard).unset_flag(WRITE_STATE_FLAG_PENDING);
		}

		Ok((close, write_count))
	}

	fn stop(&mut self) -> Result<(), Error> {
		// stop thread pool and all threads
		match &mut self.stopper {
			Some(ref mut stopper) => stopper.stop()?,
			None => {}
		}
		for i in 0..self.config.threads {
			wlock!(self.state[i]).stop = true;
			self.wakeups[i].wakeup()?;
		}

		Ok(())
	}
}

impl Event {
	pub(crate) fn new(handle: Handle, etype: EventType) -> Self {
		Self { handle, etype }
	}

	fn empty() -> Self {
		Self {
			etype: EventType::Read,
			handle: 0,
		}
	}
}

impl EventIn {
	pub(crate) fn new(handle: Handle, etype: EventTypeIn) -> Self {
		Self { handle, etype }
	}
}

impl EventHandlerContext {
	fn new(
		wakeups: Array<Wakeup>,
		tid: usize,
		global_stats: Box<dyn LockBox<GlobalStats>>,
	) -> Result<Self, Error> {
		let ret_event_count = 0;
		let ret_events = [Event::empty(); MAX_RET_HANDLES];
		let in_events = vec![];
		let id_hash = HashMap::new();
		let handle_hash = HashMap::new();

		Ok(Self {
			ret_event_count,
			ret_events,
			in_events,
			id_hash,
			handle_hash,
			wakeups,
			tid,
			last_housekeeping: 0,
			trigger_on_read_list: vec![],
			trigger_itt: 0,
			ret_event_itt: 0,
			thread_stats: EvhStats::new(),
			global_stats,
			last_stats_update: 0,
			#[cfg(target_os = "linux")]
			linux_ctx: LinuxContext::new()?,
			#[cfg(target_os = "macos")]
			macos_ctx: MacosContext::new()?,
		})
	}
}

impl EvhStats {
	fn new() -> Self {
		Self {
			accepts: 0,
			closes: 0,
			reads: 0,
			delay_writes: 0,
			event_loops: 0,
		}
	}

	fn reset(&mut self) {
		self.accepts = 0;
		self.closes = 0;
		self.reads = 0;
		self.delay_writes = 0;
		self.event_loops = 0;
	}

	fn incr_stats(&mut self, stats: &EvhStats) {
		self.accepts += stats.accepts;
		self.closes += stats.closes;
		self.reads += stats.reads;
		self.delay_writes += stats.delay_writes;
		self.event_loops += stats.event_loops;
	}
}
