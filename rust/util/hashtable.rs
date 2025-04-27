use core::iter::IntoIterator;
use core::iter::Iterator;
use core::ptr::null_mut;
use prelude::*;
use std::ffi::rand_bytes;
use util::errors::Duplicate;
use util::node::HashtableNode;
use util::Hasher128;

pub struct Murmur3Hasher {
	seed: u32,
}
impl BuildHasher for Murmur3Hasher {
	type Hasher = Hasher128;
	fn build_hasher(&self) -> Hasher128 {
		Hasher128::with_seed(self.seed)
	}
}

impl Default for Murmur3Hasher {
	fn default() -> Self {
		let mut seed = 0;
		if unsafe { rand_bytes(&mut seed as *mut u32 as *mut u8, 4) } != 0 {
			// note: for murmur hash, we're not using this for cryptographic hashing.
			// We can continue with the default seed.
			println!("Could not obtain needed entropy to setup Murmur3Hash! Using default value.");
			seed = 0;
		}
		Self { seed }
	}
}

impl Murmur3Hasher {
	pub fn new(seed: u32) -> Self {
		Self { seed }
	}
}

pub struct Hashtable<K: PartialEq + Hash, V, S: BuildHasher + Default> {
	arr: Vec<Ptr<HashtableNode<K, V>>>,
	hasher: S,
	count: u64,
}

pub struct HashtableRefIterator<'a, K: PartialEq + Hash, V, S: BuildHasher + Default> {
	hashtable: &'a Hashtable<K, V, S>,
	cur: Ptr<HashtableNode<K, V>>,
	index: usize,
}

impl<'a, K: PartialEq + Hash, V, S: BuildHasher + Default> Iterator
	for HashtableRefIterator<'a, K, V, S>
{
	type Item = Ptr<HashtableNode<K, V>>;

	fn next(&mut self) -> Option<Self::Item> {
		while self.cur.is_null() && self.index < self.hashtable.arr.len() {
			self.cur = self.hashtable.arr[self.index];
			if !self.cur.is_null() {
				break;
			}
			self.index += 1;
		}

		match self.cur.is_null() {
			true => None,
			false => match self.cur.next.is_null() {
				true => {
					self.index += 1;
					let ret = self.cur;
					self.cur = Ptr::null();
					Some(ret)
				}
				false => {
					let ret = self.cur;
					self.cur = self.cur.next;
					Some(ret)
				}
			},
		}
	}
}

impl<'a, K: Hash + PartialEq, V, S: BuildHasher + Default> IntoIterator for &'a Hashtable<K, V, S> {
	type Item = Ptr<HashtableNode<K, V>>;
	type IntoIter = HashtableRefIterator<'a, K, V, S>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<K: PartialEq + Hash, V, S: BuildHasher + Default> Hashtable<K, V, S> {
	pub fn new(size: usize) -> Result<Self> {
		if size == 0 {
			return err!(IllegalArgument);
		}
		let mut arr = Vec::new();
		let hasher = S::default();
		match arr.resize(size) {
			Ok(_) => Ok(Self {
				arr,
				hasher,
				count: 0,
			}),
			Err(e) => Err(e),
		}
	}
	pub fn with_hasher(size: usize, hasher: S) -> Result<Self> {
		if size == 0 {
			return err!(IllegalArgument);
		}
		let mut arr = Vec::new();
		match arr.resize(size) {
			Ok(_) => Ok(Self {
				arr,
				hasher,
				count: 0,
			}),
			Err(e) => Err(e),
		}
	}

