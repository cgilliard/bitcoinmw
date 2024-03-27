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

#[cfg(target_os = "macos")]
use crate::mac::*;
#[cfg(target_os = "windows")]
use crate::win::*;

#[cfg(target_os = "linux")]
use crate::linux::*;

use crate::constants::*;
use bmw_err::*;
use bmw_util::*;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::mpsc::SyncSender;

pub trait EventHandler<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
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
	fn start(&mut self) -> Result<(), Error>;
	fn set_on_read(&mut self, on_read: OnRead) -> Result<(), Error>;
	fn set_on_accept(&mut self, on_accept: OnAccept) -> Result<(), Error>;
	fn set_on_close(&mut self, on_close: OnClose) -> Result<(), Error>;
	fn set_on_housekeeper(&mut self, on_housekeeper: OnHousekeeper) -> Result<(), Error>;
	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error>;
	fn add_server_connection(
		&mut self,
		connection: Box<dyn ServerConnection + Send + Sync>,
	) -> Result<(), Error>;
	fn add_client_connection(
		&mut self,
		connection: Box<dyn ClientConnection + Send + Sync>,
	) -> Result<Box<dyn WriteHandle + Send + Sync>, Error>;
}

pub trait ClientConnection: Connection {
	fn as_connection(&mut self) -> Box<dyn Connection + '_ + Send + Sync>;
	fn set_state(&mut self, state: Box<dyn LockBox<EventHandlerState>>) -> Result<(), Error>;
	fn set_wakeup(&mut self, wakeup: Wakeup) -> Result<(), Error>;
	fn set_tx(&mut self, tx: SyncSender<()>);
	fn get_tx(&mut self) -> Option<&mut SyncSender<()>>;
}

pub trait ServerConnection: Connection {
	fn as_connection(&mut self) -> Box<dyn Connection + '_ + Send + Sync>;
	fn set_tx(&mut self, tx: SyncSender<()>);
	fn get_tx(&mut self) -> Option<&mut SyncSender<()>>;
}

pub trait Connection {
	fn handle(&self) -> Handle;
	fn id(&self) -> u128;
	fn get_slab_offset(&self) -> usize;
	fn get_first_slab(&self) -> usize;
	fn get_last_slab(&self) -> usize;
	fn set_slab_offset(&mut self, offset: usize);
	fn set_first_slab(&mut self, first_slab: usize);
	fn set_last_slab(&mut self, last_slab: usize);
	fn write_handle(&self) -> Result<Box<dyn WriteHandle + Send + Sync>, Error>;
}

pub trait WriteHandle {
	fn write(&mut self, data: &[u8]) -> Result<(), Error>;
	fn close(&mut self) -> Result<(), Error>;
	fn trigger_on_read(&mut self) -> Result<(), Error>;
	fn is_set(&self, flag: u8) -> Result<bool, Error>;
	fn set_flag(&mut self, flag: u8) -> Result<(), Error>;
	fn unset_flag(&mut self, flag: u8) -> Result<(), Error>;
	fn write_state(&mut self) -> Result<&mut Box<dyn LockBox<WriteState>>, Error>;
}

pub trait UserContext {
	fn clone_next_chunk(
		&mut self,
		connection: &mut Box<dyn Connection + '_ + Send + Sync>,
		buf: &mut [u8],
	) -> Result<usize, Error>;
	fn cur_slab_id(&self) -> usize;
	fn clear_all(
		&mut self,
		connection: &mut Box<dyn Connection + '_ + Send + Sync>,
	) -> Result<(), Error>;
	fn clear_through(
		&mut self,
		slab_id: usize,
		connection: &mut Box<dyn Connection + '_ + Send + Sync>,
	) -> Result<(), Error>;
	fn get_user_data(&mut self) -> &mut Option<Box<dyn Any>>;
	fn set_user_data(&mut self, user_data: Box<dyn Any>);
}

pub struct EvhBuilder {}

pub struct EventHandlerState {
	pub(crate) nconnections: VecDeque<ConnectionVariant>,
	pub(crate) write_queue: VecDeque<u128>,
	pub(crate) stop: bool,
}

