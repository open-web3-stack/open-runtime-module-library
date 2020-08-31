//! Weights for the Tokens Module

use frame_support::weights::{
	constants::{RocksDbWeight as DbWeight, WEIGHT_PER_MICROS},
	Weight,
};

impl crate::WeightInfo for () {
	fn transfer() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(84)
			.saturating_add(DbWeight::get().reads_writes(4, 2))
	}

	fn transfer_all() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(88)
			.saturating_add(DbWeight::get().reads_writes(4, 2))
	}
}
