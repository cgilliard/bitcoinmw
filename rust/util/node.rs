use core::ptr::null_mut;
use prelude::*;

pub struct HashtableNode<K, V> {
	pub(crate) key: K,
	pub(crate) value: V,
	pub(crate) next: Ptr<HashtableNode<K, V>>,
}

impl<K: PartialEq, V> PartialEq for HashtableNode<K, V> {
	fn eq(&self, other: &HashtableNode<K, V>) -> bool {
		self.key == other.key
	}
}

impl<K, V> HashtableNode<K, V> {
	pub fn new(key: K, value: V) -> Self {
		Self {
			key,
			value,
			next: Ptr::null(),
		}
	}
}

pub(crate) enum Color {
	Black,
	Red,
}

pub struct RbTreeNode<V: Ord> {
	pub parent: Ptr<RbTreeNode<V>>,
	pub right: Ptr<RbTreeNode<V>>,
	pub left: Ptr<RbTreeNode<V>>,
	pub value: V,
}

impl<V: Ord> Display for RbTreeNode<V> {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		writef!(
			f,
			"Node: parent={},left={},right={},color={},bitcolor={}",
			self.parent,
			self.left,
			self.right,
			if self.is_red() { "red" } else { "black" },
			if self.parent.get_bit() {
				"red"
			} else {
				"black"
			}
		)
	}
}

impl<V: Ord> RbTreeNode<V> {
	pub fn new(value: V) -> Self {
		Self {
			parent: Ptr::new_bit_set(null_mut()),
			right: Ptr::null(),
			left: Ptr::null(),
			value,
		}
	}

	pub fn alloc(value: V) -> Result<Ptr<Self>> {
		Ptr::alloc(RbTreeNode::new(value))
	}

	pub(crate) fn set_color(&mut self, color: Color) {
		match color {
			Color::Black => {
				self.parent.set_bit(false);
			}
			Color::Red => {
				self.parent.set_bit(true);
			}
		}
	}

	pub(crate) fn is_root(&self) -> bool {
		self.parent.is_null()
	}

	pub(crate) fn is_red(&self) -> bool {
		self.parent.get_bit()
	}

	pub(crate) fn is_black(&self) -> bool {
		!self.is_red()
	}

	pub(crate) fn set_parent(&mut self, parent: Ptr<Self>) {
		match self.is_black() {
			true => {
				self.parent = parent;
				self.parent.set_bit(false);
			}
			false => {
				self.parent = parent;
				self.parent.set_bit(true);
			}
		}
	}
}