#[derive(Clone)]
pub struct Wakeup {
	pub(crate) lock: Box<dyn LockBox<bool>>,
	pub(crate) lock2: Box<dyn LockBox<()>>,
	pub(crate) reader: Handle,
	pub(crate) writer: Handle,
	pub(crate) id: u128,
}

// crate local structures

pub(crate) struct WriteHandleImpl {
	pub(crate) handle: Handle,
	pub(crate) id: u128,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) wakeup: Wakeup,
	pub(crate) state: Box<dyn LockBox<EventHandlerState>>,
}

pub struct WriteState {
	pub(crate) flags: u8,
	pub(crate) write_buffer: Vec<u8>,
}

pub(crate) struct ConnectionImpl {
	pub(crate) handle: Handle,
	pub(crate) id: u128,
	pub(crate) slab_offset: usize,
	pub(crate) first_slab: usize,
	pub(crate) last_slab: usize,
	pub(crate) write_state: Box<dyn LockBox<WriteState>>,
	pub(crate) wakeup: Option<Wakeup>,
	pub(crate) state: Option<Box<dyn LockBox<EventHandlerState>>>,
	pub(crate) tx: Option<SyncSender<()>>,
}
pub(crate) struct UserContextImpl {
	pub(crate) read_slabs: Box<dyn SlabAllocator + Send + Sync>,
	pub(crate) user_data: Option<Box<dyn Any>>,
	pub(crate) slab_cur: usize,
}

#[derive(Clone)]
pub(crate) struct EventHandlerConfig {
	pub(crate) threads: usize,
	pub(crate) debug: bool,
	pub(crate) timeout: u16,
	pub(crate) read_slab_size: usize,
	pub(crate) read_slab_count: usize,
	pub(crate) housekeeping_frequency_millis: usize,
}
pub(crate) struct EventHandlerImpl<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
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
	pub(crate) callbacks: EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>,
	pub(crate) config: EventHandlerConfig,
	pub(crate) state: Array<Box<dyn LockBox<EventHandlerState>>>,
	pub(crate) wakeups: Array<Wakeup>,
	pub(crate) stopper: Option<ThreadPoolStopper>,
}

#[derive(Clone)]
pub(crate) struct EventHandlerCallbacks<OnRead, OnAccept, OnClose, OnHousekeeper, OnPanic>
where
	OnRead: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnAccept: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	OnClose: FnMut(
			&mut Box<dyn Connection + '_ + Send + Sync>,
			&mut Box<dyn UserContext + '_>,
		) -> Result<(), Error>
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
	pub(crate) on_read: Option<Pin<Box<OnRead>>>,
	pub(crate) on_accept: Option<Pin<Box<OnAccept>>>,
	pub(crate) on_close: Option<Pin<Box<OnClose>>>,
	pub(crate) on_panic: Option<Pin<Box<OnPanic>>>,
	pub(crate) on_housekeeper: Option<Pin<Box<OnHousekeeper>>>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum EventType {
	Read,
	Write,
	ReadWrite,
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Event {
	pub(crate) handle: Handle,
	pub(crate) etype: EventType,
}

#[derive(PartialEq)]
pub(crate) enum EventTypeIn {
	Accept,
	Read,
	Write,
	Suspend,
	Resume,
}

pub(crate) struct EventIn {
	pub(crate) handle: Handle,
	pub(crate) etype: EventTypeIn,
}

pub(crate) struct EventHandlerContext {
	pub(crate) ret_event_count: usize,
	pub(crate) ret_events: [Event; MAX_RET_HANDLES],
	pub(crate) in_events: Vec<EventIn>,
	pub(crate) handle_hash: HashMap<Handle, u128>,
	pub(crate) id_hash: HashMap<u128, ConnectionVariant>,
	pub(crate) wakeups: Array<Wakeup>,
	pub(crate) tid: usize,
	pub(crate) last_housekeeping: usize,

	#[cfg(target_os = "linux")]
	pub(crate) linux_ctx: LinuxContext,
}

pub(crate) enum ConnectionVariant {
	ServerConnection(Box<dyn ServerConnection + Send + Sync>),
	ClientConnection(Box<dyn ClientConnection + Send + Sync>),
	Connection(Box<dyn Connection + Send + Sync>),
	Wakeup(Wakeup),
}