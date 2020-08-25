//! Weights for the Vesting Module

use frame_support::weights::{
	constants::{RocksDbWeight as DbWeight, WEIGHT_PER_MICROS},
	Weight,
};

impl crate::WeightInfo for () {
	fn claim() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(30)
			.saturating_add(DbWeight::get().reads_writes(4, 4))
	}

	fn vested_transfer() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(150)
			.saturating_add(DbWeight::get().reads_writes(3, 3))
	}

	fn update_vesting_schedules() -> Weight {
		WEIGHT_PER_MICROS
			.saturating_mul(62)
			.saturating_add(DbWeight::get().reads_writes(2, 3))
	}
}
