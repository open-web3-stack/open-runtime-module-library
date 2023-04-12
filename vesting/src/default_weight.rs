//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn vested_transfer() -> Weight {
		Weight::from_parts(310_862_000, 0)
			.saturating_add(DbWeight::get().reads(4 as u64))
			.saturating_add(DbWeight::get().writes(4 as u64))
	}
	fn claim(i: u32) -> Weight {
		Weight::from_parts(158_614_000, 0)
			.saturating_add(Weight::from_parts(958_000, 0).saturating_mul(i as u64))
			.saturating_add(DbWeight::get().reads(3 as u64))
			.saturating_add(DbWeight::get().writes(3 as u64))
	}
	fn update_vesting_schedules(i: u32) -> Weight {
		Weight::from_parts(119_811_000, 0)
			.saturating_add(Weight::from_parts(2_320_000, 0).saturating_mul(i as u64))
			.saturating_add(DbWeight::get().reads(2 as u64))
			.saturating_add(DbWeight::get().writes(3 as u64))
	}
}
