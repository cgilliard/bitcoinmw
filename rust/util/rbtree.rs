use core::marker::PhantomData;
use core::ptr::null_mut;
use prelude::*;
use std::ffi::alloc;
use util::errors::Duplicate;

pub struct RbTreeNode<V: Ord> {
	pub parent: Ptr<RbTreeNode<V>>,
	pub right: Ptr<RbTreeNode<V>>,
	pub left: Ptr<RbTreeNode<V>>,
	pub value: Ptr<V>,
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

impl<V: Ord> Drop for RbTreeNode<V> {
	fn drop(&mut self) {
		let _b = self.deallocate();
	}
}

impl<V: Ord> RbTreeNode<V> {
	pub fn stack(value: V) -> Result<Self> {
		let value = Box::new(value)?;
		let value = unsafe { value.into_raw() };
		Ok(Self {
			parent: Ptr::new_bit_set(null_mut()),
			right: Ptr::null(),
			left: Ptr::null(),
			value,
		})
	}

	pub fn alloc(value: V) -> Result<Ptr<Self>> {
		let v = Self::stack(value)?;
		let node: *mut RbTreeNode<V> = unsafe { alloc(size_of::<RbTreeNode<V>>()) as *mut _ };
		unsafe {
			crate::core::ptr::write(node as *mut _, v);
		}

		Ok(Ptr::new(node))
	}

	pub fn release(mut ptr: Ptr<Self>) {
		(*ptr).deallocate();
		ptr.release();
	}

	pub fn deallocate(&mut self) -> Box<V> {
		unsafe { Box::from_raw(self.value) }
	}

	fn set_color(&mut self, color: Color) {
		match color {
			Color::Black => {
				self.parent.set_bit(false);
			}
			Color::Red => {
				self.parent.set_bit(true);
			}
		}
	}

	fn is_root(&self) -> bool {
		self.parent.is_null()
	}

	fn is_red(&self) -> bool {
		self.parent.get_bit()
	}

	fn is_black(&self) -> bool {
		!self.is_red()
	}

	fn set_parent(&mut self, parent: Ptr<Self>) {
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

enum Color {
	Black,
	Red,
}

pub struct RbTree<V: Ord> {
	root: Ptr<RbTreeNode<V>>,
	count: usize,
}

pub struct RbTreeIterator<'a, V: Ord> {
	stack: [Option<Ptr<RbTreeNode<V>>>; 80], // Fixed-size stack
	stack_top: usize,
	_phantom: PhantomData<&'a V>, // For lifetime tracking
}

pub struct RbNodePair<V: Ord> {
	pub cur: Ptr<RbTreeNode<V>>,
	pub parent: Ptr<RbTreeNode<V>>,
	pub is_right: bool,
}

impl<'a, V: Ord> RbTreeIterator<'a, V> {
	// Push all left children starting from a node
	fn push_leftmost(&mut self, mut node: Ptr<RbTreeNode<V>>) {
		while !node.is_null() {
			if self.stack_top >= self.stack.len() {
				// Stack overflow; tree is too deep
				// 80 depth for RBTree 2 log(n) worst case
				// depth means at least 2^40 nodes
				// or over 1 trillion nodes before this occurs.
				return;
			}
			self.stack[self.stack_top] = Some(node);
			self.stack_top += 1;
			node = (*node).left;
		}
	}
}
impl<'a, V: Ord> Iterator for RbTreeIterator<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		if self.stack_top > 0 && self.stack_top < self.stack.len() {
			self.stack_top -= 1;
			let node = self.stack[self.stack_top].take()?;
			// Get the raw pointer
			let raw = node.raw();
			if raw.is_null() {
				return None;
			}
			// Convert raw pointer to reference with lifetime 'a
			let node_ref = unsafe { &*raw };
			self.push_leftmost(node_ref.right);
			Some(&node_ref.value)
		} else {
			None
		}
	}
}

impl<'a, V: Ord> IntoIterator for &'a RbTree<V> {
	type Item = &'a V;
	type IntoIter = RbTreeIterator<'a, V>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<T: TryClone + Ord> TryClone for RbTree<T> {
	fn try_clone(&self) -> Result<Self>
	where
		Self: Sized,
	{
		let mut ret: RbTree<T> = RbTree::new();
		let root = self.root();
		if !root.is_null() {
			self.insert_children(root, &mut ret)?;
		}

		Ok(ret)
	}
}

