use sp_core::{bounded::BoundedVec, ConstU32};
use xcm::v5::prelude::*;

pub const ASSET_HUB_ID: u32 = 1000;

pub trait Reserve {
	/// Returns assets reserve location.
	fn reserve(asset: &Asset) -> Option<Location>;
}

pub trait RelativeLocations {
	fn sibling_parachain_general_key(para_id: u32, general_key: BoundedVec<u8, ConstU32<32>>) -> Location;
}

impl RelativeLocations for Location {
	fn sibling_parachain_general_key(para_id: u32, general_key: BoundedVec<u8, ConstU32<32>>) -> Location {
		Location::new(1, [Parachain(para_id), general_key.as_bounded_slice().into()])
	}
}
