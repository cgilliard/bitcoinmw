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

use crate::constants::*;
use crate::types::{
	FutureWrapper, Lock, ThreadPoolConfig, ThreadPoolHandle, ThreadPoolImpl, ThreadPoolState,
};
use crate::{LockBox, PoolResult, ThreadPool, ThreadPoolExecutor, ThreadPoolStopper, UtilBuilder};
use bmw_conf::ConfigOptionName as CN;
use bmw_conf::{ConfigBuilder, ConfigOption};
use bmw_deps::futures::executor::block_on;
use bmw_err::{cbreak, err, Error};
use bmw_log::*;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;

info!();

impl<T, E> PoolResult<T, E> {
	pub fn is_err(&self) -> bool {
		match self {
			PoolResult::Err(_) => true,
			PoolResult::Ok(_) => false,
			PoolResult::Panic => true,
		}
	}
}

impl<T> ThreadPoolHandle<T> {
	fn new(id: u128, recv_handle: Receiver<PoolResult<T, Error>>) -> Self {
		Self { id, recv_handle }
	}

	pub fn id(&self) -> u128 {
		self.id
	}

	pub fn block_on(&self) -> PoolResult<T, Error> {
		match self.recv_handle.recv() {
			Ok(res) => res,
			Err(e) => PoolResult::Err(err!(
				ErrKind::ThreadPanic,
				format!("thread pool panic: {}", e)
			)),
		}
	}
}

unsafe impl<T, E> Send for PoolResult<T, E> {}
unsafe impl<T, E> Sync for PoolResult<T, E> {}

impl<T, OnPanic> ThreadPoolImpl<T, OnPanic>
where
	OnPanic: FnMut(u128, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
	T: 'static + Send + Sync,
{
	pub(crate) fn new(configs: Vec<ConfigOption>) -> Result<Self, Error> {
		let config = ConfigBuilder::build_config(configs);
		config.check_config(vec![CN::SyncChannelSize, CN::MinSize, CN::MaxSize], vec![])?;
		let min_size = THREAD_POOL_DEFAULT_MIN_SIZE;
		let min_size = config.get_or_usize(&CN::MinSize, min_size);
		let max_size = config.get_or_usize(&CN::MaxSize, min_size);
		let sync_channel_size = THREAD_POOL_DEFAULT_SYNC_CHANNEL_SIZE;
		let sync_channel_size = config.get_or_usize(&CN::SyncChannelSize, sync_channel_size);

		if min_size == 0 || min_size > max_size {
			let fmt = "min_size must be > 0 and <= max_size";
			Err(err!(ErrKind::Configuration, fmt))
		} else if sync_channel_size == 0 {
			let fmt = "sync_channel_size must be greater than 0";
			Err(err!(ErrKind::Configuration, fmt))
		} else {
			let config = ThreadPoolConfig {
				min_size,
				max_size,
				sync_channel_size,
			};

			let waiting = 0;
			let stop = false;
			let tps = ThreadPoolState {
				waiting,
				cur_size: min_size,
				config: config.clone(),
				stop,
			};
			let state = UtilBuilder::build_lock_box(tps)?;

			let rx = None;
			let tx = None;

			let ret = Self {
				config,
				tx,
				rx,
				state,
				on_panic: None,
			};
			Ok(ret)
		}
	}

	fn run_thread<R: 'static>(
		rx: Arc<Mutex<Receiver<FutureWrapper<R>>>>,
		mut state: Box<dyn LockBox<ThreadPoolState>>,
		mut on_panic: Option<Pin<Box<OnPanic>>>,
	) -> Result<(), Error> {
		spawn(move || -> Result<(), Error> {
			loop {
				let rx = rx.clone();
				let mut state_clone = state.clone();
				let on_panic_clone = on_panic.clone();
				let mut id = UtilBuilder::build_lock(0)?;
				let id_clone = id.clone();
				let jh = spawn(move || -> Result<(), Error> {
					loop {
						let (next, do_run_thread) = {
							let mut do_run_thread = false;
							{
								let mut state = state_clone.wlock()?;
								let guard = &mut **state.guard()?;

								debug!("state = {:?}", guard)?;
								// we have too many threads or stop
								// was set. Exit this one.
								if guard.stop || guard.waiting >= guard.config.min_size {
									return Ok(());
								}
								guard.waiting += 1;
							}
							let rx = rx.lock()?;
							let ret = rx.recv()?;
							let mut state = state_clone.wlock()?;
							let guard = &mut **state.guard()?;
							guard.waiting = guard.waiting.saturating_sub(1);
							if guard.waiting == 0 {
								if guard.cur_size < guard.config.max_size {
									guard.cur_size += 1;
									do_run_thread = true;
								}
							}
							debug!("cur state = {:?}", guard)?;
							(ret, do_run_thread)
						};

						if do_run_thread {
							debug!("spawning a new thread")?;
							Self::run_thread(
								rx.clone(),
								state_clone.clone(),
								on_panic_clone.clone(),
							)?;
						}

						{
							let mut id = id.wlock()?;
							let guard = id.guard()?;
							(**guard) = next.id;
						}
						match block_on(next.f) {
							Ok(res) => {
								let send_res = next.tx.send(PoolResult::Ok(res));
								if send_res.is_err() {
									let e = send_res.unwrap_err();
									debug!("error sending response: {}", e)?;
								}
							}
							Err(e) => {
								debug!("sending an err")?;
								// if the reciever is not there we
								// just ignore the error that would
								// occur
								let _ = next.tx.send(PoolResult::Err(e));
							}
						}
					}
				});

				let res = jh.join();
				if res.is_ok() {
					let mut state = state.wlock()?;
					let guard = &mut **state.guard()?;
					guard.cur_size = guard.cur_size.saturating_sub(1);
					debug!("exiting a thread, ncur={}", guard.cur_size)?;
					cbreak!(true);
				} else {
					let e = res.unwrap_err();
					if on_panic.is_some() {
						let on_panic = on_panic.as_mut().unwrap();
						debug!("found an onpanic")?;
						let id = id_clone.rlock()?;
						let guard = id.guard()?;
						let res = on_panic(**guard, e);
						if res.is_err() {
							let e = res.unwrap_err();
							warn!("on_panic handler generated error: {}", e)?;
						}
					}
				}
			}
			Ok(())
		});
		Ok(())
	}
}

