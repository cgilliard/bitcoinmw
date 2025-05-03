use core::convert::{AsMut, AsRef};
use core::marker::PhantomData;
use core::mem::{drop, needs_drop, size_of};
use core::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeTo};
use core::ptr;
use core::ptr::{drop_in_place, null_mut, write_bytes};
use core::slice::{from_raw_parts, from_raw_parts_mut};
use prelude::*;
use std::constants::VEC_MIN_SIZE;
use std::ffi::{alloc, release, resize};
use std::misc::slice_copy;

pub struct Vec<T> {
	value: Ptr<u8>,
	capacity: usize,
	elements: usize,
	_marker: PhantomData<T>,
}

impl<T: Debug> Debug for Vec<T> {
	fn fmt(&self, _f: &mut CoreFormatter<'_>) -> FmtResult {
		#[cfg(test)]
		write!(_f, "{:?}", self.as_ref())?;
		Ok(())
	}
}

impl<T: Display> Display for Vec<T> {
	fn format(&self, f: &mut Formatter) -> Result<()> {
		let mut first = true;
		for x in self {
			if first {
				writef!(f, "[{}", x)?;
			} else {
				writef!(f, ", {}", x)?;
			}
			first = false;
		}
		if first {
			writef!(f, "[")?;
		}
		writef!(f, "]")
	}
}

impl<T> AsRef<[T]> for Vec<T> {
	fn as_ref(&self) -> &[T] {
		self.slice_all()
	}
}

impl<T> AsMut<[T]> for Vec<T> {
	fn as_mut(&mut self) -> &mut [T] {
		self.slice_mut_all()
	}
}

impl<T> Deref for Vec<T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T> DerefMut for Vec<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}

impl<T: TryClone> TryClone for Vec<T> {
	fn try_clone(&self) -> Result<Self>
	where
		Self: Sized,
	{
		// Allocate new Vec with same capacity
		let mut v = Vec::with_capacity(self.capacity)?;

		// Clone elements one by one
		let mut i = 0;
		while i < self.elements {
			match self[i].try_clone() {
				Ok(cloned) => {
					// Write cloned element to uninitialized slot
					unsafe {
						let dest_ptr = v.value.raw() as *mut u8;
						let dest_ptr = dest_ptr.add(size_of::<T>() * i) as *mut T;
						ptr::write(dest_ptr, cloned);
					}
					i += 1;
					v.elements = i; // Update elements for drop safety
				}
				Err(e) => {
					// Drop cloned elements and return error
					v.elements = i; // Ensure only cloned elements are dropped
					drop(v);
					return Err(e);
				}
			}
		}
		Ok(v)
	}
}

impl<T: PartialEq> PartialEq for Vec<T> {
	fn eq(&self, other: &Vec<T>) -> bool {
		if self.len() != other.len() {
			false
		} else {
			for i in 0..self.len() {
				if self[i] != other[i] {
					return false;
				}
			}
			true
		}
	}
}

impl<T> Drop for Vec<T> {
	fn drop(&mut self) {
		let raw = self.value.raw();
		if !raw.is_null() {
			if needs_drop::<T>() {
				for i in 0..self.elements {
					unsafe {
						let ptr = (raw as *const u8).add(i * size_of::<T>()) as *mut T;
						drop_in_place(ptr);
					}
				}
			}
			unsafe {
				release(raw as *const u8);
			}
		}
	}
}

impl<T> Index<Range<usize>> for Vec<T> {
	type Output = [T];
	fn index(&self, r: Range<usize>) -> &Self::Output {
		let slice = self.slice(r.start, r.end);
		&slice
	}
}

impl<T> IndexMut<Range<usize>> for Vec<T> {
	fn index_mut(&mut self, r: Range<usize>) -> &mut <Self as Index<Range<usize>>>::Output {
		self.slice_mut(r.start, r.end)
	}
}

impl<T> Index<RangeFrom<usize>> for Vec<T> {
	type Output = [T];
	fn index(&self, r: RangeFrom<usize>) -> &Self::Output {
		self.slice(r.start, self.len())
	}
}

impl<T> IndexMut<RangeFrom<usize>> for Vec<T> {
	fn index_mut(&mut self, r: RangeFrom<usize>) -> &mut Self::Output {
		self.slice_mut(r.start, self.len())
	}
}

impl<T> Index<RangeTo<usize>> for Vec<T> {
	type Output = [T];
	fn index(&self, r: RangeTo<usize>) -> &Self::Output {
		self.slice(0, r.end)
	}
}

