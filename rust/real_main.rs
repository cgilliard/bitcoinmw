use prelude::*;
use util::{Hashtable, Node};

struct KVPair {
	k: u32,
	v: u64,
}

impl Hash for KVPair {
	fn hash<H>(&self, state: &mut H)
	where
		H: Hasher,
	{
		self.k.hash(state);
	}
}

impl PartialEq for KVPair {
	fn eq(&self, other: &KVPair) -> bool {
		self.k == other.k
	}
}

fn res() -> Result<(), Error> {
	let mut hash = Hashtable::new(1024)?;
	let v = Ptr::alloc(Node::new(KVPair { k: 1, v: 2 }))?;
	hash.insert(v);
	match hash.find(&KVPair { k: 1, v: 10 }) {
		Some(v2) => {
			println!("v2.value={}", v2.value.v);
		}
		None => println!("not found"),
	}
	match hash.find(&KVPair { k: 2, v: 10 }) {
		Some(v2) => {
			println!("v2.value={}", v2.value.v);
		}
		None => println!("not found"),
	}

	match hash.remove(&KVPair { k: 1, v: 10 }) {
		Some(v) => {
			println!("removing v.value={}", v.value.v);
			v.release();
		}
		None => {}
	}
	Ok(())
}

#[no_mangle]
pub extern "C" fn real_main(_argc: i32, _argv: *const *const u8) -> i32 {
	let _ = res();
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
