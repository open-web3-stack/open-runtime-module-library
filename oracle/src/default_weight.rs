//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn feed_values(c: u32) -> Weight {
		(74_792_000 as Weight)
			.saturating_add((11_208_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
			.saturating_add(DbWeight::get().writes((2 as Weight).saturating_mul(c as Weight)))
	}
	fn on_finalize() -> Weight {
		(15_881_000 as Weight).saturating_add(DbWeight::get().writes(1 as Weight))
	}
}
