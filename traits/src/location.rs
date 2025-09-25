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

#[cfg(test)]
mod tests {
	use super::*;

	const PARACHAIN: Junction = Parachain(1);
	const GENERAL_INDEX: Junction = GeneralIndex(1);

	fn concrete_fungible(id: Location) -> Asset {
		(id, 1).into()
	}

	#[test]
	fn parent_as_reserve_chain() {
		assert_eq!(
			AbsoluteReserveProvider::reserve(&concrete_fungible(Location::new(1, [GENERAL_INDEX]))),
			Some(Location::parent())
		);
		assert_eq!(
			RelativeReserveProvider::reserve(&concrete_fungible(Location::new(1, [GENERAL_INDEX]))),
			Some(Location::parent())
		);
	}

	#[test]
	fn sibling_parachain_as_reserve_chain() {
		assert_eq!(
			AbsoluteReserveProvider::reserve(&concrete_fungible(Location::new(1, [PARACHAIN, GENERAL_INDEX]))),
			Some(Location::new(1, [PARACHAIN]))
		);
		assert_eq!(
			RelativeReserveProvider::reserve(&concrete_fungible(Location::new(1, [PARACHAIN, GENERAL_INDEX]))),
			Some(Location::new(1, [PARACHAIN]))
		);
	}

	#[test]
	fn child_parachain_as_reserve_chain() {
		assert_eq!(
			AbsoluteReserveProvider::reserve(&concrete_fungible(Location::new(0, [PARACHAIN, GENERAL_INDEX]))),
			Some(PARACHAIN.into())
		);
		assert_eq!(
			RelativeReserveProvider::reserve(&concrete_fungible(Location::new(0, [PARACHAIN, GENERAL_INDEX]))),
			Some(PARACHAIN.into())
		);
	}

	#[test]
	fn no_reserve_chain_for_absolute_self_for_relative() {
		assert_eq!(
			AbsoluteReserveProvider::reserve(&concrete_fungible(Location::new(
				0,
				[Junction::from(BoundedVec::try_from(b"DOT".to_vec()).unwrap())]
			))),
			None
		);
		assert_eq!(
			RelativeReserveProvider::reserve(&concrete_fungible(Location::new(
				0,
				[Junction::from(BoundedVec::try_from(b"DOT".to_vec()).unwrap())]
			))),
			Some(Location::here())
		);
	}

	#[test]
	fn non_chain_part_works() {
		assert_eq!(Location::parent().non_chain_part(), None);
		assert_eq!(Location::new(1, [PARACHAIN]).non_chain_part(), None);
		assert_eq!(Location::new(0, [PARACHAIN]).non_chain_part(), None);

		assert_eq!(
			Location::new(1, [GENERAL_INDEX]).non_chain_part(),
			Some(GENERAL_INDEX.into())
		);
		assert_eq!(
			Location::new(1, [GENERAL_INDEX, GENERAL_INDEX]).non_chain_part(),
			Some((GENERAL_INDEX, GENERAL_INDEX).into())
		);
		assert_eq!(
			Location::new(1, [PARACHAIN, GENERAL_INDEX]).non_chain_part(),
			Some(GENERAL_INDEX.into())
		);
		assert_eq!(
			Location::new(0, [PARACHAIN, GENERAL_INDEX]).non_chain_part(),
			Some(GENERAL_INDEX.into())
		);
	}
}
