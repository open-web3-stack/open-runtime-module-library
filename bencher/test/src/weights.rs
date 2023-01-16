

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
	// Test::Bar (r: 0, w: 1)
	pub fn whitelist() -> Weight {
		Weight::from_ref_time(5_356_000)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage access info
	//
	// Test::Value (r: 1, w: 1)
	// Unknown 0x3a7472616e73616374696f6e5f6c6576656c3a (r: 1, w: 1)
	pub fn set_value() -> Weight {
		Weight::from_ref_time(3_919_000)
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	// Storage access info
	//
	// Test::Bar (r: 1, w: 2)
	// Test::Foo (r: 0, w: 1)
	// Test::Value (r: 0, w: 1)
	pub fn set_foo() -> Weight {
		Weight::from_ref_time(5_133_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	// Storage access info
	//
	pub fn remove_all_bar() -> Weight {
		Weight::from_ref_time(1_533_000)
	}
	// Storage access info
	//
	// Test::Bar (r: 0, w: 10)
	pub fn remove_all_bar_with_limit() -> Weight {
		Weight::from_ref_time(1_600_000)
			.saturating_add(T::DbWeight::get().writes(10))
	}
}
