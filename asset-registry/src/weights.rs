#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

pub trait WeightInfo {
	fn register_asset() -> Weight;
	fn update_asset() -> Weight;
	// fn set_asset_location() -> Weight;
}

/// Default weights.
impl WeightInfo for () {
	// Storage: Tokens NextCurrencyId (r:1 w:1)
	// Storage: Tokens Accounts (r:1 w:0)
	// Storage: AssetRegistry Metadata (r:1 w:1)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:1)
	fn register_asset() -> Weight {
		Weight::from_parts(34_624_000, 0)
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	// Storage: AssetRegistry Metadata (r:1 w:1)
	// Storage: AssetRegistry LocationToAssetId (r:1 w:1)
	fn update_asset() -> Weight {
		Weight::from_parts(28_712_000, 0)
			.saturating_add(RocksDbWeight::get().reads(2 as u64))
			.saturating_add(RocksDbWeight::get().writes(2 as u64))
	}
	// fn set_asset_location() -> Weight {
	// 	Weight::from_ref_time(28_712_000)
	// 		.saturating_add(RocksDbWeight::get().reads(2 as Weight))
	// 		.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	// }
}