impl<T, OnPanic> ThreadPool<T, OnPanic> for ThreadPoolImpl<T, OnPanic>
where
	T: 'static + Send + Sync,
	OnPanic: FnMut(u128, Box<dyn Any + Send>) -> Result<(), Error>
		+ Send
		+ 'static
		+ Clone
		+ Sync
		+ Unpin,
{
	fn execute<F>(&self, f: F, id: u128) -> Result<ThreadPoolHandle<T>, Error>
	where
		F: Future<Output = Result<T, Error>> + Send + 'static,
	{
		if self.tx.is_none() {
			let fmt = "Thread pool has not been initialized";
			return Err(err!(ErrKind::IllegalState, fmt));
		}

		let (tx, rx) = sync_channel::<PoolResult<T, Error>>(1);
		let fw = FutureWrapper {
			f: Box::pin(f),
			tx,
			id,
		};
		self.tx.as_ref().unwrap().send(fw)?;
		Ok(ThreadPoolHandle::new(id, rx))
	}

	fn start(&mut self) -> Result<(), Error> {
		if self.tx.is_some() {
			return Err(err!(ErrKind::IllegalState, "thread pool already started"));
		}
		let (tx, rx) = sync_channel(self.config.sync_channel_size);
		let rx = Arc::new(Mutex::new(rx));
		self.rx = Some(rx.clone());
		self.tx = Some(tx.clone());

		for _ in 0..self.config.min_size {
			Self::run_thread(rx.clone(), self.state.clone(), self.on_panic.clone())?;
		}

		let mut count = 0;
		loop {
			if count > 0 {
				let state = self.state.rlock()?;
				let guard = &**state.guard()?;
				cbreak!(guard.waiting == self.config.min_size);
			}
			sleep(Duration::from_millis(1));
			count += 1;
		}

		Ok(())
	}

	fn stop(&mut self) -> Result<(), Error> {
		let mut state = self.state.wlock()?;
		(**state.guard()?).stop = true;
		self.tx = None;
		Ok(())
	}

	fn size(&self) -> Result<usize, Error> {
		let state = self.state.rlock()?;
		Ok((**state.guard()?).cur_size)
	}

	fn stopper(&self) -> Result<ThreadPoolStopper, Error> {
		Ok(ThreadPoolStopper {
			state: self.state.clone(),
		})
	}

	fn executor(&self) -> Result<ThreadPoolExecutor<T>, Error> {
		Ok(ThreadPoolExecutor {
			tx: self.tx.clone(),
		})
	}

	fn set_on_panic(&mut self, on_panic: OnPanic) -> Result<(), Error> {
		self.on_panic = Some(Box::pin(on_panic));
		Ok(())
	}

	#[cfg(test)]
	fn set_on_panic_none(&mut self) -> Result<(), Error> {
		self.on_panic = None;
		Ok(())
	}
}

impl<T> ThreadPoolExecutor<T>
where
	T: Send + Sync,
{
	pub fn execute<F>(&self, f: F, id: u128) -> Result<Receiver<PoolResult<T, Error>>, Error>
	where
		F: Future<Output = Result<T, Error>> + Send + 'static,
	{
		if self.tx.is_none() {
			let fmt = "Thread pool has not been initialized";
			return Err(err!(ErrKind::IllegalState, fmt));
		}

		let (tx, rx) = sync_channel::<PoolResult<T, Error>>(1);
		let fw = FutureWrapper {
			f: Box::pin(f),
			tx,
			id,
		};
		self.tx.as_ref().unwrap().send(fw)?;
		Ok(rx)
	}
}

impl ThreadPoolStopper {
	/// Stop all threads in the thread pool from executing new tasks.
	/// note that this does not terminate the threads if they are idle, it
	/// will just make the threads end after their next task is executed.
	/// The main purpose of this function is so that the state can be stored
	/// in a struct, but caller must ensure that the threads stop.
	/// This is not the case with [`crate::ThreadPool::stop`] and that function
	/// should be used where possible.
	pub fn stop(&mut self) -> Result<(), Error> {
		(**self.state.wlock()?.guard()?).stop = true;
		Ok(())
	}
}
