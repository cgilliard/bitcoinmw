#[derive(PartialEq)]
pub enum Ordering {
	Equal,
	Less,
	Greater,
}

pub trait Ord {
	fn cmp(&self, other: &Self) -> Ordering;
}

macro_rules! impl_ord {
	($type:ident) => {
		impl Ord for $type {
			fn cmp(&self, other: &Self) -> Ordering {
				if *self < *other {
					Ordering::Less
				} else if *self > *other {
					Ordering::Greater
				} else {
					Ordering::Equal
				}
			}
		}
	};
}

impl_ord!(i8);
impl_ord!(i16);
impl_ord!(i32);
impl_ord!(i64);
impl_ord!(i128);
impl_ord!(u8);
impl_ord!(u16);
impl_ord!(u32);
impl_ord!(u64);
impl_ord!(u128);
impl_ord!(usize);
impl_ord!(isize);
