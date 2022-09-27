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
	fn set_asset_location() -> Weight;
}

/// Default weights.
impl WeightInfo for () {
	fn register_asset() -> Weight {
		Weight::zero()
	}
	fn update_asset() -> Weight {
		Weight::zero()
	}
	fn set_asset_location() -> Weight {
		Weight::zero()
	}
}
