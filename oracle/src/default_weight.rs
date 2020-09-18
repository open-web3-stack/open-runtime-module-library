//! Weights for the Auction Module

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn feed_values(values_len: usize) -> Weight {
		(101600000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes((1 + values_len * 2) as Weight))
	}

	fn on_initialize() -> Weight {
		(18400000 as Weight).saturating_add(DbWeight::get().writes(1) as Weight)
	}
}
