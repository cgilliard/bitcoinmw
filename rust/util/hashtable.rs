use core::iter::IntoIterator;
use core::iter::Iterator;
use core::ops::{Deref, DerefMut};
use core::option::Option as CoreOption;
use core::ptr::null_mut;
use prelude::*;
use std::ffi::rand_bytes;
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

pub struct Node<V: PartialEq> {
	next: Ptr<Node<V>>,
	pub value: V,
}

impl<V: PartialEq> PartialEq for Node<V> {
	fn eq(&self, other: &Node<V>) -> bool {
		self.value == other.value
	}
}

impl<V: PartialEq> Deref for Node<V> {
	type Target = V;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<V: PartialEq> DerefMut for Node<V> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

impl<V: PartialEq> Node<V> {
	pub fn new(value: V) -> Self {
		Self {
			next: Ptr::new(null_mut()),
			value,
		}
	}
}

pub struct Hashtable<V: PartialEq + Hash, S: BuildHasher + Default> {
	arr: Vec<Ptr<Node<V>>>,
	hasher: S,
}

pub struct HashtableIterator<V: PartialEq + Hash, S: BuildHasher + Default> {
	hashtable: Hashtable<V, S>,
	cur: Ptr<Node<V>>,
	index: usize,
}

pub struct HashtableRefIterator<'a, V: PartialEq + Hash, S: BuildHasher + Default> {
	hashtable: &'a Hashtable<V, S>,
	cur: Ptr<Node<V>>,
	index: usize,
}

impl<'a, V: PartialEq + Hash, S: BuildHasher + Default> Iterator
	for HashtableRefIterator<'a, V, S>
{
	type Item = Ptr<Node<V>>;

	fn next(&mut self) -> CoreOption<Self::Item> {
		while self.cur.is_null() && self.index < self.hashtable.arr.len() {
			self.cur = self.hashtable.arr[self.index];
			if !self.cur.is_null() {
				break;
			}
			self.index += 1;
		}

		match self.cur.is_null() {
			true => CoreOption::None,
			false => match self.cur.next.is_null() {
				true => {
					self.index += 1;
					let ret = self.cur;
					self.cur = Ptr::null();
					CoreOption::Some(ret)
				}
				false => {
					let ret = self.cur;
					self.cur = self.cur.next;
					CoreOption::Some(ret)
				}
			},
		}
	}
}

impl<V: PartialEq + Hash, S: BuildHasher + Default> Iterator for HashtableIterator<V, S> {
	type Item = Ptr<Node<V>>;
	fn next(&mut self) -> CoreOption<Self::Item> {
		while self.cur.is_null() && self.index < self.hashtable.arr.len() {
			self.cur = self.hashtable.arr[self.index];
			if !self.cur.is_null() {
				break;
			}
			self.index += 1;
		}

		match self.cur.is_null() {
			true => CoreOption::None,
			false => match self.cur.next.is_null() {
				true => {
					self.index += 1;
					let ret = self.cur;
					self.cur = Ptr::null();
					CoreOption::Some(ret)
				}
				false => {
					let ret = self.cur;
					self.cur = self.cur.next;
					CoreOption::Some(ret)
				}
			},
		}
	}
}

impl<V: PartialEq + Hash, S: BuildHasher + Default> IntoIterator for Hashtable<V, S> {
	type Item = Ptr<Node<V>>;
	type IntoIter = HashtableIterator<V, S>;

	fn into_iter(self) -> Self::IntoIter {
		let cur = self.arr[0];
		HashtableIterator {
			hashtable: self,
			cur,
			index: 0,
		}
	}
}

impl<'a, V: PartialEq + Hash, S: BuildHasher + Default> IntoIterator for &'a Hashtable<V, S> {
	type Item = Ptr<Node<V>>;
	type IntoIter = HashtableRefIterator<'a, V, S>;

	fn into_iter(self) -> Self::IntoIter {
		HashtableRefIterator {
			hashtable: self,
			cur: self.arr[0],
			index: 0,
		}
	}
}

impl<V: PartialEq + Hash, S: BuildHasher + Default> Hashtable<V, S> {
	pub fn new(size: usize) -> Result<Self, Error> {
		let mut arr = Vec::new();
		let hasher = S::default();
		match arr.resize(size) {
			Ok(_) => Ok(Self { arr, hasher }),
			Err(e) => Err(e),
		}
	}
	pub fn with_hasher(size: usize, hasher: S) -> Result<Self, Error> {
		let mut arr = Vec::new();
		match arr.resize(size) {
			Ok(_) => Ok(Self { arr, hasher }),
			Err(e) => Err(e),
		}
	}

	pub fn insert(&mut self, mut node: Ptr<Node<V>>) -> bool {
		(*node).next = Ptr::null();
		let value = &*node;
		if self.arr.len() == 0 {
			return false;
		}
		let mut hasher = self.hasher.build_hasher();
		value.hash(&mut hasher);
		let index = hasher.finish() as usize % self.arr.len();
		let mut ptr = self.arr[index];
		if ptr.is_null() {
			self.arr[index] = node;
		} else {
			let mut prev = Ptr::new(null_mut());
			while !ptr.is_null() {
				if *ptr == *value {
					return false;
				}
				prev = ptr;
				ptr = (*ptr).next;
			}

			(*prev).next = node;
		}
		true
	}

	pub fn find(&self, value: &V) -> Option<Ptr<Node<V>>> {
		if self.arr.len() == 0 {
			return None;
		}
		let mut hasher = self.hasher.build_hasher();
		value.hash(&mut hasher);
		let mut ptr = self.arr[hasher.finish() as usize % self.arr.len()];
		while !ptr.is_null() {
			if &ptr.value == value {
				return Some(Ptr::new(ptr.raw()));
			}
			ptr = (ptr.as_ref()).next;
		}
		None
	}

