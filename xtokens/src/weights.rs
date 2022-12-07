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
	fn set_minimum_execution_fee() -> Weight;
}

/// Default weights.
impl WeightInfo for () {
	fn set_minimum_execution_fee() -> Weight {
        // ref_time guesstimated by the tokens::set_balance ref_time 
            Weight::from_ref_time(34_000_000)
                .saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
}
