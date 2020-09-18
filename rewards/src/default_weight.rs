//! Weights for the Rewards Module

use frame_support::weights::Weight;

impl crate::WeightInfo for () {
	fn on_initialize() -> Weight {
		(20100000 as Weight).saturating_add(DbWeight::get().reads(1 as Weight))
	}
}