	pub fn remove(&mut self, value: &V) -> Option<Ptr<Node<V>>> {
		if self.arr.len() > 0 {
			let mut hasher = self.hasher.build_hasher();
			value.hash(&mut hasher);
			let index = hasher.finish() as usize % self.arr.len();
			let mut ptr = self.arr[index];

			if !ptr.is_null() && (*ptr).value == *value {
				self.arr[index] = (*ptr).next;
				return Some(Ptr::new(ptr.raw()));
			}
			let mut prev = self.arr[index];

			while !ptr.is_null() {
				if (*ptr).value == *value {
					(*prev).next = (*ptr).next;
					return Some(Ptr::new(ptr.raw()));
				}
				prev = ptr;
				ptr = (*ptr).next;
			}
		}
		None
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::mem::size_of;
	use std::ffi::{alloc, getalloccount};

	struct TestValue {
		k: i32,
		v: i32,
	}

	impl PartialEq for TestValue {
		fn eq(&self, other: &TestValue) -> bool {
			self.k == other.k
		}
	}

	impl Hash for TestValue {
		fn hash<H>(&self, state: &mut H)
		where
			H: Hasher,
		{
			self.k.hash(state);
		}
	}

	impl From<i32> for TestValue {
		fn from(k: i32) -> Self {
			Self { k, v: 0 }
		}
	}

	#[test]
	fn test_hashtable1() {
		let initial = unsafe { getalloccount() };
		let v;
		unsafe {
			v = alloc(size_of::<Node<TestValue>>()) as *mut Node<TestValue>;
			*v = Node::new(TestValue { k: 1i32, v: 2i32 });
		}
		{
			let mut hash: Hashtable<TestValue, Murmur3Hasher> = Hashtable::new(1024).unwrap();
			let node = Ptr::new(v);
			hash.insert(node);

			let mut n = hash.find(&1i32.into()).unwrap();
			assert_eq!((*n).v, 2);
			(*n).v = 3i32;
			assert!(hash.find(&4i32.into()).is_none());
			let n = hash.find(&1i32.into()).unwrap();
			assert_eq!((*n).v, 3);
			let n = hash.remove(&1i32.into()).unwrap();
			assert_eq!((*n).v, 3);
			n.release();
			assert!(hash.remove(&1i32.into()).is_none());
		}
		assert_eq!(unsafe { getalloccount() }, initial);
	}

	#[test]
	fn test_hashtable_collisions() {
		let initial = unsafe { getalloccount() };

		let v1 = Ptr::alloc(Node::new(TestValue { k: 1, v: 2 })).unwrap();
		let v2 = Ptr::alloc(Node::new(TestValue { k: 2, v: 3 })).unwrap();
		let v3 = Ptr::alloc(Node::new(TestValue { k: 3, v: 4 })).unwrap();

		let v4 = Ptr::alloc(Node::new(TestValue { k: 1, v: 2 })).unwrap();
		let v5 = Ptr::alloc(Node::new(TestValue { k: 2, v: 3 })).unwrap();
		let v6 = Ptr::alloc(Node::new(TestValue { k: 3, v: 4 })).unwrap();

		{
			let mut hash: Hashtable<TestValue, Murmur3Hasher> = Hashtable::new(1).unwrap();
			assert!(hash.insert(v1));
			assert!(hash.insert(v2));
			assert!(hash.insert(v3));
			assert!(!hash.insert(v4));
			assert!(!hash.insert(v5));
			assert!(!hash.insert(v6));

			assert_eq!(hash.find(&1i32.into()).unwrap().v, 2);
			assert!(hash.remove(&4i32.into()).is_none());

			v4.release();
			v5.release();
			v6.release();

			let mut n = hash.find(&1i32.into()).unwrap();
			assert_eq!((*n).v, 2);
			(*n).v = 3;
			let n = hash.find(&1i32.into()).unwrap();
			assert_eq!((*n).v, 3);

			let mut n = hash.find(&2i32.into()).unwrap();
			assert_eq!((*n).v, 3);
			(*n).v = 4;
			let n = hash.find(&2i32.into()).unwrap();
			assert_eq!((*n).v, 4);

			let mut n = hash.find(&3i32.into()).unwrap();
			assert_eq!((*n).v, 4);
			(*n).v = 5;
			let n = hash.find(&3i32.into()).unwrap();
			assert_eq!((*n).v, 5);

			let n = hash.remove(&1i32.into()).unwrap();
			assert_eq!((*n).v, 3);
			assert!(hash.remove(&1i32.into()).is_none());

			n.release();

			let n = hash.remove(&2i32.into()).unwrap();
			assert_eq!((*n).v, 4);
			assert!(hash.remove(&2i32.into()).is_none());

			n.release();

			let n = hash.remove(&3i32.into()).unwrap();
			assert_eq!((*n).v, 5);
			assert!(hash.remove(&3i32.into()).is_none());
			n.release();
		}
		assert_eq!(unsafe { getalloccount() }, initial);
	}

	#[test]
	fn test_hashtable_iter() -> Result<(), Error> {
		let hasher = Murmur3Hasher::new(105);
		let mut hash = Hashtable::with_hasher(3, hasher)?;
		for i in 0..10 {
			let v = Ptr::alloc(Node::new(TestValue { k: i, v: i })).unwrap();
			hash.insert(v);
		}

		let mut check: Vec<u32> = Vec::new();
		assert!(check.resize(10).is_ok());
		for x in hash {
			check[x.v as usize] += 1;
		}
		for i in 0..10 {
			assert_eq!(check[i], 1);
		}

		Ok(())
	}
}
