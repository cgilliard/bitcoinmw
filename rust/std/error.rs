use prelude::*;

#[derive(Clone)]
pub struct Error {
	code: u64,
	display: fn() -> &'static str,
	bt: Backtrace,
}

impl PartialEq for Error {
	fn eq(&self, other: &Error) -> bool {
		self.code == other.code
	}
}

impl Debug for Error {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> FmtResult {
		#[cfg(test)]
		{
			let kind_str = (self.display)();
			let bt_text = self.bt.as_str();
			if bt_text.len() == 0 {
				write!(_f, "ErrorKind={}\n{}", kind_str,
                                "Backtrace disabled. To view backtrace set env variable; export RUST_BACKTRACE=1.")?;
			} else {
				write!(_f, "ErrorKind={}\n{}", kind_str, bt_text)?;
			}
		}
		Ok(())
	}
}
