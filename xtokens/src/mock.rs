#![cfg(test)]

use super::*;
use crate as orml_xtokens;

use frame_support::parameter_types;
use orml_traits::parameter_type_with_key;
use orml_xcm_support::{IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset, XcmHandler as XcmHandlerT};
use polkadot_parachain::primitives::Sibling;
use serde::{Deserialize, Serialize};
use sp_io::TestExternalities;
use sp_runtime::AccountId32;
use xcm::v0::{Junction, MultiLocation::*, NetworkId};
use xcm_builder::{
	AccountId32Aliases, LocationInverter, ParentIsDefault, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SovereignSignedViaLocation,
};
use xcm_executor::Config as XcmConfigT;
use xcm_simulator::{decl_test_network, decl_test_parachain, prelude::*};

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	/// Relay chain token.
	R,
	/// Parachain A token.
	A,
	/// Parachain B token.
	B,
}

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		match id {
			CurrencyId::R => Some(Junction::Parent.into()),
			CurrencyId::A => Some(
				(
					Junction::Parent,
					Junction::Parachain { id: 1 },
					Junction::GeneralKey("A".into()),
				)
					.into(),
			),
			CurrencyId::B => Some(
				(
					Junction::Parent,
					Junction::Parachain { id: 2 },
					Junction::GeneralKey("B".into()),
				)
					.into(),
			),
		}
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(l: MultiLocation) -> Option<CurrencyId> {
		let a: Vec<u8> = "A".into();
		let b: Vec<u8> = "B".into();
		match l {
			X1(Parent) => Some(CurrencyId::R),
			X3(Junction::Parent, Junction::Parachain { id: 1 }, Junction::GeneralKey(k)) if k == a => {
				Some(CurrencyId::A)
			}
			X3(Junction::Parent, Junction::Parachain { id: 2 }, Junction::GeneralKey(k)) if k == b => {
				Some(CurrencyId::B)
			}
			_ => None,
		}
	}
}
impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(a: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset::ConcreteFungible { id, amount: _ } = a {
			Self::convert(id)
		} else {
			None
		}
	}
}

pub type Balance = u128;
pub type Amount = i128;

decl_test_parachain! {
	pub struct ParaA {
		new_ext = parachain_ext::<para_a::Runtime>(1),
		para_id = 1,
	}
	pub mod para_a {
		test_network = super::TestNetwork,
		xcm_config = {
			use super::*;

			parameter_types! {
				pub ParaANetwork: NetworkId = NetworkId::Any;
				pub RelayChainOrigin: Origin = cumulus_pallet_xcm_handler::Origin::Relay.into();
				pub Ancestry: MultiLocation = MultiLocation::X1(Junction::Parachain {
					id: ParachainInfo::get().into(),
				});
				pub const RelayChainCurrencyId: CurrencyId = CurrencyId::R;
			}

			pub type LocationConverter = (
				ParentIsDefault<AccountId>,
				SiblingParachainConvertsVia<Sibling, AccountId>,
				AccountId32Aliases<ParaANetwork, AccountId>,
			);

			pub type LocalAssetTransactor = MultiCurrencyAdapter<
				Tokens,
				(),
				IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
				AccountId,
				LocationConverter,
				CurrencyId,
				CurrencyIdConvert,
			>;

			pub type LocalOriginConverter = (
				SovereignSignedViaLocation<LocationConverter, Origin>,
				RelayChainAsNative<RelayChainOrigin, Origin>,
				SiblingParachainAsNative<cumulus_pallet_xcm_handler::Origin, Origin>,
				SignedAccountId32AsNative<ParaANetwork, Origin>,
			);

			pub struct XcmConfig;
			impl XcmConfigT for XcmConfig {
				type Call = Call;
				type XcmSender = XcmHandler;
				type AssetTransactor = LocalAssetTransactor;
				type OriginConverter = LocalOriginConverter;
				type IsReserve = MultiNativeAsset;
				type IsTeleporter = ();
				type LocationInverter = LocationInverter<Ancestry>;
			}
		},
		extra_config = {
			parameter_type_with_key! {
				pub ExistentialDeposits: |_currency_id: super::CurrencyId| -> Balance {
					Default::default()
				};
			}

			impl orml_tokens::Config for Runtime {
				type Event = Event;
				type Balance = Balance;
				type Amount = Amount;
				type CurrencyId = super::CurrencyId;
				type WeightInfo = ();
				type ExistentialDeposits = ExistentialDeposits;
				type OnDust = ();
			}

			pub struct HandleXcm;
			impl XcmHandlerT<AccountId> for HandleXcm {
				fn execute_xcm(origin: AccountId, xcm: Xcm) -> DispatchResult {
					XcmHandler::execute_xcm(origin, xcm)
				}
			}

			pub struct AccountId32Convert;
			impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
				fn convert(account_id: AccountId) -> [u8; 32] {
					account_id.into()
				}
			}

			parameter_types! {
				pub SelfLocation: MultiLocation = (Junction::Parent, Junction::Parachain { id: ParachainInfo::get().into() }).into();
			}

			impl orml_xtokens::Config for Runtime {
				type Event = Event;
				type Balance = Balance;
				type CurrencyId = CurrencyId;
				type CurrencyIdConvert = CurrencyIdConvert;
				type AccountId32Convert = AccountId32Convert;
				type SelfLocation = SelfLocation;
				type XcmHandler = HandleXcm;
			}
		},
		extra_modules = {
			Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
			XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>},
		},
	}
}

