use core::convert::{AsMut, AsRef};
use core::marker::PhantomData;
use core::mem::{needs_drop, size_of};
use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::ptr;
use core::ptr::{drop_in_place, null_mut, write_bytes};
use core::slice::{from_raw_parts, from_raw_parts_mut};
use prelude::*;
use std::ffi::{alloc, release, resize};
use std::misc::array_copy;

pub struct Vec<T> {
	value: Ptr<u8>,
	capacity: usize,
	elements: usize,
	min: usize,
	_marker: PhantomData<T>,
}

impl<T> AsRef<[T]> for Vec<T> {
	fn as_ref(&self) -> &[T] {
		self.as_slice()
	}
}

impl<T> AsMut<[T]> for Vec<T> {
	fn as_mut(&mut self) -> &mut [T] {
		self.as_mut_slice()
	}
}

impl<T> Deref for Vec<T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		self.as_slice()
	}
}

impl<T> DerefMut for Vec<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut_slice()
	}
}

impl<T: Clone> TryClone for Vec<T> {
	fn try_clone(&self) -> Result<Self, Error>
	where
		Self: Sized,
	{
		match Vec::with_capacity(self.capacity) {
			Ok(mut v) => {
				v.elements = self.elements;
				v.min = self.min;
				for i in 0..v.elements {
					v[i] = self[i].clone();
				}

				Ok(v)
			}
			Err(e) => Err(e),
		}
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
				release(raw);
			}
		}
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

impl<T> Vec<T> {
	pub fn new() -> Self {
		let value = Ptr::null();
		let capacity = 0;
		let elements = 0;
		let min = 0;

		Self {
			value,
			capacity,
			elements,
			min,
			_marker: PhantomData,
		}
	}

	pub fn with_capacity(capacity: usize) -> Result<Self, Error> {
		let ptr = unsafe { alloc(capacity * size_of::<T>()) };
		if ptr.is_null() {
			Err(Error::new(Alloc))
		} else {
			Ok(Self {
				value: Ptr::new(ptr),
				capacity,
				elements: 0,
				min: 0,
				_marker: PhantomData,
			})
		}
	}

	pub fn push(&mut self, v: T) -> Result<(), Error> {
		let size = size_of::<T>();

		if self.elements + 1 > self.capacity {
			if !self.resize_impl(self.elements + 1) {
				return Err(Error::new(Alloc));
			}
		}

		let dest_ptr = self.value.raw() as *mut u8;
		unsafe {
			let dest_ptr = dest_ptr.add(size * self.elements) as *mut T;
			ptr::write(dest_ptr, v);
		}
		self.elements += 1;

		Ok(())
	}

