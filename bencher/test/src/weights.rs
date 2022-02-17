

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

pub struct ModuleWeights<T>(PhantomData<T>);
impl<T: frame_system::Config> ModuleWeights<T> {
	// Storage access info
	//
	// Test::Value (r: 1, w: 1)
	pub fn set_value() -> Weight {
		(5_236_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage access info
	//
	// Test::Bar (r: 1, w: 2)
	// Test::Foo (r: 0, w: 1)
	// Test::Value (r: 0, w: 1)
	pub fn set_foo() -> Weight {
		(13_274_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage access info
	//
	pub fn remove_all_bar() -> Weight {
		(3_449_000 as Weight)
	}
}
