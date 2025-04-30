use core::marker::PhantomData;
use core::ptr;
use prelude::*;
use std::ffi::release;
use util::ffi::{
	channel_destroy, channel_handle_size, channel_init, channel_pending, channel_recv, channel_send,
};

#[repr(C)]
struct ChannelMessage<T> {
	_reserved: u64,
	value: T,
}

struct ChannelInner<T> {
	handle: [u8; 128],
	_marker: PhantomData<T>,
}

pub struct Sender<T> {
	inner: Rc<ChannelInner<T>>,
}

pub struct Receiver<T> {
	inner: Rc<ChannelInner<T>>,
}

pub fn channel<T>() -> Result<(Sender<T>, Receiver<T>)> {
	if unsafe { channel_handle_size() } > 128 {
		exit!("channel_handle_size() > 128");
	}
	let handle = [0u8; 128];
	let send_inner = match Rc::new(ChannelInner {
		handle,
		_marker: PhantomData,
	}) {
		Ok(inner) => inner,
		Err(e) => return Err(e),
	};

	let mut recv_inner = send_inner.clone();

	if unsafe { channel_init(&mut recv_inner.handle as *mut u8) } < 0 {
		err!(ChannelInit)
	} else {
		Ok((Sender { inner: send_inner }, Receiver { inner: recv_inner }))
	}
}

impl<T> Drop for ChannelInner<T> {
	fn drop(&mut self) {
		while self.pending() {
			let _recv = self.recv();
		}
		let handle = &self.handle;
		unsafe {
			channel_destroy(handle as *const u8);
		}
	}
}

impl<T> ChannelInner<T> {
	pub fn recv(&self) -> T {
		let handle = &self.handle;
		let recv = unsafe { channel_recv(handle as *const u8) } as *mut ChannelMessage<T>;
		let ptr = Ptr::new(recv);
		let mut nbox = unsafe { Box::from_raw(ptr) };
		unsafe {
			nbox.leak();
		}
		let v = unsafe { ptr::read(nbox.as_ptr().raw()) };
		unsafe { release(recv as *mut u8) };
		v.value
	}

	pub fn send(&self, value: T) -> Result<()> {
		let msg = ChannelMessage {
			_reserved: 0,
			value,
		};
		match Box::new(msg) {
			Ok(mut b) => {
				unsafe {
					b.leak();
				}
				let handle = &self.handle;
				if unsafe { channel_send(handle as *const u8, b.as_ptr().raw() as *mut u8) } < 0 {
					err!(ChannelSend)
				} else {
					Ok(())
				}
			}
			Err(e) => Err(e),
		}
	}

	pub fn pending(&self) -> bool {
		unsafe { channel_pending(&self.handle as *const u8) != 0 }
	}
}

impl<T> Clone for Sender<T> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl<T> Clone for Receiver<T> {
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
		}
	}
}

impl<T> Sender<T> {
	pub fn send(&self, value: T) -> Result<()> {
		self.inner.send(value)
	}
}

impl<T> Receiver<T> {
	pub fn recv(&self) -> T {
		self.inner.recv()
	}

