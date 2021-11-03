use sp_std::prelude::*;
use xcm::latest::prelude::*;

pub trait Parse {
	/// Returns the "chain" location part. It could be parent, sibling
	/// parachain, or child parachain.
	fn chain_part(&self) -> Option<MultiLocation>;
	/// Returns "non-chain" location part.
	fn non_chain_part(&self) -> Option<MultiLocation>;
}

fn is_chain_junction(junction: Option<&Junction>) -> bool {
	matches!(junction, Some(Parachain(_)))
}

impl Parse for MultiLocation {
	fn chain_part(&self) -> Option<MultiLocation> {
		match (self.parents, self.first_interior()) {
			// sibling parachain
			(1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
			// parent
			(1, _) => Some(MultiLocation::parent()),
			// children parachain
			(0, Some(Parachain(id))) => Some(MultiLocation::new(0, X1(Parachain(*id)))),
			_ => None,
		}
	}

	fn non_chain_part(&self) -> Option<MultiLocation> {
		let mut junctions = self.interior().clone();
		while is_chain_junction(junctions.first()) {
			let _ = junctions.take_first();
		}

		if junctions != Here {
			Some(MultiLocation::new(0, junctions))
		} else {
			None
		}
	}
}

pub trait Reserve {
	/// Returns assets reserve location.
	fn reserve(&self) -> Option<MultiLocation>;
}

impl Reserve for MultiAsset {
	fn reserve(&self) -> Option<MultiLocation> {
		if let Concrete(location) = &self.id {
			location.chain_part()
		} else {
			None
		}
	}
}

pub trait RelativeLocations {
	fn sibling_parachain_general_key(para_id: u32, general_key: Vec<u8>) -> MultiLocation;
}

impl RelativeLocations for MultiLocation {
	fn sibling_parachain_general_key(para_id: u32, general_key: Vec<u8>) -> MultiLocation {
		MultiLocation::new(1, X2(Parachain(para_id), GeneralKey(general_key)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const PARACHAIN: Junction = Parachain(1);
	const GENERAL_INDEX: Junction = GeneralIndex(1);

	fn concrete_fungible(id: MultiLocation) -> MultiAsset {
		(id, 1).into()
	}

	#[test]
	fn parent_as_reserve_chain() {
		assert_eq!(
			concrete_fungible(MultiLocation::new(1, X1(GENERAL_INDEX))).reserve(),
			Some(MultiLocation::parent())
		);
	}

	#[test]
	fn sibling_parachain_as_reserve_chain() {
		assert_eq!(
			concrete_fungible(MultiLocation::new(1, X2(PARACHAIN, GENERAL_INDEX))).reserve(),
			Some(MultiLocation::new(1, X1(PARACHAIN)))
		);
	}

	#[test]
	fn child_parachain_as_reserve_chain() {
		assert_eq!(
			concrete_fungible(MultiLocation::new(0, X2(PARACHAIN, GENERAL_INDEX))).reserve(),
			Some(PARACHAIN.into())
		);
	}

	#[test]
	fn no_reserve_chain() {
		assert_eq!(
			concrete_fungible(MultiLocation::new(0, X1(GeneralKey("DOT".into())))).reserve(),
			None
		);
	}

	#[test]
	fn non_chain_part_works() {
		assert_eq!(MultiLocation::parent().non_chain_part(), None);
		assert_eq!(MultiLocation::new(1, X1(PARACHAIN)).non_chain_part(), None);
		assert_eq!(MultiLocation::new(0, X1(PARACHAIN)).non_chain_part(), None);

		assert_eq!(
			MultiLocation::new(1, X1(GENERAL_INDEX)).non_chain_part(),
			Some(GENERAL_INDEX.into())
		);
		assert_eq!(
			MultiLocation::new(1, X2(GENERAL_INDEX, GENERAL_INDEX)).non_chain_part(),
			Some((GENERAL_INDEX, GENERAL_INDEX).into())
		);
		assert_eq!(
			MultiLocation::new(1, X2(PARACHAIN, GENERAL_INDEX)).non_chain_part(),
			Some(GENERAL_INDEX.into())
		);
		assert_eq!(
			MultiLocation::new(0, X2(PARACHAIN, GENERAL_INDEX)).non_chain_part(),
			Some(GENERAL_INDEX.into())
		);
	}
}