	pub fn insert(&mut self, mut node: Ptr<HashtableNode<K, V>>) -> Result<()> {
		(*node).next = Ptr::null();
		let key = &(&*node).key;
		let mut hasher = self.hasher.build_hasher();
		key.hash(&mut hasher);
		let index = hasher.finish() as usize % self.arr.len();
		let mut ptr = self.arr[index];
		if ptr.is_null() {
			self.arr[index] = node;
		} else {
			let mut prev = Ptr::new(null_mut());
			while !ptr.is_null() {
				if *ptr == *node {
					return err!(Duplicate);
				}
				prev = ptr;
				ptr = (*ptr).next;
			}

			(*prev).next = node;
		}
		self.count += 1;
		Ok(())
	}

	pub fn find(&self, key: &K) -> Option<&mut V> {
		let mut hasher = self.hasher.build_hasher();
		key.hash(&mut hasher);
		let mut ptr = self.arr[hasher.finish() as usize % self.arr.len()];
		while !ptr.is_null() {
			let node = ptr.as_ref();
			if &node.key == key {
				return Some(unsafe { &mut (*ptr.raw()).value });
			}
			ptr = node.next;
		}
		None
	}

	pub fn remove(&mut self, key: &K) -> Option<Ptr<HashtableNode<K, V>>> {
		if self.arr.len() > 0 {
			let mut hasher = self.hasher.build_hasher();
			key.hash(&mut hasher);
			let index = hasher.finish() as usize % self.arr.len();
			let mut ptr = self.arr[index];

			if !ptr.is_null() && (*ptr).key == *key {
				self.arr[index] = (*ptr).next;
				self.count -= 1;
				return Some(Ptr::new(ptr.raw()));
			}
			let mut prev = self.arr[index];

			while !ptr.is_null() {
				if (*ptr).key == *key {
					(*prev).next = (*ptr).next;
					self.count -= 1;
					return Some(Ptr::new(ptr.raw()));
				}
				prev = ptr;
				ptr = (*ptr).next;
			}
		}
		None
	}

	pub fn len(&self) -> u64 {
		self.count
	}

	pub fn iter<'a>(&'a self) -> HashtableRefIterator<'a, K, V, S> {
		HashtableRefIterator {
			hashtable: self,
			cur: self.arr[0],
			index: 0,
		}
	}
}

#[cfg(test)]
mod test {
	use prelude::*;
	use util::{Hashtable, HashtableNode, Murmur3Hasher};

	#[test]
	fn test_hashtable1() -> Result<()> {
		let mut hashtable: Hashtable<u64, i32> = Hashtable::new(1024)?;
		let node = Ptr::alloc(HashtableNode::new(1, 2))?;
		assert!(hashtable.insert(node).is_ok());
		let v = hashtable.find(&1).unwrap();
		assert_eq!(*v, 2);
		let n = hashtable.remove(&1).unwrap();
		assert_eq!(n.key, 1);
		assert_eq!(n.value, 2);
		n.release();

		let node = Ptr::alloc(HashtableNode::new(1, 2))?;
		assert!(hashtable.insert(node).is_ok());
		let node = Ptr::alloc(HashtableNode::new(9, 9))?;
		assert!(hashtable.insert(node).is_ok());
		let mut count = 0;
		for n in &hashtable {
			if n.key == 9 {
				assert_eq!(n.value, 9);
				count += 1;
			} else if n.key == 1 {
				assert_eq!(n.value, 2);
				count += 1;
			} else {
				return err!(IllegalState);
			}
		}

		assert_eq!(hashtable.len(), 2);
		assert_eq!(count, 2);
		hashtable.remove(&1).unwrap().release();
		hashtable.remove(&9).unwrap().release();
		assert_eq!(hashtable.len(), 0);

		Ok(())
	}

	struct TestValue {
		x: i32,
		y: i32,
	}

	#[test]
	fn test_hashtable2() -> Result<()> {
		let hasher = Murmur3Hasher { seed: 1234 };
		let mut hash = Hashtable::with_hasher(1024, hasher).unwrap();
		let node = Ptr::alloc(HashtableNode::new(101, TestValue { x: 1, y: 2 }))?;
		hash.insert(node)?;
		let x = hash.find(&101).unwrap();
		assert_eq!(x.x, 1);
		assert_eq!(x.y, 2);
		let mut count = 0;
		for value in &hash {
			assert_eq!(value.key, 101);
			assert_eq!(value.value.x, 1);
			assert_eq!(value.value.y, 2);
			count += 1;
		}
		assert_eq!(count, 1);
		hash.remove(&101).unwrap().release();

		Ok(())
	}

