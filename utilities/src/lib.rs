#![cfg_attr(not(feature = "std"), no_std)]

pub mod linked_list;

pub use linked_list::{LinkedItem, LinkedList};

pub fn add(a: u32, b: u32) -> u32 {
	a + b
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn it_works() {
		assert_eq!(add(1, 1), 2);
	}
}
