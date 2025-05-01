use net::errors::*;
use net::evh::*;
use net::socket::*;
use prelude::*;
use std::cstring::CString;
use std::ffi::getmicros;

fn exec_server() -> Result<()> {
	let mut evh: Evh<u64> = Evh::new()?;

	let port = 9090;
	let mut s = Socket::listen([127, 0, 0, 1], 9090, 10)?;
	let recv: OnRecv<u64> = Box::new(
		move |attach: &mut u64, conn: &mut Connection<u64>, bytes: &[u8]| -> Result<()> {
			conn.write(bytes)?;
			Ok(())
		},
	)?;
	let accept: OnAccept<u64> =
		Box::new(move |attach: &mut u64, conn: &Connection<u64>| -> Result<()> { Ok(()) })?;
	let close: OnClose<u64> =
		Box::new(move |attach: &mut u64, conn: &Connection<u64>| -> Result<()> { Ok(()) })?;

	let rc_close = Rc::new(close)?;
	let rc_accept = Rc::new(accept)?;
	let rc_recv = Rc::new(recv)?;

	let mut server = Connection::acceptor(s, rc_recv, rc_accept, rc_close, 0u64)?;
	evh.register(server.clone())?;

	evh.start()?;

	park();

	Ok(())
}

fn exec_client() -> Result<()> {
	let port = 9090;
	let lock = lock_box!()?;
	let mut count = Rc::new(0)?;
	let start = unsafe { getmicros() };
	let recv_client: OnRecv<u64> = Box::new(
		move |attach: &mut u64, conn: &mut Connection<u64>, bytes: &[u8]| -> Result<()> {
			println!("recv resp");
			let _l = lock.write();
			*count += 1;
			if *count <= 5 {
				conn.write(b"test")?;
			} else {
				println!("compelte in {}us", unsafe { getmicros() } - start);
			}
			Ok(())
		},
	)?;
	let close_client: OnClose<u64> =
		Box::new(move |attach: &mut u64, conn: &Connection<u64>| -> Result<()> { Ok(()) })?;
	let rc_recv_client = Rc::new(recv_client)?;
	let rc_close_client = Rc::new(close_client)?;
	let mut client = Socket::connect([127, 0, 0, 1], port)?;
	let mut connector = Connection::outbound(client, rc_recv_client, rc_close_client, 1u64)?;
	let mut evh = Evh::new()?;
	evh.register(connector.clone())?;
	evh.start()?;

	loop {
		match connector.write(b"test") {
			Ok(v) => {
				if v != 4 {
					exit!("len !+ 4");
				}
				break;
			}
			Err(e) => {
				if e == EAgain {
					println!("eagain1");
					continue;
				} else {
					return err!(e);
				}
			}
		}
	}

	println!("sent");
	park();

	//println!("success {}s", time as f64 / 1_000_000 as f64);
	Ok(())
}

fn proc_args(argc: i32, argv: *const *const u8) -> Result<()> {
	let arg2 = unsafe { CString::from_ptr(*(argv.add(1)), true) };
	if arg2.as_str()? == String::new("server")? {
		println!("Starting server!");
		exec_server()?;
	} else if arg2.as_str()? == String::new("client")? {
		println!("Starting client!");
		exec_client()?;
	}
	Ok(())
}

#[no_mangle]
pub extern "C" fn real_main(argc: i32, argv: *const *const u8) -> i32 {
	if argc == 2 {
		match proc_args(argc, argv) {
			Ok(_) => {}
			Err(e) => println!("proc_args generated error: {}", e),
		}
	}
	0
}

#[cfg(test)]
mod test {
	use super::*;
	use core::ptr::null;

	#[test]
	fn test_real_main() {
		assert_eq!(real_main(0, null()), 0);
	}
}
