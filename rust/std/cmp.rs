#[derive(PartialEq)]
pub enum Order {
	Equal,
	Less,
	Greater,
}

pub trait Ord {
	fn cmp(&self, other: &Self) -> Order;
}

impl Ord for i32 {
	fn cmp(&self, other: &Self) -> Order {
		if *self < *other {
			Order::Less
		} else if *self > *other {
			Order::Greater
		} else {
			Order::Equal
		}
	}
}

impl Ord for u64 {
	fn cmp(&self, other: &Self) -> Order {
		if *self < *other {
			Order::Less
		} else if *self > *other {
			Order::Greater
		} else {
			Order::Equal
		}
	}
}
