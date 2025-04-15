#[derive(PartialEq)]
pub enum Ordering {
	Equal,
	Less,
	Greater,
}

pub trait Ord {
	fn cmp(&self, other: &Self) -> Ordering;
}

impl Ord for i32 {
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

impl Ord for u64 {
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
