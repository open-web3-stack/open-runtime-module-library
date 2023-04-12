//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn gradually_update() -> Weight {
		Weight::from_parts(57_922_000, 0)
			.saturating_add(DbWeight::get().reads(2 as u64))
			.saturating_add(DbWeight::get().writes(1 as u64))
	}
	fn cancel_gradually_update() -> Weight {
		Weight::from_parts(66_687_000, 0)
			.saturating_add(DbWeight::get().reads(1 as u64))
			.saturating_add(DbWeight::get().writes(1 as u64))
	}
	fn on_finalize(u: u32) -> Weight {
		Weight::from_parts(37_067_000, 0)
			.saturating_add(Weight::from_parts(20_890_000, 0).saturating_mul(u as u64))
			.saturating_add(DbWeight::get().reads(3 as u64))
			.saturating_add(DbWeight::get().writes(3 as u64))
	}
}