impl<T> IndexMut<RangeTo<usize>> for Vec<T> {
	fn index_mut(&mut self, r: RangeTo<usize>) -> &mut Self::Output {
		self.slice_mut(0, r.end)
	}
}

impl<T> Index<RangeFull> for Vec<T> {
	type Output = [T];
	fn index(&self, _r: RangeFull) -> &Self::Output {
		self.slice(0, self.len())
	}
}

impl<T> IndexMut<RangeFull> for Vec<T> {
	fn index_mut(&mut self, _r: RangeFull) -> &mut Self::Output {
		self.slice_mut(0, self.len())
	}
}

impl<T> Index<usize> for Vec<T> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		if index >= self.elements as usize {
			exit!("array index out of bounds!");
		}

		unsafe {
			let target = self.value.raw() as *const T;
			let target = target.add(index);
			&*(target as *const T)
		}
	}
}

impl<T> IndexMut<usize> for Vec<T> {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		if index >= self.elements as usize {
			exit!("array index out of bounds!");
		}

		unsafe {
			let target = self.value.raw() as *const T;
			let target = target.add(index);
			&mut *(target as *mut T)
		}
	}
}

pub struct VecIterator<T> {
	vec: Vec<T>,
	index: usize,
	len: usize,
}

impl<T> Iterator for VecIterator<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		let size = size_of::<T>();
		if self.index < self.len {
			let ptr = self.vec.value.raw() as *const u8;
			let ptr = unsafe { ptr.add(self.index * size) as *mut T };
			let element = unsafe { ptr::read(ptr) }; // Move the element out
			self.index += 1;
			Some(element)
		} else {
			None
		}
	}
}

impl<T> Drop for VecIterator<T> {
	fn drop(&mut self) {
		if self.vec.value.raw().is_null() {
			return;
		}
		if needs_drop::<T>() {
			let size = size_of::<T>();
			while self.index < self.vec.elements {
				let ptr = unsafe { self.vec.value.raw().add(self.index * size) as *mut T };
				unsafe { ptr::drop_in_place(ptr) };
				self.index += 1;
			}
		}
		self.vec.elements = 0;
	}
}

impl<T> IntoIterator for Vec<T> {
	type Item = T;
	type IntoIter = VecIterator<T>;

	fn into_iter(self) -> Self::IntoIter {
		let len = self.elements;
		let ret = VecIterator {
			vec: self,
			index: 0,
			len,
		};
		ret
	}
}

pub struct VecRefMutIterator<'a, T> {
	vec: &'a mut Vec<T>,
	index: usize,
}

impl<'a, T> Iterator for VecRefMutIterator<'a, T> {
	type Item = &'a mut T;

	fn next(&mut self) -> Option<Self::Item> {
		if self.index < self.vec.elements && !self.vec.value.raw().is_null() {
			unsafe {
				let ptr = self.vec.value.raw() as *mut T;
				let item_ptr = ptr.add(self.index);
				self.index += 1;
				Some(&mut *item_ptr)
			}
		} else {
			None
		}
	}
}

pub struct VecRefIterator<'a, T> {
	vec: &'a Vec<T>,
	index: usize,
}

impl<'a, T> Iterator for VecRefIterator<'a, T> {
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item> {
		if self.index < self.vec.elements && !self.vec.value.raw().is_null() {
			unsafe {
				let ptr = self.vec.value.raw() as *const T;
				let item_ptr = ptr.add(self.index);
				self.index += 1;
				Some(&*item_ptr)
			}
		} else {
			None
		}
	}
}

impl<'a, T> IntoIterator for &'a Vec<T> {
	type Item = &'a T;
	type IntoIter = VecRefIterator<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		VecRefIterator {
			vec: &self,
			index: 0,
		}
	}
}

impl<T: Copy> Vec<T> {
	pub fn resize(&mut self, n: usize) -> Result<()> {
		self.resize_impl(n)?;
		self.elements = n;
		Ok(())
	}
}

impl<T> Vec<T> {
	pub fn new() -> Self {
		let value = Ptr::null();
		let capacity = 0;
		let elements = 0;

		Self {
			value,
			capacity,
			elements,
			_marker: PhantomData,
		}
	}

	pub fn with_capacity(capacity: usize) -> Result<Self> {
		if capacity == 0 {
			return Ok(Self::new());
		}
		let ptr = unsafe { alloc(capacity * size_of::<T>()) };
		if ptr.is_null() {
			err!(Alloc)
		} else {
			Ok(Self {
				value: Ptr::new(ptr as *const u8),
				capacity,
				elements: 0,
				_marker: PhantomData,
			})
		}
	}