	pub fn iter_mut(&mut self) -> VecRefMutIterator<'_, T> {
		VecRefMutIterator {
			vec: self,
			index: 0,
		}
	}

	pub fn clear(&mut self) -> Result<(), Error> {
		self.truncate(0)?;

		if !self.resize_impl(self.min) {
			Err(Error::new(Alloc))
		} else {
			self.capacity = self.min;
			Ok(())
		}
	}

	pub fn truncate(&mut self, n: usize) -> Result<(), Error> {
		if n > self.elements {
			return Err(Error::new(IllegalArgument));
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

	pub fn resize(&mut self, n: usize) -> Result<(), Error> {
		if self.resize_impl(n) {
			self.elements = n;
			Ok(())
		} else {
			Err(Error::new(Alloc))
		}
	}

	pub fn set_min(&mut self, n: usize) {
		self.min = n;
	}

	pub fn len(&self) -> usize {
		self.elements
	}

	pub fn set_len(&mut self, elements: usize) {
		self.elements = elements;
	}

	pub fn as_mut_ptr(&mut self) -> *mut T {
		self.value.raw() as *mut T
	}

	pub fn as_ptr(&self) -> *const T {
		self.value.raw() as *const T
	}

	pub fn as_slice(&self) -> &[T] {
		unsafe { from_raw_parts(self.value.raw() as *const T, self.elements) }
	}

	pub fn as_mut_slice(&mut self) -> &mut [T] {
		unsafe { from_raw_parts_mut(self.value.raw() as *mut T, self.elements) }
	}

	pub fn append(&mut self, v: &Vec<T>) -> Result<(), Error>
	where
		T: Copy,
	{
		let len = v.len();
		if self.elements + len > self.capacity {
			if !self.resize_impl(self.elements + len) {
				return Err(Error::new(Alloc));
			}
		}

		let size = size_of::<T>();

		let dest_ptr = self.value.raw() as *mut u8;
		let dest_ptr = unsafe {
			let offset = size
				.checked_mul(self.elements)
				.ok_or_else(|| Error::new(Overflow))?;
			if offset > self.capacity * size {
				return Err(Error::new(ArrayIndexOutOfBounds));
			}
			dest_ptr.add(offset)
		};

		let dest_slice = unsafe { from_raw_parts_mut(dest_ptr as *mut T, len) };
		array_copy(v.as_ref(), dest_slice, len)?;

		self.elements += len;
		Ok(())
	}

	pub fn mut_slice(&mut self, start: usize, end: usize) -> &mut [T] {
		if start > end || end > self.elements {
			exit!(
				"Slice out of bounds: {}..{} > {}",
				start,
				end,
				self.elements
			);
		} else {
			let size = size_of::<T>();
			unsafe { from_raw_parts_mut(self.value.raw().add(start * size) as *mut T, end - start) }
		}
	}

	pub fn slice(&self, start: usize, end: usize) -> &[T] {
		if start > end || end > self.elements {
			exit!(
				"Slice out of bounds: {}..{} > {}",
				start,
				end,
				self.elements
			);
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
		} else {
			let size = size_of::<T>();
			unsafe { from_raw_parts_mut(self.value.raw().add(start * size) as *mut T, end - start) }
		}
	}

	fn next_power_of_two(&self, mut n: usize) -> usize {
		if n < self.min {
			return self.min;
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

	fn resize_impl(&mut self, needed: usize) -> bool {
		let ncapacity = self.next_power_of_two(needed);

		if ncapacity == self.capacity {
			return true;
		}

		let rptr = self.value.raw();

		let nptr = if ncapacity == 0 {
			if !rptr.is_null() {
				unsafe {
					release(rptr as *mut u8);
				}
			}
			null_mut()
		} else if rptr.is_null() {
			unsafe { alloc(ncapacity * size_of::<T>()) }
		} else {
			unsafe { resize(rptr as *mut u8, ncapacity * size_of::<T>()) }
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
			let nptr = Ptr::new(nptr as *mut u8);
			if self.value.raw().is_null() {
				self.value = nptr;
			} else {
				self.value = nptr;
			}
			true
		} else {
			self.value = Ptr::null();
			ncapacity == 0
		}
	}
}

#[cfg(test)]
mod test {
	#![allow(unused_mut)]

	use super::*;
	use core::fmt::Formatter as CoreFormatter;
	use std::ffi::getalloccount;

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
		let initial = unsafe { getalloccount() };
		{
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
		unsafe {
			assert_eq!(initial, getalloccount());
		}
	}

	impl<T> Debug for Vec<T> {
		fn fmt(&self, _: &mut CoreFormatter<'_>) -> Result<(), FmtError> {
			Ok(())
		}
	}

	#[test]
	fn test_vec_append() {
		let initial = unsafe { getalloccount() };
		{
			let mut v1 = vec![1u64, 2, 3].unwrap();
			let v2 = vec![4u64, 5, 6].unwrap();
			assert!(v1.append(&v2).is_ok());
			assert_eq!(v1.len(), 6);
			assert_eq!(v2.len(), 3);

			assert_eq!(v1, vec![1, 2, 3, 4, 5, 6].unwrap());
			assert!(v1 != vec![1, 2, 3, 4, 6, 6].unwrap());
			assert!(v1 == vec![1, 2, 3, 4, 5, 6].unwrap());
			assert!(v1 != v2);

			// try a u8 version
			let mut v1 = vec![1u8, 2, 3].unwrap();
			let v2 = vec![4u8, 5, 6].unwrap();
			assert!(v1.append(&v2).is_ok());

			assert_eq!(v1, vec![1, 2, 3, 4, 5, 6].unwrap());
			assert!(v1 != vec![1, 2, 3, 4, 6, 6].unwrap());
			assert!(v1 == vec![1, 2, 3, 4, 5, 6].unwrap());
			assert!(v1 != v2);
			assert_eq!(v1.len(), 6);
			assert_eq!(v2.len(), 3);
		}
		unsafe {
			assert_eq!(initial, getalloccount());
		}
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
		let x = DropTest { x: 8 };

		let initial = unsafe { getalloccount() };
		{
			let mut v: Vec<DropTest> = vec![].unwrap();
			assert!(v.resize(1).is_ok());
			v[0] = x;
			assert_eq!(v[0].x, 8);
		}

		assert_eq!(unsafe { VTEST }, 2);
		unsafe {
			assert_eq!(initial, getalloccount());
		}
	}

	#[test]
	fn test_vec_iter_drop() {
		let initial = unsafe { getalloccount() };
		{
			unsafe {
				VTEST = 0;
			}
			{
				let v = vec![DropTest { x: 1 }, DropTest { x: 2 }, DropTest { x: 3 }].unwrap();
				for y in v {
					let _z = y;
				}
			}
			assert_eq!(unsafe { VTEST }, 3);
		}
		unsafe {
			assert_eq!(initial, getalloccount());
		}
	}

	#[test]
	fn test_set_min0() {
		let initial = unsafe { getalloccount() };
		{
			let mut v = Vec::new();
			v.set_min(0);
			assert!(v.push(1).is_ok());
			assert!(v.resize(128).is_ok());
			assert!(v.resize(0).is_ok());
			// it's already freed at this point
			unsafe {
				assert_eq!(initial, getalloccount());
			}
		}
		unsafe {
			assert_eq!(initial, getalloccount());
		}
	}

	#[test]
	fn test_vec_range() {
		let mut v = vec![1, 2, 3, 4, 5].unwrap();
		let r = &v.slice(1, 3);
		assert_eq!(r[0], 2);
		assert_eq!(&v.slice(1, 3), &vec![2, 3].unwrap().slice(0, 2));

		let slice = &mut v.slice(1, 1);
		assert_eq!(slice, &mut []);

		v.clear().unwrap();
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
	fn test_as_ref() -> Result<(), Error> {
		let mut v = vec![1, 2, 3]?;
		assert_eq!(v.as_mut(), &mut [1, 2, 3]);
		Ok(())
	}
}
