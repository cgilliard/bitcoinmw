use prelude::*;

pub struct Node<K, V> {
	pub(crate) key: K,
	pub(crate) value: V,
	pub(crate) next: Ptr<Node<K, V>>,
}

impl<K: PartialEq, V> PartialEq for Node<K, V> {
	fn eq(&self, other: &Node<K, V>) -> bool {
		self.key == other.key
	}
}

impl<K, V> Node<K, V> {
	pub fn new(key: K, value: V) -> Self {
		Self {
			key,
			value,
			next: Ptr::null(),
		}
	}
}
