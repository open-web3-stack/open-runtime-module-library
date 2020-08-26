//! Weights for the Auction Module

use frame_support::weights::{
	constants::{RocksDbWeight as DbWeight, WEIGHT_PER_MICROS},
	Weight,
};

impl crate::WeightInfo for () {
	fn bid() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(300)
			.saturating_add(DbWeight::get().reads_writes(14, 14))
	}

	fn on_finalize(_a: u32) -> Weight {
		0
	}
}
