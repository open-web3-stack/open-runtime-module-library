use sp_core::{bounded::BoundedVec, ConstU32};
use xcm::v4::prelude::*;

pub trait Parse {
	/// Returns the "chain" location part. It could be parent, sibling
	/// parachain, or child parachain.
	fn chain_part(&self) -> Option<Location>;
	/// Returns "non-chain" location part.
	fn non_chain_part(&self) -> Option<Location>;
}

fn is_chain_junction(junction: Option<&Junction>) -> bool {
	matches!(junction, Some(Parachain(_)))
}

impl Parse for Location {
	fn chain_part(&self) -> Option<Location> {
		match (self.parents, self.first_interior()) {
			// sibling parachain
			(1, Some(Parachain(id))) => Some(Location::new(1, [Parachain(*id)])),
			// parent
			(1, _) => Some(Location::parent()),
			// children parachain
			(0, Some(Parachain(id))) => Some(Location::new(0, [Parachain(*id)])),
			_ => None,
		}
	}

	fn non_chain_part(&self) -> Option<Location> {
		let mut junctions = self.interior().clone();
		while is_chain_junction(junctions.first()) {
			let _ = junctions.take_first();
		}

		if junctions != Here {
			Some(Location::new(0, junctions))
		} else {
			None
		}
	}
}

pub trait Reserve {
	/// Returns assets reserve location.
	fn reserve(asset: &Asset) -> Option<Location>;
}

// Provide reserve in absolute path view
pub struct AbsoluteReserveProvider;

impl Reserve for AbsoluteReserveProvider {
	fn reserve(asset: &Asset) -> Option<Location> {
		let AssetId(location) = &asset.id;
		location.chain_part()
	}
}

// Provide reserve in relative path view
// Self tokens are represeneted as Here
pub struct RelativeReserveProvider;

impl Reserve for RelativeReserveProvider {
	fn reserve(asset: &Asset) -> Option<Location> {
		let AssetId(location) = &asset.id;
		if location.parents == 0 && !is_chain_junction(location.first_interior()) {
			Some(Location::here())
		} else {
			location.chain_part()
		}
	}
}

pub trait RelativeLocations {
	fn sibling_parachain_general_key(para_id: u32, general_key: BoundedVec<u8, ConstU32<32>>) -> Location;
}

impl RelativeLocations for Location {
	fn sibling_parachain_general_key(para_id: u32, general_key: BoundedVec<u8, ConstU32<32>>) -> Location {
		return Location::new(1, [Parachain(para_id), general_key.as_bounded_slice().into()]);
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
