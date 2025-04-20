use prelude::*;

pub struct Node<K, V> {
	pub(crate) k: K,
	pub(crate) v: V,
	pub(crate) next: Ptr<Node<K, V>>,
}

impl<K: PartialEq, V> PartialEq for Node<K, V> {
	fn eq(&self, other: &Node<K, V>) -> bool {
		self.k == other.k
	}
}

impl<K, V> Node<K, V> {
	pub fn new(k: K, v: V) -> Self {
		Self {
			k,
			v,
			next: Ptr::null(),
		}
	}
}