impl<V: Ord> RbTree<V> {
	pub fn new() -> Self {
		Self {
			root: Ptr::null(),
			count: 0,
		}
	}

	pub fn root(&self) -> Ptr<RbTreeNode<V>> {
		self.root
	}

	pub fn insert(&mut self, n: Ptr<RbTreeNode<V>>) -> Option<Ptr<RbTreeNode<V>>> {
		let pair = self.search(n);
		let ret = self.insert_impl(n, pair);
		if ret.is_none() {
			self.count += 1;
			self.insert_fixup(n);
		}
		ret
	}

	pub fn try_insert(&mut self, n: Ptr<RbTreeNode<V>>) -> Result<()> {
		let pair = self.search(n);
		if !pair.cur.is_null() {
			return err!(Duplicate);
		}
		self.insert_impl(n, pair);
		self.count += 1;
		self.insert_fixup(n);

		Ok(())
	}

	pub fn remove_ptr(&mut self, n: Ptr<RbTreeNode<V>>) -> Option<Ptr<RbTreeNode<V>>> {
		let pair = self.search(n);
		if pair.cur.is_null() {
			return None;
		}
		let ret = pair.cur.clone();
		self.remove_impl(pair);
		self.count -= 1;
		Some(ret)
	}

	pub fn len(&self) -> usize {
		self.count
	}

