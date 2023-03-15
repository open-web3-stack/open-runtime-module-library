use crate::{Config, LocationToAssetId, Metadata, Pallet, Weight};
use frame_support::pallet_prelude::*;
use frame_support::{migration::storage_key_iter, traits::OnRuntimeUpgrade, StoragePrefixedMap};

use xcm::v3::prelude::*;

pub mod v0 {
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::traits::ConstU32;
	use frame_support::WeakBoundedVec;
	use scale_info::TypeInfo;

	// these imports are unchanged from v0, see:
	// v2 reimport from v1: https://github.com/paritytech/polkadot/blob/645723987cf9662244be8faf4e9b63e8b9a1b3a3/xcm/src/v2/mod.rs#L65-L69
	// v1 reimport from v0: https://github.com/paritytech/polkadot/blob/645723987cf9662244be8faf4e9b63e8b9a1b3a3/xcm/src/v1/mod.rs#L92
	use xcm::v2::{BodyId, BodyPart, NetworkId};

	// copied from https://github.com/paritytech/polkadot/blob/645723987cf9662244be8faf4e9b63e8b9a1b3a3/xcm/src/v0/multi_location.rs#L46-L65
	#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug, scale_info::TypeInfo)]
	pub enum MultiLocation {
		/// The interpreting consensus system.
		Null,
		/// A relative path comprising 1 junction.
		X1(Junction),
		/// A relative path comprising 2 junctions.
		X2(Junction, Junction),
		/// A relative path comprising 3 junctions.
		X3(Junction, Junction, Junction),
		/// A relative path comprising 4 junctions.
		X4(Junction, Junction, Junction, Junction),
		/// A relative path comprising 5 junctions.
		X5(Junction, Junction, Junction, Junction, Junction),
		/// A relative path comprising 6 junctions.
		X6(Junction, Junction, Junction, Junction, Junction, Junction),
		/// A relative path comprising 7 junctions.
		X7(Junction, Junction, Junction, Junction, Junction, Junction, Junction),
		/// A relative path comprising 8 junctions.
		X8(
			Junction,
			Junction,
			Junction,
			Junction,
			Junction,
			Junction,
			Junction,
			Junction,
		),
	}

	// copied from https://github.com/paritytech/polkadot/blob/645723987cf9662244be8faf4e9b63e8b9a1b3a3/xcm/src/v0/junction.rs#L115-L169
	#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug, TypeInfo, MaxEncodedLen)]
	pub enum Junction {
		/// The consensus system of which the context is a member and state-wise
		/// super-set.
		///
		/// NOTE: This item is *not* a sub-consensus item: a consensus system
		/// may not identify itself trustlessly as a location that includes this
		/// junction.
		Parent,
		/// An indexed parachain belonging to and operated by the context.
		///
		/// Generally used when the context is a Polkadot Relay-chain.
		Parachain(#[codec(compact)] u32),
		/// A 32-byte identifier for an account of a specific network that is
		/// respected as a sovereign endpoint within the context.
		///
		/// Generally used when the context is a Substrate-based chain.
		AccountId32 { network: NetworkId, id: [u8; 32] },
		/// An 8-byte index for an account of a specific network that is
		/// respected as a sovereign endpoint within the context.
		///
		/// May be used when the context is a Frame-based chain and includes
		/// e.g. an indices pallet.
		AccountIndex64 {
			network: NetworkId,
			#[codec(compact)]
			index: u64,
		},
		/// A 20-byte identifier for an account of a specific network that is
		/// respected as a sovereign endpoint within the context.
		///
		/// May be used when the context is an Ethereum or Bitcoin chain or
		/// smart-contract.
		AccountKey20 { network: NetworkId, key: [u8; 20] },
		/// An instanced, indexed pallet that forms a constituent part of the
		/// context.
		///
		/// Generally used when the context is a Frame-based chain.
		PalletInstance(u8),
		/// A non-descript index within the context location.
		///
		/// Usage will vary widely owing to its generality.
		///
		/// NOTE: Try to avoid using this and instead use a more specific item.
		GeneralIndex(#[codec(compact)] u128),
		/// A nondescript datum acting as a key within the context location.
		///
		/// Usage will vary widely owing to its generality.
		///
		/// NOTE: Try to avoid using this and instead use a more specific item.
		GeneralKey(WeakBoundedVec<u8, ConstU32<32>>),
		/// The unambiguous child.
		///
		/// Not currently used except as a fallback when deriving ancestry.
		OnlyChild,
		/// A pluralistic body existing within consensus.
		///
		/// Typical to be used to represent a governance origin of a chain, but
		/// could in principle be used to represent things such as multisigs
		/// also.
		Plurality { id: BodyId, part: BodyPart },
	}
}

pub mod pre_polkadot_0_9_38 {
	use codec::{Decode, Encode};
	use frame_support::{pallet_prelude::Member, Parameter, RuntimeDebug};
	use scale_info::TypeInfo;
	use sp_std::vec::Vec;

