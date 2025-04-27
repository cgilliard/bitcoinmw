use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;
use crypto::Sha3_256;
use prelude::*;

const BIBLE_VERSE_COUNT: usize = 31107;
const BIBLE_SHA3_256_HASH: [u8; 32] = [
	74, 250, 234, 251, 53, 214, 95, 98, 53, 200, 128, 99, 85, 98, 96, 39, 84, 185, 194, 248, 203,
	235, 56, 26, 253, 230, 106, 249, 73, 223, 22, 4,
];

pub struct Bible {
	ptr: *const u8,
	indices: [u32; BIBLE_VERSE_COUNT + 1],
}

extern "C" {
	fn get_bible_data() -> *const u8;
	fn get_bible_len() -> usize;
}

impl Bible {
	pub fn new() -> Self {
		let ptr = unsafe { get_bible_data() };
		let len = unsafe { get_bible_len() };
		Self::validate_hash(ptr, len);
		let indices = Self::build_indices(ptr, len);
		Self { ptr, indices }
	}

	pub fn find_mod(&self, v: usize) -> &str {
		let index = v % BIBLE_VERSE_COUNT;
		let offset = self.indices[index];
		unsafe {
			let len = (self.indices[index + 1] - self.indices[index]) - 2;
			let slice = from_raw_parts(self.ptr.add(offset as usize), len as usize);
			from_utf8_unchecked(slice)
		}
	}

	fn validate_hash(ptr: *const u8, len: usize) {
		unsafe {
			let sha3 = Sha3_256::new();
			let slice = from_raw_parts(ptr, len);
			sha3.update(slice);
			let hash = sha3.finalize();
			if hash != BIBLE_SHA3_256_HASH {
				exit!(
					"Bible is corrupted! Expected hash {}. Found hash {}. Halting!",
					BIBLE_SHA3_256_HASH,
					hash
				);
			}
		}
	}

	fn build_indices(ptr: *const u8, len: usize) -> [u32; BIBLE_VERSE_COUNT + 1] {
		let mut ret = [0u32; BIBLE_VERSE_COUNT + 1];

		let itt = ptr;
		let mut offt = 0;
		let mut count = 0;
		loop {
			if offt >= len {
				break;
			}

			let start = offt;
			let mut pos_ptr = unsafe { itt.offset(offt as isize) };
			while offt < len && unsafe { *pos_ptr != b'\n' } {
				pos_ptr = unsafe { itt.offset(offt as isize) };
				offt += 1;
			}
			if count < ret.len() {
				ret[count] = start as u32;
				count += 1;
			} else {
				break;
			}
		}

		// set last item to len+2 to include last period.
		ret[BIBLE_VERSE_COUNT] = (2 + len) as u32;

		ret
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_bible1() -> Result<()> {
		let bible = Bible::new();

		// first verse
		assert_eq!(
			bible.find_mod(0),
			"Genesis||1||1||In the beginning God created the heaven and the earth."
		);

		// random verse
		assert_eq!(
                    bible.find_mod(199),
                    "Genesis||8||16||Go forth of the ark, you, and your wife, and your sons, and your sons'wives with you."
                );

		// final verse
		assert_eq!(
			bible.find_mod(BIBLE_VERSE_COUNT - 1),
			"Revelation||22||21||The grace of our Lord Jesus Christ be with you all. Amen."
		);

		// assert that we wrap arround correctly
		assert_eq!(bible.find_mod(BIBLE_VERSE_COUNT), bible.find_mod(0));

		Ok(())
	}
}
