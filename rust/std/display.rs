use prelude::*;

pub trait Display {
	fn format(&self, f: &mut Formatter) -> Result<()>;
}