	pub fn iter(&self) -> RbTreeIterator<'_, V> {
		let mut iter = RbTreeIterator {
			stack: [None; 80],
			stack_top: 0,
			_phantom: PhantomData,
		};
		iter.push_leftmost(self.root);
		iter
	}

	pub fn remove(&mut self, value: V) -> Option<Ptr<RbTreeNode<V>>> {
		let node = RbTreeNode::stack(value).unwrap();
		let ptr = Ptr::new(&node as *const RbTreeNode<V>);
		self.remove_ptr(ptr)
	}

	pub fn search(&self, value: Ptr<RbTreeNode<V>>) -> RbNodePair<V> {
		let mut is_right = false;
		let mut cur = self.root();
		let mut parent = Ptr::null();

		while !cur.is_null() {
			let cmp = (*value).value.cmp(&(*cur).value);
			match cmp {
				Ordering::Equal => break,
				Ordering::Less => {
					parent = cur;
					is_right = false;
					cur = cur.left;
				}
				Ordering::Greater => {
					parent = cur;
					is_right = true;
					cur = cur.right;
				}
			}
		}

		RbNodePair {
			cur,
			parent,
			is_right,
		}
	}

	fn insert_children<T: TryClone + Ord>(
		&self,
		ptr: Ptr<RbTreeNode<T>>,
		n: &mut RbTree<T>,
	) -> Result<()> {
		let nval = RbTreeNode::alloc((*ptr.value).try_clone()?)?;
		n.insert(nval);

		if !ptr.right.is_null() {
			self.insert_children(ptr.right, n)?;
		}
		if !ptr.left.is_null() {
			self.insert_children(ptr.left, n)?;
		}
		Ok(())
	}

	fn remove_impl(&mut self, pair: RbNodePair<V>) {
		let node_to_delete = pair.cur;
		let mut do_fixup = node_to_delete.is_black();
		let (x, p, w);
		if node_to_delete.left.is_null() {
			x = node_to_delete.right;
			self.remove_transplant(node_to_delete, x);
			p = node_to_delete.parent;
			if !p.is_null() {
				if p.left.is_null() {
					w = p.right;
				} else {
					w = p.left;
				}
			} else {
				w = Ptr::null();
				do_fixup = false;
				if !self.root.is_null() {
					self.root.set_color(Color::Black);
				}
			}
		} else if node_to_delete.right.is_null() {
			x = node_to_delete.left;
			self.remove_transplant(node_to_delete, node_to_delete.left);
			p = node_to_delete.parent;
			if !p.is_null() {
				w = p.left;
			} else {
				w = Ptr::null();
			}
		} else {
			let mut successor = self.find_successor(node_to_delete);
			do_fixup = successor.is_black();
			x = successor.right;
			if !successor.parent.right.is_null() {
				if successor.parent.right.parent == node_to_delete {
					w = node_to_delete.left;
					p = successor;
				} else {
					w = successor.parent.right;
					p = w.parent;
				}
			} else {
				w = Ptr::null();
				p = Ptr::null();
			}

			if successor.parent != node_to_delete {
				self.remove_transplant(successor, successor.right);
				successor.right = node_to_delete.right;
				if !successor.right.is_null() {
					let successor_clone = successor.clone();
					successor.right.set_parent(successor_clone);
				}
			}

			self.remove_transplant(node_to_delete, successor);
			successor.left = node_to_delete.left;
			let successor_clone = successor.clone();
			successor.left.set_parent(successor_clone);
			if node_to_delete.is_black() {
				successor.set_color(Color::Black);
			} else {
				successor.set_color(Color::Red);
			}
		}
		if do_fixup {
			self.remove_fixup(p, w, x);
		}
	}

	fn find_successor(&mut self, mut x: Ptr<RbTreeNode<V>>) -> Ptr<RbTreeNode<V>> {
		x = x.right;
		loop {
			if x.left.is_null() {
				return x;
			}
			x = x.left;
		}
	}

	fn remove_transplant(&mut self, mut dst: Ptr<RbTreeNode<V>>, mut src: Ptr<RbTreeNode<V>>) {
		if dst.parent.is_null() {
			self.root = src;
		} else if dst == dst.parent.left {
			dst.parent.left = src;
		} else {
			dst.parent.right = src;
		}
		if !src.is_null() {
			src.set_parent(dst.parent);
		}
	}

	fn set_color_of_parent(&mut self, mut child: Ptr<RbTreeNode<V>>, parent: Ptr<RbTreeNode<V>>) {
		match parent.is_red() {
			true => child.set_color(Color::Red),
			false => child.set_color(Color::Black),
		}
	}

	fn is_root(&self, x: Ptr<RbTreeNode<V>>) -> bool {
		match x.is_null() {
			true => false,
			false => x.is_root(),
		}
	}

	fn is_black(&self, x: Ptr<RbTreeNode<V>>) -> bool {
		match x.is_null() {
			true => true,
			false => x.is_black(),
		}
	}

	fn is_red(&self, x: Ptr<RbTreeNode<V>>) -> bool {
		!self.is_black(x)
	}

	fn remove_fixup(
		&mut self,
		mut p: Ptr<RbTreeNode<V>>,
		mut w: Ptr<RbTreeNode<V>>,
		mut x: Ptr<RbTreeNode<V>>,
	) {
		while !self.is_root(x) && self.is_black(x) {
			if w == p.right {
				if self.is_red(w) {
					w.set_color(Color::Black);
					p.set_color(Color::Red);
					self.rotate_left(p);
					w = p.right;
				}
				if (w.left.is_null() || w.left.is_black())
					&& (w.right.is_null() || w.right.is_black())
				{
					w.set_color(Color::Red);
					x = p;
					p = p.parent;
					if !p.is_null() {
						let pl = p.left;
						if x == pl {
							w = p.right;
						} else {
							w = pl;
						}
					} else {
						w = Ptr::null();
					}
				} else {
					if w.right.is_null() || w.right.is_black() {
						w.left.set_color(Color::Black);
						w.set_color(Color::Red);
						self.rotate_right(w);
						w = p.right;
					}
					self.set_color_of_parent(w, p);
					p.set_color(Color::Black);
					w.right.set_color(Color::Black);
					self.rotate_left(p);
					x = self.root;
				}
			} else {
				if !w.is_null() && w.is_red() {
					w.set_color(Color::Black);
					p.set_color(Color::Red);
					self.rotate_right(p);
					w = p.left;
				}
				if (w.left.is_null() || w.left.is_black())
					&& (w.right.is_null() || w.right.is_black())
				{
					w.set_color(Color::Red);
					x = p;
					p = p.parent;
					if !p.is_null() {
						let pl = p.left;
						if x == pl {
							w = p.right;
						} else {
							w = pl;
						}
					} else {
						w = Ptr::null();
					}
				} else {
					if w.left.is_null() || w.left.is_black() {
						w.right.set_color(Color::Black);
						w.set_color(Color::Red);
						self.rotate_left(w);
						w = p.left;
					}
					self.set_color_of_parent(w, p);
					p.set_color(Color::Black);
					w.left.set_color(Color::Black);
					self.rotate_right(p);
					x = self.root;
				}
			}
		}
		if !x.is_null() {
			x.set_color(Color::Black);
		}
	}

	fn insert_impl(
		&mut self,
		mut n: Ptr<RbTreeNode<V>>,
		mut pair: RbNodePair<V>,
	) -> Option<Ptr<RbTreeNode<V>>> {
		let mut ret = None;
		if pair.cur.is_null() {
			n.set_parent(pair.parent);
			if pair.parent.is_null() {
				self.root = n;
			} else {
				match pair.is_right {
					true => pair.parent.right = n,
					false => pair.parent.left = n,
				}
			}
		} else {
			self.insert_transplant(pair.cur, n);
			if self.is_root(pair.cur) {
				self.root = n;
			}
			ret = Some(pair.cur);
		}
		ret
	}

	fn insert_transplant(&mut self, mut prev: Ptr<RbTreeNode<V>>, mut next: Ptr<RbTreeNode<V>>) {
		next.set_parent(prev.parent);
		next.right = prev.right;
		next.left = prev.left;
		if prev.is_red() {
			next.set_color(Color::Red);
		} else {
			next.set_color(Color::Black);
		}
		if !prev.parent.is_null() {
			if prev.parent.right == prev {
				prev.parent.right = next;
			} else {
				prev.parent.left = next;
			}
		}
		if !prev.right.is_null() {
			prev.right.parent = next;
		}

		if !prev.left.is_null() {
			prev.left.parent = next;
		}
	}

	fn rotate_left(&mut self, mut x: Ptr<RbTreeNode<V>>) {
		let mut y = x.right;
		x.right = y.left;
		if !y.left.is_null() {
			y.left.set_parent(x);
		}
		y.set_parent(x.parent);
		if x.parent.is_null() {
			self.root = y;
		} else if x == x.parent.left {
			x.parent.left = y;
		} else {
			x.parent.right = y;
		}
		y.left = x;
		x.set_parent(y);
	}

	fn rotate_right(&mut self, mut x: Ptr<RbTreeNode<V>>) {
		let mut y = x.left;
		x.left = y.right;
		if !y.right.is_null() {
			y.right.set_parent(x);
		}
		y.set_parent(x.parent);
		if x.parent.is_null() {
			self.root = y;
		} else if x == x.parent.right {
			x.parent.right = y;
		} else {
			x.parent.left = y;
		}
		y.right = x;
		x.set_parent(y);
	}

	fn insert_fixup(&mut self, mut k: Ptr<RbTreeNode<V>>) {
		let (mut parent, mut uncle, mut gparent);
		while !k.is_root() && k.parent.is_red() {
			parent = k.parent;
			gparent = parent.parent;
			if parent == gparent.left {
				uncle = gparent.right;
				if !uncle.is_null() && uncle.is_red() {
					parent.set_color(Color::Black);
					uncle.set_color(Color::Black);
					gparent.set_color(Color::Red);
					k = gparent
				} else {
					if k == parent.right {
						k = k.parent;
						self.rotate_left(k);
					}
					parent = k.parent;
					gparent = parent.parent;
					parent.set_color(Color::Black);
					gparent.set_color(Color::Red);
					self.rotate_right(gparent);
				}
			} else {
				uncle = gparent.left;
				if !uncle.is_null() && uncle.is_red() {
					parent.set_color(Color::Black);
					uncle.set_color(Color::Black);
					gparent.set_color(Color::Red);
					k = gparent;
				} else {
					if k == parent.left {
						k = k.parent;
						self.rotate_right(k);
					}
					parent = k.parent;
					gparent = parent.parent;
					parent.set_color(Color::Black);
					gparent.set_color(Color::Red);
					self.rotate_left(gparent);
				}
			}
		}
		self.root.set_color(Color::Black);
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::misc::{fnvhash, to_be_bytes_u64};

	fn fnvhash_32_of_u64(source: u64) -> u32 {
		let mut bytes = [0u8; 8];
		to_be_bytes_u64(source, &mut bytes);
		fnvhash(&bytes) as u32
	}

	fn validate_node(
		node: Ptr<RbTreeNode<u64>>,
		mut black_count: Ptr<i32>,
		mut current_black_count: i32,
	) {
		if node.is_null() {
			if *black_count == 0 {
				*black_count = current_black_count;
			} else {
				assert_eq!(current_black_count, *black_count);
			}
			return;
		}

		if node.is_black() {
			current_black_count += 1;
		} else {
			if !node.parent.is_black() {
				println!("red/black violation node={}", node);
			}
			assert!(node.parent.is_black());
		}
		validate_node(node.right, black_count, current_black_count);
		validate_node(node.left, black_count, current_black_count);
	}

	fn validate_tree(root: Ptr<RbTreeNode<u64>>) {
		let black_count = Ptr::alloc(0).unwrap();
		if !root.is_null() {
			assert!(root.is_black());
			validate_node(root, black_count, 0);
		}
		black_count.release();
	}

	#[allow(dead_code)]
	fn print_node(node: Ptr<RbTreeNode<u64>>, depth: usize) {
		if node.is_null() {
			for _ in 0..depth {
				print!("    ");
			}
			println!("0 (B)");
			return;
		}

		print_node((*node).right, depth + 1);
		for _ in 0..depth {
			print!("    ");
		}
		println!(
			"{} {} ({})",
			node,
			node.value,
			if node.is_red() { "R" } else { "B" }
		);
		print_node((*node).left, depth + 1);
	}

	#[allow(dead_code)]
	fn print_tree(root: Ptr<RbTreeNode<u64>>) {
		if root.is_null() {
			println!("Red-Black Tree (root = 0) Empty Tree!");
		} else {
			println!("Red-Black Tree (root = {})", root);
			println!("===================================");
			print_node(root, 0);
			println!("===================================");
		}
	}

	#[test]
	fn test_rbtree1() -> Result<()> {
		let mut tree = RbTree::new();

		let size = 100;
		for x in 0..5 {
			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let next = RbTreeNode::alloc(v as u64)?;
				assert!(tree.insert(next).is_none());
				validate_tree(tree.root());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let node = RbTreeNode::stack(v as u64)?;
				let ptr = Ptr::new(&node as *const _);
				let res = tree.search(ptr);
				assert!(!res.cur.is_null());
				assert_eq!(*(*(res.cur)).value, v as u64);
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let node_out = tree.remove(v as u64).unwrap();
				RbTreeNode::release(node_out);
				validate_tree(tree.root());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let node = RbTreeNode::stack(v as u64)?;
				let ptr = Ptr::new(&node as *const _);
				let res = tree.search(ptr);
				assert!(res.cur.is_null());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x + 1);
				let next = RbTreeNode::alloc(v as u64)?;
				assert!(tree.insert(next).is_none());
				validate_tree(tree.root());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x + 1);
				let node = RbTreeNode::stack(v as u64)?;
				let ptr = Ptr::new(&node as *const _);
				let res = tree.search(ptr);
				assert!(!res.cur.is_null());
				assert_eq!(*(*(res.cur)).value, v as u64);
			}

			let mut c = 0;

			for i in 0..size / 2 {
				c += 1;
				let v = fnvhash_32_of_u64(i + x + 1);
				let node_out = tree.remove(v as u64).unwrap();
				RbTreeNode::release(node_out);
				validate_tree(tree.root());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x + 30000);
				let next = RbTreeNode::alloc(v as u64)?;
				assert!(tree.insert(next).is_none());
				validate_tree(tree.root());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x + 30000);
				let node = RbTreeNode::stack(v as u64)?;
				let ptr = Ptr::new(&node as *const _);
				let res = tree.search(ptr);
				assert!(!res.cur.is_null());
				assert_eq!(*(*(res.cur)).value, v as u64);
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x + 30000);
				let node_out = tree.remove(v as u64).unwrap();
				RbTreeNode::release(node_out);
				validate_tree(tree.root());
			}

			for i in (size / 2)..size {
				c += 1;
				let v = fnvhash_32_of_u64(i + x + 1);
				let node_out = tree.remove(v as u64).unwrap();
				RbTreeNode::release(node_out);
				validate_tree(tree.root());
			}
			assert_eq!(c, size);
		}

		Ok(())
	}

	#[test]
	fn test_rbtree2() -> Result<()> {
		let mut tree = RbTree::new();

		let size = 100;
		for x in 0..5 {
			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let vs = format!("{}", v)?;
				let next = RbTreeNode::alloc(vs)?;
				assert!(tree.insert(next).is_none());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let vs = format!("{}", v)?;
				let node = RbTreeNode::stack(vs.clone())?;
				let ptr = Ptr::new(&node as *const _);
				let res = tree.search(ptr);
				assert!(!res.cur.is_null());
				assert_eq!(*(*(res.cur)).value, vs);
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let vs = format!("{}", v)?;
				let node = RbTreeNode::stack(vs)?;
				let ptr = Ptr::new(&node as *const _);
				let node_out = tree.remove_ptr(ptr).unwrap();
				RbTreeNode::release(node_out);
				let res = tree.search(ptr);
				assert!(res.cur.is_null());
			}
		}

		Ok(())
	}

	#[test]
	fn test_rbtree_try_insert() -> Result<()> {
		let mut tree = RbTree::new();

		let size = 3;
		for x in 0..2 {
			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let next = RbTreeNode::alloc(v as u64)?;
				assert!(tree.try_insert(next).is_ok());
				validate_tree(tree.root());
				let check = RbTreeNode::alloc(v as u64)?;
				assert!(tree.try_insert(check).is_err());
				RbTreeNode::release(check);
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let ptr = RbTreeNode::alloc(v as u64)?;
				let res = tree.search(ptr);
				assert!(!res.cur.is_null());
				assert_eq!(*(*(res.cur)).value, v as u64);
				RbTreeNode::release(ptr);
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let node_out = tree.remove(v as u64).unwrap();
				RbTreeNode::release(node_out);
				validate_tree(tree.root());
			}
		}

		Ok(())
	}
	/*

		#[derive(Debug, PartialEq, Clone, PartialOrd, Eq)]
		struct TestTransplant {
			x: u64,
			y: u64,
		}

		impl Ord for TestTransplant {
			fn cmp(&self, other: &Self) -> Ordering {
				self.x.cmp(&other.x)
			}
		}

		#[test]
		fn test_transplant() {
			let mut tree = RbTree::new();

			{
				let size = 3;
				for i in 0..size {
					let v = TestTransplant { x: i, y: i };
					let next = Ptr::alloc(RbTreeNode::new(v)).unwrap();
					let res = tree.insert(next);
					assert!(res.is_none());
				}

				for i in 0..size {
					let v = TestTransplant { x: i, y: i };
					let ptr = Ptr::alloc(RbTreeNode::new(v.clone())).unwrap();
					let res = tree.search(tree.root(), ptr);
					assert!(!res.cur.is_null());
					assert_eq!((*(res.cur)).value, v);
					ptr.release();
				}

				for i in 0..size {
					let v = TestTransplant { x: i, y: i + 1 };
					let next = Ptr::alloc(RbTreeNode::new(v)).unwrap();
					let res = tree.insert(next);
					assert!(res.is_some());
					res.unwrap().release();
				}

				for i in 0..size {
					let v = TestTransplant { x: i, y: i + 1 };
					let ptr = Ptr::alloc(RbTreeNode::new(v.clone())).unwrap();
					let res = tree.search(tree.root(), ptr);
					assert!(!res.cur.is_null());
					assert_eq!((*(res.cur)).value, v);
					ptr.release();
				}

				for i in 0..size {
					let v = TestTransplant { x: i, y: i + 91 };
					let ptr = Ptr::alloc(RbTreeNode::new(v)).unwrap();
					let res = tree.remove_ptr(ptr);
					res.unwrap().release();
					let res = tree.search(tree.root(), ptr);
					assert!(res.cur.is_null());
					ptr.release();
				}

				for i in 0..size {
					let v = TestTransplant { x: i, y: i + 10 };
					let next = Ptr::alloc(RbTreeNode::new(v)).unwrap();
					let res = tree.insert(next);
					assert!(res.is_none());
				}

				for i in 0..size {
					let v = TestTransplant { x: i, y: i + 10 };
					let ptr = Ptr::alloc(RbTreeNode::new(v.clone())).unwrap();
					let res = tree.search(tree.root(), ptr);
					assert!(!res.cur.is_null());
					assert_eq!((*(res.cur)).value, v);
					ptr.release();
				}

				for i in 0..size {
					let v = TestTransplant { x: i, y: i + 91 };
					let ptr = Ptr::alloc(RbTreeNode::new(v)).unwrap();
					let res = tree.remove_ptr(ptr);
					res.unwrap().release();
					let res = tree.search(tree.root(), ptr);
					assert!(res.cur.is_null());
					ptr.release();
				}
			}
		}
	*/

	#[test]
	fn test_rbtree_iter() -> Result<()> {
		let mut tree = RbTree::new();

		let size = 100;
		for x in 0..5 {
			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let next = RbTreeNode::alloc(v as u64)?;
				assert!(tree.try_insert(next).is_ok());
				validate_tree(tree.root());
			}

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let node = RbTreeNode::stack(v as u64)?;
				let ptr = Ptr::new(&node as *const _);
				let res = tree.search(ptr);
				assert!(!res.cur.is_null());
				assert_eq!(*(*(res.cur)).value, v as u64);
			}

			let mut i = 0;
			let mut last = 0;
			for v in tree.iter() {
				i += 1;
				assert!(last < *v);
				last = *v;
			}
			assert_eq!(i, size);

			for i in 0..size {
				let v = fnvhash_32_of_u64(i + x);
				let node_out = tree.remove(v as u64).unwrap();
				RbTreeNode::release(node_out);
				validate_tree(tree.root());
			}
		}

		Ok(())
	}

	#[test]
	fn test_three_rbtree_iters() -> Result<()> {
		let mut tree = RbTree::new();

		let size = 100;
		for i in 0..size {
			let next = RbTreeNode::alloc(i as u64)?;
			assert!(tree.insert(next).is_none());
			validate_tree(tree.root());
		}

		let mut i = 0;
		for v in tree.iter() {
			assert_eq!(*v, i);
			i += 1;
		}
		assert_eq!(i, size);

		i = 0;
		for v in &tree {
			assert_eq!(*v, i);
			i += 1;
		}
		assert_eq!(i, size);

		assert_eq!(tree.len(), size as usize);
		for i in 0..size {
			RbTreeNode::release(tree.remove(i).unwrap());
		}
		assert_eq!(tree.len(), 0);

		let size = 100;
		for i in 0..size {
			let next = RbTreeNode::alloc(i as u64)?;
			assert!(tree.insert(next).is_none());
			validate_tree(tree.root());
		}

		let mut i = 0;
		for v in tree.iter() {
			assert_eq!(*v, i);
			i += 1;
		}
		assert_eq!(i, size);

		i = 0;
		for v in &tree {
			assert_eq!(*v, i);
			i += 1;
		}
		assert_eq!(i, size);

		assert_eq!(tree.len(), size as usize);

		for i in 0..size {
			let ptr_target = RbTreeNode::stack(i)?;
			let ptr = Ptr::new(&ptr_target as *const RbTreeNode<u64>);
			let res = tree.remove_ptr(ptr).unwrap();
			RbTreeNode::release(res);
			validate_tree(tree.root());
			let res = tree.search(ptr);
			assert!(res.cur.is_null());
		}

		assert_eq!(tree.len(), 0);

		Ok(())
	}

	struct DropMeInner {
		s: String,
		v: u64,
	}

	#[derive(Clone)]
	struct DropMe {
		inner: Rc<DropMeInner>,
	}

	impl PartialEq for DropMe {
		fn eq(&self, other: &DropMe) -> bool {
			self.inner.v == other.inner.v
		}
	}
	impl Eq for DropMe {}
	impl Ord for DropMe {
		fn cmp(&self, other: &Self) -> Ordering {
			self.inner.v.cmp(&other.inner.v)
		}
	}
	impl PartialOrd for DropMe {
		fn partial_cmp(&self, other: &DropMe) -> Option<Ordering> {
			self.inner.v.partial_cmp(&other.inner.v)
		}
	}

	#[test]
	fn test_rc_rbtree() -> Result<()> {
		let size = 100;
		let mut tree = RbTree::new();

		for i in 0..size {
			let v = DropMe {
				inner: Rc::new(DropMeInner {
					s: String::new("hi")?,
					v: i,
				})?,
			};
			assert_eq!(v.inner.s, String::new("hi")?);
			let next = RbTreeNode::alloc(v)?;
			assert!(tree.insert(next).is_none());
		}

		let mut i = 0;
		for v in tree.iter() {
			assert_eq!((*v).inner.v, i);
			i += 1;
		}
		assert_eq!(i, size as u64);

		i = 0;
		for v in &tree {
			assert_eq!((*v).inner.v, i);
			i += 1;
		}
		assert_eq!(i, size as u64);

		assert_eq!(tree.len(), size as usize);
		for i in 0..size {
			let v = DropMe {
				inner: Rc::new(DropMeInner {
					s: String::new("hi")?,
					v: i,
				})?,
			};
			let ret = tree.remove(v).unwrap();
			RbTreeNode::release(ret);
		}

		Ok(())
	}
}
