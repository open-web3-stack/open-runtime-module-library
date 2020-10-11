//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn dispatch_as() -> Weight {
		(43_132_000 as Weight)
	}
	fn schedule_dispatch_without_delay() -> Weight {
		(123_715_000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn schedule_dispatch_with_delay() -> Weight {
		(116_719_000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn fast_track_scheduled_dispatch() -> Weight {
		(59_055_000 as Weight)
	}
	fn delay_scheduled_dispatch() -> Weight {
		(44_796_000 as Weight)
	}
	fn cancel_scheduled_dispatch() -> Weight {
		(140_123_000 as Weight)
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
}
