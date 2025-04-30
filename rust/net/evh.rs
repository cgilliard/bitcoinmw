#![allow(dead_code)]
#![allow(unused_variables)]

use core::marker::PhantomData;
use core::ops::FnMut;
use net::multiplex::Multiplex;
use net::socket::Socket;
use prelude::*;
use util::channel::{Receiver, Sender};

type OnRecv<T> = Box<dyn FnMut(&mut T, &mut Connection<T>, &[u8]) -> Result<()>>;
type OnAccept<T> = Box<dyn FnMut(&mut T, &Connection<T>) -> Result<()>>;
type OnClose<T> = Box<dyn FnMut(&mut T, &Connection<T>) -> Result<()>>;
type OnWritable<T> = Box<dyn FnMut(&mut Connection<T>) -> Result<()>>;

struct AcceptorData<T>
where
	T: Clone,
{
	socket: Socket,
	on_recv: Rc<OnRecv<T>>,
	on_accept: Rc<OnAccept<T>>,
	on_close: Rc<OnClose<T>>,
	attach: T,
}

struct InboundData<T>
where
	T: Clone,
{
	socket: Socket,
	acceptor: Connection<T>,
	is_closed: bool,
	lock: Lock,
	multiplex: Multiplex,
	on_writable: Option<OnWritable<T>>,
}

struct OutboundData<T>
where
	T: Clone,
{
	socket: Socket,
	on_recv: Rc<OnRecv<T>>,
	on_close: Rc<OnClose<T>>,
	attach: T,
}

enum ConnectionData<T>
where
	T: Clone,
{
	Inbound(InboundData<T>),
	Outbound(OutboundData<T>),
	Acceptor(AcceptorData<T>),
	Close,
}

#[derive(Clone)]
pub struct Connection<T>
where
	T: Clone,
{
	inner: Rc<ConnectionData<T>>,
}

struct CloseData {
	flag: bool,
	port: u16,
	lock: Lock,
	socket: Socket,
	recv: Receiver<()>,
	send: Sender<()>,
}

pub struct Evh<T>
where
	T: Clone,
{
	multiplex: Multiplex,
	close: Rc<CloseData>,
	_phantom_data: PhantomData<T>,
}

impl<T> Connection<T>
where
	T: Clone,
{
	pub fn acceptor(
		socket: Socket,
		on_recv: Rc<OnRecv<T>>,
		on_accept: Rc<OnAccept<T>>,
		on_close: Rc<OnClose<T>>,
		attach: T,
	) -> Result<Self> {
		err!(Todo)
	}

	pub fn outbound(
		socket: Socket,
		on_recv: Rc<OnRecv<T>>,
		on_close: Rc<OnClose<T>>,
		attach: T,
	) -> Result<Self> {
		err!(Todo)
	}

	pub fn socket(&self) -> Socket {
		todo!()
	}

	pub fn write(&self, b: &[u8]) -> Result<usize> {
		err!(Todo)
	}

	pub fn close(&self) -> Result<()> {
		err!(Todo)
	}

	pub fn on_writable(&mut self, on_writable: OnWritable<T>) -> Result<()> {
		err!(Todo)
	}

	pub unsafe fn drop_rc(&mut self) {}
}

impl<T> Evh<T>
where
	T: Clone,
{
	pub fn new() -> Result<Self> {
		err!(Todo)
	}

	pub fn register(&mut self, conn: Connection<T>) -> Result<()> {
		err!(Todo)
	}

	pub fn stop(&mut self) -> Result<()> {
		err!(Todo)
	}

	pub fn start(&mut self) -> Result<()> {
		err!(Todo)
	}
}