	pub fn pending(&self) -> bool {
		self.inner.pending()
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::ffi::sleep_millis;

	#[test]
	fn test_channel_std() {
		{
			let (sender, receiver) = channel().unwrap();
			let lock = lock!();
			let rc = Rc::new(1).unwrap();
			let mut rc_clone = rc.clone();
			let mut jh = spawnj(|| {
				let v = receiver.recv();
				assert_eq!(v, 101);
				let _v = lock.write();
				assert_eq!(*rc_clone, 1);
				*rc_clone += 1;
				assert_eq!(*rc_clone, 2);
			})
			.unwrap();

			sender.send(101).unwrap();

			loop {
				{
					let _v = lock.read();
					if *rc != 1 {
						assert_eq!(*rc, 2);
						break;
					}
				}
				unsafe {
					sleep_millis(100);
				}
			}
			assert!(jh.join().is_ok());
		}
	}

	#[test]
	fn test_channel_clone() {
		{
			let (sender, receiver) = channel().unwrap();
			let _sender2: Sender<i32> = sender.clone();
			let _recevier2: Receiver<i32> = receiver.clone();
		}
	}

	#[test]
	fn test_channel_move_std() {
		{
			let (sender, receiver) = channel().unwrap();
			let lock = lock_box!().unwrap();
			let lock_clone = lock.clone();
			let rc = Rc::new(1).unwrap();
			let mut rc_clone = rc.clone();
			let mut jh = spawnj(move || {
				let v = receiver.recv();
				assert_eq!(v, 101);
				let _v = lock_clone.write();
				assert_eq!(*rc_clone, 1);
				*rc_clone += 1;
				assert_eq!(*rc_clone, 2);
			})
			.unwrap();

			sender.send(101).unwrap();

			loop {
				{
					let _v = lock.read();
					if *rc == 1 {
					} else {
						assert_eq!(*rc, 2);
						break;
					}
				}
				unsafe {
					sleep_millis(1);
				}
			}
			assert!(jh.join().is_ok());
		}
	}

	#[test]
	fn test_channel_result() {
		{
			let (sender, receiver) = channel().unwrap();
			let (sender2, receiver2) = channel().unwrap();
			let lock = lock_box!().unwrap();
			let lock_clone = lock.clone();
			let rc = Rc::new(0).unwrap();
			let mut rc_clone = rc.clone();

			let mut jh = spawnj(move || {
				{
					let input = receiver.recv();
					let _v = lock_clone.write();
					*rc_clone = input + 100;
				}
				sender2.send(()).unwrap();
			})
			.unwrap();

			sender.send(301).unwrap();
			let result = receiver2.recv();

			assert_eq!(result, ());
			assert_eq!(*rc, 401);

			assert!(jh.join().is_ok());
		}
	}

	struct DropTest {
		x: u32,
	}

	static mut DROPCOUNT: u32 = 0;
	static mut DROPSUM: u32 = 0;

	impl Drop for DropTest {
		fn drop(&mut self) {
			unsafe {
				DROPCOUNT += 1;
				DROPSUM += self.x;
			}
		}
	}

	#[test]
	fn test_channel_drop() {
		{
			let (sender, receiver) = channel().unwrap();
			let (sender2, receiver2) = channel().unwrap();
			let lock = lock_box!().unwrap();
			let lock_clone = lock.clone();
			let rc = Rc::new(0).unwrap();
			let mut rc_clone = rc.clone();

			let mut jh = spawnj(move || {
				{
					let input: DropTest = receiver.recv();
					let _v = lock_clone.write();
					*rc_clone = input.x + 100;
					assert_eq!(unsafe { DROPCOUNT }, 0);
				}
				assert_eq!(unsafe { DROPCOUNT }, 1);
				sender2.send(DropTest { x: 4 }).unwrap();
			})
			.unwrap();

			sender.send(DropTest { x: 301 }).unwrap();
			let result = receiver2.recv();

			assert_eq!(result.x, 4);
			assert_eq!(*rc, 401);
			assert!(jh.join().is_ok());
			assert_eq!(unsafe { DROPCOUNT }, 1);
		}
		assert_eq!(unsafe { DROPCOUNT }, 2);
		assert_eq!(unsafe { DROPSUM }, 305);
	}

	#[test]
	fn test_cleanup() {
		{
			let (send, _recv) = channel().unwrap();
			send.send(0).unwrap();
			send.send(0).unwrap();
		}
	}

	#[test]
	fn test_multisend_chan() {
		{
			let (channel, recv) = channel().unwrap();
			channel.send(0).unwrap();
			channel.send(1).unwrap();
			channel.send(2).unwrap();
			channel.send(3).unwrap();
			channel.send(4).unwrap();
			channel.send(5).unwrap();

			assert_eq!(recv.recv(), 0);
			assert_eq!(recv.recv(), 1);
			assert_eq!(recv.recv(), 2);

			// still pending at this point
			assert!(recv.pending());
		}
	}
}
