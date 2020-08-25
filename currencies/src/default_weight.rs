//! Weights for the Currencies Module

use frame_support::weights::{
	constants::{RocksDbWeight as DbWeight, WEIGHT_PER_MICROS},
	Weight,
};

impl crate::WeightInfo for () {
	fn transfer() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(90)
			.saturating_add(DbWeight::get().reads_writes(5, 2))
	}

	fn transfer_native_currency() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(70)
			.saturating_add(DbWeight::get().reads_writes(2, 2))
	}

	fn update_balance() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(66)
			.saturating_add(DbWeight::get().reads_writes(5, 2))
	}
}
