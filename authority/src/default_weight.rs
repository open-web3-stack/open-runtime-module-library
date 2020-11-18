//! Weights for orml_authority
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-11-18, STEPS: [1, ], REPEAT: 1, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB
//! CACHE: 128

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};
use sp_std::marker::PhantomData;

/// Weight functions for orml_authority.
impl crate::WeightInfo for () {
	fn dispatch_as() -> Weight {
		(48_680_000 as Weight)
	}
	fn schedule_dispatch_without_delay() -> Weight {
		(121_510_000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn schedule_dispatch_with_delay() -> Weight {
		(130_696_000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn fast_track_scheduled_dispatch() -> Weight {
		(145_530_000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn delay_scheduled_dispatch() -> Weight {
		(145_169_000 as Weight)
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn cancel_scheduled_dispatch() -> Weight {
		(104_990_000 as Weight)
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
}
