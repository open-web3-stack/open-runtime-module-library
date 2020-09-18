//! Weights for the Rewards Module

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn on_initialize() -> Weight {
		(20100000 as Weight).saturating_add(DbWeight::get().reads(1 as Weight))
	}
}