	#[derive(scale_info::TypeInfo, Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
	pub enum VersionedMultiLocation {
		V0(super::v0::MultiLocation),
		V1(xcm::v2::MultiLocation), /* v2::multilocation is identical to v1::multilocation. See https://github.com/paritytech/polkadot/blob/645723987cf9662244be8faf4e9b63e8b9a1b3a3/xcm/src/v2/mod.rs#L68 */
	}

	#[derive(scale_info::TypeInfo, Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
	pub struct AssetMetadata<Balance, CustomMetadata: Parameter + Member + TypeInfo> {
		pub decimals: u32,
		pub name: Vec<u8>,
		pub symbol: Vec<u8>,
		pub existential_deposit: Balance,
		pub location: Option<VersionedMultiLocation>,
		pub additional: CustomMetadata,
	}
}

/// Migration that only works if Metadata.location is None or
/// Some(VersionedMultiLocation::V1(_)). Any metadata items that are
/// Some(VersionedMultiLocation::V0(_)) will be set to None.
#[allow(non_camel_case_types)]
pub struct MigrateV1Only_V0NotSupported<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrateV1Only_V0NotSupported<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut weight: Weight = Weight::zero();
		let onchain_version = Pallet::<T>::on_chain_storage_version();
		if onchain_version < 2 {
			let inner_weight = v2::migrate::<T>();
			weight.saturating_accrue(inner_weight);
		}
		weight
	}
}

mod v2 {
	use super::*;
	use frame_support::log;
	use orml_traits::asset_registry::AssetMetadata;
	use xcm::VersionedMultiLocation;
	pub(crate) fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();
		let module_prefix = LocationToAssetId::<T>::module_prefix();
		let storage_prefix = LocationToAssetId::<T>::storage_prefix();

		weight.saturating_accrue(T::DbWeight::get().reads(1));
		let old_data =
			storage_key_iter::<xcm::v2::MultiLocation, T::AssetId, Twox64Concat>(module_prefix, storage_prefix)
				.drain()
				.collect::<sp_std::vec::Vec<_>>();

		for (old_key, value) in old_data {
			weight.saturating_accrue(T::DbWeight::get().writes(1));
			let new_key: MultiLocation = old_key.try_into().expect("Stored xcm::v2::MultiLocation");
			LocationToAssetId::<T>::insert(new_key, value);
		}

		Metadata::<T>::translate(
			|_, old: pre_polkadot_0_9_38::AssetMetadata<T::Balance, T::CustomMetadata>| {
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));

				let new_location = match old.location {
					Some(pre_polkadot_0_9_38::VersionedMultiLocation::V1(x)) => {
						// v2::multilocation is identical to v1::multilocation. See https://github.com/paritytech/polkadot/blob/645723987cf9662244be8faf4e9b63e8b9a1b3a3/xcm/src/v2/mod.rs#L68
						Some(VersionedMultiLocation::V2(x))
					}
					Some(pre_polkadot_0_9_38::VersionedMultiLocation::V0(_)) => {
						log::error!("Migration for an item in metadata.location failed because V0 is not supported");
						None
					}
					None => None,
				};

				let new_metadata = AssetMetadata {
					additional: old.additional,
					decimals: old.decimals,
					existential_deposit: old.existential_deposit,
					name: old.name,
					symbol: old.symbol,
					location: new_location,
				};

				Some(new_metadata)
			},
		);

		StorageVersion::new(2).put::<Pallet<T>>();
		weight.saturating_accrue(T::DbWeight::get().writes(1));
		weight
	}
}
