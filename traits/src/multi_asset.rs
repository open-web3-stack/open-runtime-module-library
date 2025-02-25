use sp_core::{bounded::BoundedVec, ConstU32};
use xcm::v5::prelude::*;

use crate::location::RelativeLocations;

pub trait ConcreteFungibleAsset {
	fn sibling_parachain_asset(para_id: u32, general_key: BoundedVec<u8, ConstU32<32>>, amount: u128) -> Asset;
	fn parent_asset(amount: u128) -> Asset;
}

impl ConcreteFungibleAsset for Asset {
	fn sibling_parachain_asset(para_id: u32, general_key: BoundedVec<u8, ConstU32<32>>, amount: u128) -> Asset {
		(Location::sibling_parachain_general_key(para_id, general_key), amount).into()
	}

	fn parent_asset(amount: u128) -> Asset {
		(Location::parent(), amount).into()
	}
}
