//! Weights for the authority Module

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn dispatch_as() -> Weight {
		50750000 as Weight
	}
	fn schedule_dispatch() -> Weight {
		(147800000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn fast_track_scheduled_dispatch() -> Weight {
		// TODO
		0 as Weight
	}
	fn delay_scheduled_dispatch() -> Weight {
		// TODO
		0 as Weight
	}
	fn cancel_scheduled_dispatch() -> Weight {
		(127400000 as Weight)
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
}
