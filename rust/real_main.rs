use net::errors::*;
use net::evh::*;
use net::socket::*;
use prelude::*;
use std::cstring::CString;
use std::ffi::getmicros;

fn exec_server() -> Result<()> {
	let mut evh: Evh<u64, u64> = Evh::new()?;

	let port = 9090;
	let s = Socket::listen([127, 0, 0, 1], port, 10)?;
	let recv: OnRecv<u64, u64> = Box::new(
		move |_ctx: &mut u64, conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
			let mut wsum = 0;
			loop {
				if wsum < bytes.len() {
					match conn.write(&bytes[wsum..]) {
						Ok(v) => {
							wsum += v;
							if wsum == bytes.len() {
								break;
							}
						}
						Err(e) => {
							if e != EAgain {
								let _ = conn.close();
								println!("socket err, closing connection: {}", e);
								break;
							}
						}
					}
				}
			}

			Ok(())
		},
	)?;
	let accept: OnAccept<u64, u64> =
		Box::new(move |_ctx: &mut u64, _conn: &mut Connection<u64, u64>| -> Result<()> { Ok(()) })?;
	let close: OnClose<u64, u64> =
		Box::new(move |_ctx: &mut u64, _conn: &mut Connection<u64, u64>| -> Result<()> { Ok(()) })?;

	let rc_close = Rc::new(close)?;
	let rc_accept = Rc::new(accept)?;
	let rc_recv = Rc::new(recv)?;

	let server = Connection::acceptor(s, rc_recv, rc_accept, rc_close, 0u64)?;
	evh.register(server.clone())?;

	evh.start()?;

	park();

	Ok(())
}

fn exec_client(messages: u64) -> Result<()> {
	let port = 9090;
	let start = unsafe { getmicros() };
	let mut count = Rc::new(0)?;
	let lock = lock_box!()?;
	let count_clone = count.clone();
	let lock_clone = lock.clone();

	let recv_client: OnRecv<u64, u64> = Box::new(
		move |ctx: &mut u64, _conn: &mut Connection<u64, u64>, bytes: &[u8]| -> Result<()> {
			*ctx += bytes.len() as u64;
			if *ctx >= (messages * 4) {
				let ms = (unsafe { getmicros() } - start) as f64 / 1_000 as f64;
				let qps = messages as f64 / (ms / 1000 as f64);
				println!("compelte in {}ms. Bytes recv={},qps={}", ms, *ctx, qps);
			}
			*count += bytes.len();

			Ok(())
		},
	)?;
	let close_client: OnClose<u64, u64> =
		Box::new(move |_ctx: &mut u64, _conn: &mut Connection<u64, u64>| -> Result<()> { Ok(()) })?;
	let rc_recv_client = Rc::new(recv_client)?;
	let rc_close_client = Rc::new(close_client)?;
	let client = Socket::connect([127, 0, 0, 1], port)?;
	let connector = Connection::outbound(client, rc_recv_client, rc_close_client, 0u64)?;
	let mut evh = Evh::new()?;
	evh.register(connector.clone())?;
	evh.start()?;

	let mut i = 0;
	loop {
		match connector.write(b"test") {
			Ok(v) => {
				if v != 4 {
					exit!("len !+ 4");
				}
			}
			Err(e) => {
				if e == EAgain {
					continue;
				} else {
					return err!(e);
				}
			}
		}
		i += 1;
		if i == messages {
			break;
		}
	}

	loop {
		sleep(10);
		let _l = lock_clone.read();
		if *count_clone >= (messages * 4) as usize {
			break;
		}
	}
	sleep(1);

	Ok(())
}

extern "C" {
	fn atoi(s: *const u8) -> i32;
}

fn proc_args(argc: i32, argv: *const *const u8) -> Result<()> {
	let arg2 = unsafe { CString::from_ptr(*(argv.offset(1)), true) };
	let messages = if argc > 2 {
		(unsafe { atoi(*(argv.offset(2))) }) as u64
	} else {
		0
	};
	if arg2.as_str()? == String::new("server")? {
		println!("Starting server!");
		exec_server()?;
	} else if arg2.as_str()? == String::new("client")? {
		println!("Starting client!");
		exec_client(messages)?;
	}
	Ok(())
}

#[no_mangle]
pub extern "C" fn real_main(argc: i32, argv: *const *const u8) -> i32 {
	if argc >= 2 {
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