	#[test]
	fn test_hashtable_collisions() {
		let v1 = Ptr::alloc(HashtableNode::new(1, TestValue { x: 1, y: 2 })).unwrap();
		let v2 = Ptr::alloc(HashtableNode::new(2, TestValue { x: 2, y: 3 })).unwrap();
		let v3 = Ptr::alloc(HashtableNode::new(3, TestValue { x: 3, y: 4 })).unwrap();

		let v4 = Ptr::alloc(HashtableNode::new(1, TestValue { x: 1, y: 2 })).unwrap();
		let v5 = Ptr::alloc(HashtableNode::new(2, TestValue { x: 2, y: 3 })).unwrap();
		let v6 = Ptr::alloc(HashtableNode::new(3, TestValue { x: 3, y: 4 })).unwrap();

		{
			let mut hash: Hashtable<i32, TestValue> = Hashtable::new(1).unwrap();
			assert!(hash.insert(v1).is_ok());
			assert!(hash.insert(v2).is_ok());
			assert!(hash.insert(v3).is_ok());
			assert!(hash.insert(v4).is_err());
			assert!(hash.insert(v5).is_err());
			assert!(hash.insert(v6).is_err());
			assert_eq!(hash.find(&1i32.into()).unwrap().y, 2);
			assert!(hash.remove(&4i32.into()).is_none());

			v4.release();
			v5.release();
			v6.release();

			let n = hash.find(&1i32.into()).unwrap();
			assert_eq!((*n).y, 2);
			(*n).y = 3;
			let n = hash.find(&1i32.into()).unwrap();
			assert_eq!((*n).y, 3);

			let n = hash.find(&2i32.into()).unwrap();
			assert_eq!((*n).y, 3);
			(*n).y = 4;
			let n = hash.find(&2i32.into()).unwrap();
			assert_eq!((*n).y, 4);

			let n = hash.find(&3i32.into()).unwrap();
			assert_eq!((*n).y, 4);
			(*n).y = 5;
			let n = hash.find(&3i32.into()).unwrap();
			assert_eq!((*n).y, 5);

			let n = hash.remove(&1i32.into()).unwrap();
			assert_eq!((*n).value.y, 3);
			assert!(hash.remove(&1i32.into()).is_none());

			n.release();

			let n = hash.remove(&2i32.into()).unwrap();
			assert_eq!((*n).value.y, 4);
			assert!(hash.remove(&2i32.into()).is_none());

			n.release();

			let n = hash.remove(&3i32.into()).unwrap();
			assert_eq!((*n).value.y, 5);
			assert!(hash.remove(&3i32.into()).is_none());
			n.release();
		}
	}

	#[test]
	fn test_hashtable_iter() -> Result<()> {
		let hasher = Murmur3Hasher::new(105);
		let mut hash = Hashtable::with_hasher(3, hasher)?;
		for i in 0..10 {
			let v = Ptr::alloc(HashtableNode::new(i, TestValue { x: i, y: i + 1 })).unwrap();
			hash.insert(v)?;
		}

		let mut check: Vec<u32> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0]?;
		for mut x in &hash {
			check[x.value.x as usize] += 1;
			x.value.y += 1;
		}

		for x in &hash {
			check[x.value.x as usize] += 1;
			assert_eq!(x.value.y, x.value.x + 2);
		}
		for i in 0..10 {
			assert_eq!(check[i], 2);
		}

		for i in 0..10 {
			hash.remove(&i).unwrap().release();
		}

		Ok(())
	}
}
