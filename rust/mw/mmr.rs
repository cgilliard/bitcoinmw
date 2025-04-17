use prelude::*;

pub struct MMR {}

impl MMR {
	pub fn new() -> Result<Self, Error> {
		Ok(Self {})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_mmr1() -> Result<(), Error> {
		let mmr = MMR::new()?;
		Ok(())
	}
}
