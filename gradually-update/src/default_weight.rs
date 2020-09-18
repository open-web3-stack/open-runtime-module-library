//! Weights for the Gradually-update Module

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn gradually_update() -> Weight {
		(82320000 as Weight)
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn cancel_gradually_update() -> Weight {
		(72950000 as Weight)
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn on_initialize(need_update: bool, update_len: usize) -> Weight {
		if !need_update {
			return 0;
		}

		if update_len == 0 {
			(30430000 as Weight)
				.saturating_add(DbWeight::get().reads(2 as Weight))
				.saturating_add(DbWeight::get().writes(1 as Weight))
		} else {
			(91390000 + (30000000 * update_len) as Weight)
				.saturating_add(DbWeight::get().reads(3 as Weight))
				.saturating_add(DbWeight::get().writes(3 as Weight))
		}
	}
}
