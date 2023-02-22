use sp_core::{bounded::BoundedVec, ConstU32};
use xcm::latest::prelude::*;

use crate::location::RelativeLocations;

pub trait ConcreteFungibleAsset {
	fn sibling_parachain_asset(
		para_id: u32,
		general_key: BoundedVec<u8, ConstU32<32>>,
		amount: u128,
	) -> Option<MultiAsset>;
	fn parent_asset(amount: u128) -> MultiAsset;
}

impl ConcreteFungibleAsset for MultiAsset {
	fn sibling_parachain_asset(
		para_id: u32,
		general_key: BoundedVec<u8, ConstU32<32>>,
		amount: u128,
	) -> Option<MultiAsset> {
		if let Some(general_key) = MultiLocation::sibling_parachain_general_key(para_id, general_key) {
			return Some((general_key, amount).into());
		}
		None
	}

	fn parent_asset(amount: u128) -> MultiAsset {
		(MultiLocation::parent(), amount).into()
	}
}
