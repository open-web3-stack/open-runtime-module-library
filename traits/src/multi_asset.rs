use sp_std::prelude::*;
use xcm::latest::prelude::*;

use crate::location::RelativeLocations;

pub trait ConcreteFungibleAsset {
	fn sibling_parachain_asset(para_id: u32, general_key: Vec<u8>, amount: u128) -> MultiAsset;
	fn parent_asset(amount: u128) -> MultiAsset;
}

impl ConcreteFungibleAsset for MultiAsset {
	fn sibling_parachain_asset(para_id: u32, general_key: Vec<u8>, amount: u128) -> MultiAsset {
		(
			MultiLocation::sibling_parachain_general_key(para_id, general_key),
			amount,
		)
			.into()
	}

	fn parent_asset(amount: u128) -> MultiAsset {
		(MultiLocation::parent(), amount).into()
	}
}
