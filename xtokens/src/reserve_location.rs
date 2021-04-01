use xcm::v0::{Junction, MultiAsset, MultiLocation};

pub trait ReserveLocation {
	fn reserve(&self) -> Option<MultiLocation>;
}

impl ReserveLocation for MultiAsset {
	fn reserve(&self) -> Option<MultiLocation> {
		if let MultiAsset::ConcreteFungible { id, .. } = self {
			match (id.first(), id.at(1)) {
				(Some(Junction::Parent), Some(Junction::Parachain { id: para_id })) => {
					Some((Junction::Parent, Junction::Parachain { id: *para_id }).into())
				}
				(Some(Junction::Parent), _) => Some(Junction::Parent.into()),
				_ => None,
			}
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn concrete_fungible(id: MultiLocation) -> MultiAsset {
		MultiAsset::ConcreteFungible { id, amount: 1 }
	}

	#[test]
	fn parent_as_reserve_chain() {
		assert_eq!(
			concrete_fungible(MultiLocation::X2(Junction::Parent, Junction::GeneralIndex { id: 1 })).reserve(),
			Some(Junction::Parent.into())
		);
	}

	#[test]
	fn sibling_parachain_as_reserve_chain() {
		assert_eq!(
			concrete_fungible(MultiLocation::X3(
				Junction::Parent,
				Junction::Parachain { id: 1 },
				Junction::GeneralIndex { id: 1 }
			))
			.reserve(),
			Some((Junction::Parent, Junction::Parachain { id: 1 }).into())
		);
	}

	#[test]
	fn no_reserve_chain() {
		assert_eq!(
			concrete_fungible(MultiLocation::X1(Junction::GeneralKey("DOT".into()))).reserve(),
			None
		);
	}
}