	pub fn allow_zero_alloc(&mut self, v: bool) {
		self.value.set_bit(v);
	}

	pub fn push(&mut self, v: T) -> Result<()> {
		let size = size_of::<T>();

		if self.elements + 1 > self.capacity {
			self.resize_impl(self.elements + 1)?;
		}

		let dest_ptr = self.value.raw() as *mut u8;
		unsafe {
			let dest_ptr = dest_ptr.add(size * self.elements) as *mut T;
			ptr::write(dest_ptr, v);
		}
		self.elements += 1;

		Ok(())
	}

	pub fn extend(&mut self, v: &Vec<T>) -> Result<()>
	where
		T: Copy,
	{
		self.extend_from_slice(v.slice_all())
	}

	pub unsafe fn force_resize(&mut self, n: usize) -> Result<()> {
		self.resize_impl(n)?;
		self.elements = n;
		Ok(())
	}

	pub fn extend_from_slice(&mut self, other: &[T]) -> Result<()>
	where
		T: Copy,
	{
		let len = self.len();
		let other_len = other.len();
		self.resize_impl(other_len + len)?;
		self.elements = other_len + len;
		slice_copy(other, self.slice_mut_from(len), other.len())
	}

	pub fn iter_mut(&mut self) -> VecRefMutIterator<'_, T> {
		VecRefMutIterator {
			vec: self,
			index: 0,
		}
	}

	pub fn clear(&mut self) {
		let _ = self.truncate(0);
		let _ = self.resize_impl(0);
	}

	pub fn truncate(&mut self, n: usize) -> Result<()> {
		if n > self.elements {
			return err!(IllegalArgument);
		}

		// Drop elements from n to self.elements
		if needs_drop::<T>() {
			for i in n..self.elements {
				unsafe {
					let ptr = self.value.raw().add(i * size_of::<T>()) as *mut T;
					ptr::drop_in_place(ptr);
				}
			}
		}

		self.elements = n;
		Ok(())
	}

	pub fn len(&self) -> usize {
		self.elements
	}

	pub fn as_mut_ptr(&mut self) -> *mut T {
		self.value.raw() as *mut T
	}

	pub fn as_ptr(&self) -> *const T {
		self.value.raw() as *const T
	}

	pub fn slice(&self, start: usize, end: usize) -> &[T] {
		if start > end || end > self.elements {
			exit!(
				"Slice out of bounds: {}..{} > {}",
				start,
				end,
				self.elements
			);
		} else if start == end {
			&[]
		} else {
			let size = size_of::<T>();
			unsafe { from_raw_parts(self.value.raw().add(start * size) as *const T, end - start) }
		}
	}

	pub fn slice_mut(&mut self, start: usize, end: usize) -> &mut [T] {
		if start > end || end > self.elements {
			exit!(
				"Slice out of bounds: {}..{} > {}",
				start,
				end,
				self.elements
			);
		} else if start == end {
			&mut []
		} else {
			let size = size_of::<T>();
			unsafe { from_raw_parts_mut(self.value.raw().add(start * size) as *mut T, end - start) }
		}
	}

	pub fn slice_all(&self) -> &[T] {
		self.slice(0, self.len())
	}

	pub fn slice_mut_all(&mut self) -> &mut [T] {
		self.slice_mut(0, self.len())
	}

	pub fn slice_to(&self, end: usize) -> &[T] {
		self.slice(0, end)
	}

	pub fn slice_mut_to(&mut self, end: usize) -> &mut [T] {
		self.slice_mut(0, end)
	}

	pub fn slice_from(&self, start: usize) -> &[T] {
		self.slice(start, self.len())
	}

	pub fn slice_mut_from(&mut self, start: usize) -> &mut [T] {
		self.slice_mut(start, self.len())
	}

	fn next_power_of_two(&self, mut n: usize) -> usize {
		if self.value.get_bit() && n == 0 {
			return 0;
		}
		if n < VEC_MIN_SIZE {
			return VEC_MIN_SIZE;
		}
		if n == 0 {
			return 0;
		}
		n -= 1;
		n |= n >> 1;
		n |= n >> 2;
		n |= n >> 4;
		n |= n >> 8;
		n |= n >> 16;
		n |= n >> 32;
		n + 1
	}

	fn resize_impl(&mut self, needed: usize) -> Result<()> {
		let ncapacity = self.next_power_of_two(needed);

		if ncapacity == self.capacity {
			return Ok(());
		}

		let rptr = self.value.raw();

		let nptr = if ncapacity == 0 {
			if !rptr.is_null() {
				unsafe {
					release(rptr as *const u8);
				}
			}
			null_mut()
		} else if rptr.is_null() {
			unsafe { alloc(ncapacity * size_of::<T>()) }
		} else {
			unsafe { resize(rptr as *const u8, ncapacity * size_of::<T>()) }
		};

		if !nptr.is_null() {
			if ncapacity > self.capacity {
				let old_size = self.capacity * size_of::<T>();
				let new_size = ncapacity * size_of::<T>();
				unsafe {
					write_bytes((nptr as *mut u8).add(old_size), 0, new_size - old_size);
				}
			}
			self.capacity = ncapacity;
			let mut nptr = Ptr::new(nptr as *mut u8);
			if self.value.get_bit() {
				nptr.set_bit(true);
			}
			if self.value.raw().is_null() {
				self.value = nptr;
			} else {
				self.value = nptr;
			}
			Ok(())
		} else {
			self.value = Ptr::null();
			if ncapacity == 0 {
				Ok(())
			} else {
				err!(Alloc)
			}
		}
	}
}

