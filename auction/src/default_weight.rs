//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn bid_collateral_auction() -> Weight {
		(493_957_000 as Weight)
			.saturating_add(DbWeight::get().reads(12 as Weight))
			.saturating_add(DbWeight::get().writes(12 as Weight))
	}
	fn bid_surplus_auction() -> Weight {
		(257_830_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(5 as Weight))
	}
	fn bid_debit_auction() -> Weight {
		(287_271_000 as Weight)
			.saturating_add(DbWeight::get().reads(7 as Weight))
			.saturating_add(DbWeight::get().writes(8 as Weight))
	}
	fn on_finalize(c: u32) -> Weight {
		(50_992_000 as Weight)
			.saturating_add((171_653_000 as Weight).saturating_mul(c as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().reads((4 as Weight).saturating_mul(c as Weight)))
			.saturating_add(DbWeight::get().writes(2 as Weight))
			.saturating_add(DbWeight::get().writes((3 as Weight).saturating_mul(c as Weight)))
	}
}