decl_test_parachain! {
	pub struct ParaB {
		new_ext = parachain_ext::<para_b::Runtime>(2),
		para_id = 2,
	}
	pub mod para_c {
		test_network = super::TestNetwork,
		xcm_config = {
			use super::*;

			parameter_types! {
				pub ParaANetwork: NetworkId = NetworkId::Any;
				pub RelayChainOrigin: Origin = cumulus_pallet_xcm_handler::Origin::Relay.into();
				pub Ancestry: MultiLocation = MultiLocation::X1(Junction::Parachain {
					id: ParachainInfo::get().into(),
				});
				pub const RelayChainCurrencyId: CurrencyId = CurrencyId::R;
			}

			pub type LocationConverter = (
				ParentIsDefault<AccountId>,
				SiblingParachainConvertsVia<Sibling, AccountId>,
				AccountId32Aliases<ParaANetwork, AccountId>,
			);

			pub type LocalAssetTransactor = MultiCurrencyAdapter<
				Tokens,
				(),
				IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
				AccountId,
				LocationConverter,
				CurrencyId,
				CurrencyIdConvert,
			>;

			pub type LocalOriginConverter = (
				SovereignSignedViaLocation<LocationConverter, Origin>,
				RelayChainAsNative<RelayChainOrigin, Origin>,
				SiblingParachainAsNative<cumulus_pallet_xcm_handler::Origin, Origin>,
				SignedAccountId32AsNative<ParaANetwork, Origin>,
			);

			pub struct XcmConfig;
			impl XcmConfigT for XcmConfig {
				type Call = Call;
				type XcmSender = XcmHandler;
				type AssetTransactor = LocalAssetTransactor;
				type OriginConverter = LocalOriginConverter;
				type IsReserve = MultiNativeAsset;
				type IsTeleporter = ();
				type LocationInverter = LocationInverter<Ancestry>;
			}
		},
		extra_config = {
			parameter_type_with_key! {
				pub ExistentialDeposits: |_currency_id: super::CurrencyId| -> Balance {
					Default::default()
				};
			}

			impl orml_tokens::Config for Runtime {
				type Event = Event;
				type Balance = Balance;
				type Amount = Amount;
				type CurrencyId = super::CurrencyId;
				type WeightInfo = ();
				type ExistentialDeposits = ExistentialDeposits;
				type OnDust = ();
			}

			pub struct HandleXcm;
			impl XcmHandlerT<AccountId> for HandleXcm {
				fn execute_xcm(origin: AccountId, xcm: Xcm) -> DispatchResult {
					XcmHandler::execute_xcm(origin, xcm)
				}
			}

			pub struct AccountId32Convert;
			impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
				fn convert(account_id: AccountId) -> [u8; 32] {
					account_id.into()
				}
			}

			parameter_types! {
				pub SelfLocation: MultiLocation = (Junction::Parent, Junction::Parachain { id: ParachainInfo::get().into() }).into();
			}

			impl orml_xtokens::Config for Runtime {
				type Event = Event;
				type Balance = Balance;
				type CurrencyId = CurrencyId;
				type CurrencyIdConvert = CurrencyIdConvert;
				type AccountId32Convert = AccountId32Convert;
				type SelfLocation = SelfLocation;
				type XcmHandler = HandleXcm;
			}
		},
		extra_modules = {
			Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
			XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>},
		},
	}
}