#[cfg(test)]
mod test {
	#![allow(unused_mut)]

	use super::*;

	#[test]
	fn test_vec1() {
		let mut x = vec![1, 2, 3, 4, 5, 6].unwrap();
		assert_eq!(x[0], 1);
		assert_eq!(x[1], 2);
		assert_eq!(x[2], 3);
		assert_eq!(x[3], 4);
		assert_eq!(x[4], 5);
		assert_eq!(x[5], 6);
		assert_eq!(x.len(), 6);
		x[5] += 1;
		assert_eq!(x[5], 7);
	}

	#[test]
	fn test_vec2() {
		let mut v1 = Vec::new();
		for i in 0..100000 {
			assert!(v1.push(i).is_ok());
			assert_eq!(v1[i], i);
		}

		for i in 0..100000 {
			v1[i] = i + 100;
		}
		for i in 0..100000 {
			assert_eq!(v1[i], i + 100);
		}

		let v2 = vec![1, 2, 3].unwrap();
		let mut count = 0;
		for x in v2 {
			count += 1;
			assert_eq!(x, count);
		}
		assert_eq!(count, 3);
	}

	#[test]
	fn test_vec_append() {
		let mut v1 = vec![1u64, 2, 3].unwrap();
		let v2 = vec![4u64, 5, 6].unwrap();
		assert!(v1.extend_from_slice(&v2).is_ok());
		assert_eq!(v1.len(), 6);
		assert_eq!(v2.len(), 3);

		assert_eq!(v1, vec![1, 2, 3, 4, 5, 6].unwrap());
		assert!(v1 != vec![1, 2, 3, 4, 6, 6].unwrap());
		assert!(v1 == vec![1, 2, 3, 4, 5, 6].unwrap());
		assert!(v1 != v2);

		// try a u8 version
		let mut v1 = vec![1u8, 2, 3].unwrap();
		let v2 = vec![4u8, 5, 6].unwrap();
		assert!(v1.extend_from_slice(&v2).is_ok());

		assert_eq!(v1, vec![1, 2, 3, 4, 5, 6].unwrap());
		assert!(v1 != vec![1, 2, 3, 4, 6, 6].unwrap());
		assert!(v1 == vec![1, 2, 3, 4, 5, 6].unwrap());
		assert!(v1 != v2);
		assert_eq!(v1.len(), 6);
		assert_eq!(v2.len(), 3);
	}

	struct DropTest {
		x: u32,
	}

	static mut VTEST: u32 = 0;

	impl Drop for DropTest {
		fn drop(&mut self) {
			unsafe {
				VTEST += 1;
			}
		}
	}

	#[test]
	fn test_vec_drop() {
		{
			let x = DropTest { x: 8 };

			let mut v: Vec<DropTest> = vec![].unwrap();
			assert!(unsafe { v.force_resize(1).is_ok() });
			v[0] = x;
			assert_eq!(v[0].x, 8);
		}

		assert_eq!(unsafe { VTEST }, 2);
	}

	struct DropTest2 {
		x: u32,
	}

	static mut VTEST2: u32 = 0;

	impl Drop for DropTest2 {
		fn drop(&mut self) {
			unsafe {
				VTEST2 += 1;
			}
		}
	}