decl_test_parachain! {
	pub struct ParaC {
		new_ext = parachain_ext::<para_b::Runtime>(3),
		para_id = 3,
	}
	pub mod para_b {
		test_network = super::TestNetwork,
		xcm_config = {
			use super::*;

			parameter_types! {
				pub ParaANetwork: NetworkId = NetworkId::Any;
				pub RelayChainOrigin: Origin = cumulus_pallet_xcm_handler::Origin::Relay.into();
				pub Ancestry: MultiLocation = MultiLocation::X1(Junction::Parachain {
					id: ParachainInfo::get().into(),
				});
				pub const RelayChainCurrencyId: CurrencyId = CurrencyId::R;
			}

			pub type LocationConverter = (
				ParentIsDefault<AccountId>,
				SiblingParachainConvertsVia<Sibling, AccountId>,
				AccountId32Aliases<ParaANetwork, AccountId>,
			);

			pub type LocalAssetTransactor = MultiCurrencyAdapter<
				Tokens,
				(),
				IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
				AccountId,
				LocationConverter,
				CurrencyId,
				CurrencyIdConvert,
			>;

			pub type LocalOriginConverter = (
				SovereignSignedViaLocation<LocationConverter, Origin>,
				RelayChainAsNative<RelayChainOrigin, Origin>,
				SiblingParachainAsNative<cumulus_pallet_xcm_handler::Origin, Origin>,
				SignedAccountId32AsNative<ParaANetwork, Origin>,
			);

			pub struct XcmConfig;
			impl XcmConfigT for XcmConfig {
				type Call = Call;
				type XcmSender = XcmHandler;
				type AssetTransactor = LocalAssetTransactor;
				type OriginConverter = LocalOriginConverter;
				type IsReserve = MultiNativeAsset;
				type IsTeleporter = ();
				type LocationInverter = LocationInverter<Ancestry>;
			}
		},
		extra_config = {
			parameter_type_with_key! {
				pub ExistentialDeposits: |_currency_id: super::CurrencyId| -> Balance {
					Default::default()
				};
			}

			impl orml_tokens::Config for Runtime {
				type Event = Event;
				type Balance = Balance;
				type Amount = Amount;
				type CurrencyId = super::CurrencyId;
				type WeightInfo = ();
				type ExistentialDeposits = ExistentialDeposits;
				type OnDust = ();
			}

			pub struct HandleXcm;
			impl XcmHandlerT<AccountId> for HandleXcm {
				fn execute_xcm(origin: AccountId, xcm: Xcm) -> DispatchResult {
					XcmHandler::execute_xcm(origin, xcm)
				}
			}

			pub struct AccountId32Convert;
			impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
				fn convert(account_id: AccountId) -> [u8; 32] {
					account_id.into()
				}
			}

			parameter_types! {
				pub SelfLocation: MultiLocation = (Junction::Parent, Junction::Parachain { id: ParachainInfo::get().into() }).into();
			}

			impl orml_xtokens::Config for Runtime {
				type Event = Event;
				type Balance = Balance;
				type CurrencyId = CurrencyId;
				type CurrencyIdConvert = CurrencyIdConvert;
				type AccountId32Convert = AccountId32Convert;
				type SelfLocation = SelfLocation;
				type XcmHandler = HandleXcm;
			}
		},
		extra_modules = {
			Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
			XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>},
		},
	}
}

decl_test_network! {
	pub struct TestNetwork {
		relay_chain = default,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
			(3, ParaC),
		],
	}
}

pub type ParaAXtokens = orml_xtokens::Pallet<para_a::Runtime>;
pub type ParaATokens = orml_tokens::Pallet<para_a::Runtime>;
pub type ParaBTokens = orml_tokens::Pallet<para_b::Runtime>;
pub type ParaCTokens = orml_tokens::Pallet<para_c::Runtime>;

pub type RelayBalances = pallet_balances::Pallet<relay::Runtime>;

pub struct ParaExtBuilder;

impl Default for ParaExtBuilder {
	fn default() -> Self {
		ParaExtBuilder
	}
}

impl ParaExtBuilder {
	pub fn build<
		Runtime: frame_system::Config<AccountId = AccountId32> + orml_tokens::Config<CurrencyId = CurrencyId, Balance = Balance>,
	>(
		self,
		para_id: u32,
	) -> TestExternalities
	where
		<Runtime as frame_system::Config>::BlockNumber: From<u64>,
	{
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		parachain_info::GenesisConfig {
			parachain_id: para_id.into(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: vec![(ALICE, CurrencyId::R, 100)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = TestExternalities::new(t);
		ext.execute_with(|| frame_system::Pallet::<Runtime>::set_block_number(1.into()));
		ext
	}
}

pub fn parachain_ext<
	Runtime: frame_system::Config<AccountId = AccountId32> + orml_tokens::Config<CurrencyId = CurrencyId, Balance = Balance>,
>(
	para_id: u32,
) -> TestExternalities
where
	<Runtime as frame_system::Config>::BlockNumber: From<u64>,
{
	ParaExtBuilder::default().build::<Runtime>(para_id)
}