	#[test]
	fn test_vec_iter_drop() {
		unsafe {
			VTEST2 = 0;
		}
		{
			let v = vec![DropTest2 { x: 1 }, DropTest2 { x: 2 }, DropTest2 { x: 3 }].unwrap();
			assert_eq!(v[0].x, 1);
			for y in v {
				let _z = y;
			}
		}
		assert_eq!(unsafe { VTEST2 }, 3);
	}

	#[test]
	fn test_set_min0() {
		let mut v = Vec::new();
		v.allow_zero_alloc(true);
		assert!(v.push(1).is_ok());
		assert!(v.resize(128).is_ok());
		assert!(v.resize(0).is_ok());

		let mut v = Vec::new();
		assert!(v.push(1).is_ok());
		v.allow_zero_alloc(true);
		assert!(v.resize(128).is_ok());
		assert!(v.resize(0).is_ok());
	}

	#[test]
	fn test_vec_range() {
		let mut v = vec![1, 2, 3, 4, 5].unwrap();
		let r = &v.slice(1, 3);
		assert_eq!(r[0], 2);
		assert_eq!(&v.slice(1, 3), &vec![2, 3].unwrap().slice(0, 2));

		let slice = &mut v.slice(1, 1);
		assert_eq!(slice, &mut []);

		v.clear();
		assert_eq!(v.len(), 0);

		let mut v = vec![1, 2, 3, 4, 5, 6, 7].unwrap();
		let r = &mut v.slice_mut(1, 3);
		assert_eq!(r[0], 2);
		assert_eq!(r[1], 3);
		r[0] = 9;
		assert_eq!(r[0], 9);
		assert_eq!(r[1], 3);
	}

	#[test]
	fn iter_ret() {
		let mut v = vec![1, 2, 3, 4].unwrap();
		let mut i = 1;
		for x in &v {
			assert_eq!(x, &i);
			i += 1;
		}
		assert_eq!(i, 5);

		for x in v.iter_mut() {
			*x += 1;
		}

		i = 2;
		for x in v {
			assert_eq!(x, i);
			i += 1;
		}
		assert_eq!(i, 6);
	}

	#[test]
	fn test_as_ref() -> Result<()> {
		let mut v = vec![1, 2, 3]?;
		assert_eq!(v.as_mut(), &mut [1, 2, 3]);
		Ok(())
	}

	#[test]
	fn test_try_clone() -> Result<()> {
		let v1 = vec![1, 2, 3, 4]?;
		let v2 = v1.try_clone()?;
		assert_eq!(v1, v2);

		let x1 = vec![1, 2, 3]?;
		let x2 = vec![4, 5, 6]?;
		let x3 = vec![7, 8, 9]?;

		let y1 = vec![9, 9, 9, 9]?;
		let y2 = vec![10, 10, 10]?;
		let y3 = vec![11, 11]?;
		let y4 = vec![12]?;

		let y = vec![y1, y2, y3, y4]?;
		let x = vec![x1, x2, x3]?;

		let z1 = vec![y, x]?;
		let z2 = z1.try_clone()?;
		assert_eq!(z1, z2);

		assert_eq!(z1[1][2][1], 8);
		assert_eq!(z2[1][1][0], 4);
		Ok(())
	}

	#[test]
	fn test_extend_from_slice() -> Result<()> {
		let mut v1 = vec![1, 2, 3, 4]?;
		let v2 = vec![5, 6, 7, 8, 9, 10]?;
		v1.extend_from_slice(v2.as_ref())?;
		assert_eq!(v1, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]?);

		Ok(())
	}

	#[test]
	fn test_three_iters() -> Result<()> {
		let mut vec = vec![0, 1, 2, 3]?;
		let mut i = 0;
		for v in vec.iter() {
			assert_eq!(*v, i);
			i += 1;
		}
		assert_eq!(i, 4);
		i = 0;
		for v in vec.iter_mut() {
			*v += 1;
			assert_eq!(*v, i + 1);
			i += 1;
		}
		assert_eq!(i, 4);
		i = 1;
		for v in vec {
			assert_eq!(v, i);
			i += 1;
		}
		assert_eq!(i, 5);

		Ok(())
	}

	#[test]
	fn test_sort() -> Result<()> {
		let mut v = vec![9, 7, 8, 2, 10]?;
		let _ = &mut v[..].sort();
		assert_eq!(v, vec![2, 7, 8, 9, 10]?);

		Ok(())
	}
}
