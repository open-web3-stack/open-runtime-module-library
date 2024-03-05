use super::{Amount, Balance, CurrencyId, CurrencyIdConvert, ParachainXcmRouter};
use crate as orml_xtokens;

use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU128, ConstU32, Contains, Everything, Get, Nothing},
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::{
	traits::{Convert, IdentityLookup},
	AccountId32, BoundedVec,
};
use xcm::v4::{prelude::*, Weight};
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds, ParentIsPreset,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor};

use crate::mock::AllTokensAreCreatedEqualToWeight;
use orml_traits::{
	location::{AbsoluteReserveProvider, RelativeReserveProvider},
	parameter_type_with_key,
};
use orml_xcm_support::{IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};

pub type AccountId = AccountId32;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = [u8; 8];
	type MaxFreezes = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = Everything;
}

parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get()), Parachain(MsgQueue::get().into())].into();
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToCallOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

pub type LocalAssetTransactor = MultiCurrencyAdapter<
	Tokens,
	(),
	IsNativeConcrete<CurrencyId, CurrencyIdConvert>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	CurrencyIdConvert,
	(),
>;

pub type XcmRouter = ParachainXcmRouter<MsgQueue>;
pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

parameter_types! {
	pub const UnitWeightCost: Weight = Weight::from_parts(10, 10);
	pub const BaseXcmWeight: Weight = Weight::from_parts(100_000_000, 100_000_000);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = AllTokensAreCreatedEqualToWeight;
	type ResponseHandler = ();
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type AssetLocker = PolkadotXcm;
	type AssetExchanger = ();
	type PalletInstancesInfo = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = ();
	type TransactionalProcessor = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

pub struct AccountIdToLocation;
impl Convert<AccountId, Location> for AccountIdToLocation {
	fn convert(account: AccountId) -> Location {
		[Junction::AccountId32 {
			network: None,
			id: account.into(),
		}]
		.into()
	}
}

pub struct RelativeCurrencyIdConvert;
impl Convert<CurrencyId, Option<Location>> for RelativeCurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<Location> {
		match id {
			CurrencyId::R => Some(Parent.into()),
			CurrencyId::A => Some(
				(
					Parent,
					Parachain(1),
					Junction::from(BoundedVec::try_from(b"A".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::A1 => Some(
				(
					Parent,
					Parachain(1),
					Junction::from(BoundedVec::try_from(b"A1".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::B => Some(
				(
					Parent,
					Parachain(2),
					Junction::from(BoundedVec::try_from(b"B".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::B1 => Some(
				(
					Parent,
					Parachain(2),
					Junction::from(BoundedVec::try_from(b"B1".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::B2 => Some(
				(
					Parent,
					Parachain(2),
					Junction::from(BoundedVec::try_from(b"B2".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::C => Some(
				(
					Parent,
					Parachain(3),
					Junction::from(BoundedVec::try_from(b"C".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::D => Some(Junction::from(BoundedVec::try_from(b"D".to_vec()).unwrap()).into()),
		}
	}
}
impl Convert<Location, Option<CurrencyId>> for RelativeCurrencyIdConvert {
	fn convert(l: Location) -> Option<CurrencyId> {
		let mut a: Vec<u8> = "A".into();
		a.resize(32, 0);
		let mut a1: Vec<u8> = "A1".into();
		a1.resize(32, 0);
		let mut b: Vec<u8> = "B".into();
		b.resize(32, 0);
		let mut b1: Vec<u8> = "B1".into();
		b1.resize(32, 0);
		let mut b2: Vec<u8> = "B2".into();
		b2.resize(32, 0);
		let mut c: Vec<u8> = "C".into();
		c.resize(32, 0);
		let mut d: Vec<u8> = "D".into();
		d.resize(32, 0);

		let self_para_id: u32 = MsgQueue::get().into();
		if l == Location::parent() {
			return Some(CurrencyId::R);
		}
		match l.unpack() {
			(parents, interior) if parents == 1 => match interior {
				[Parachain(1), GeneralKey { data, .. }] if data.to_vec() == a => Some(CurrencyId::A),
				[Parachain(1), GeneralKey { data, .. }] if data.to_vec() == a1 => Some(CurrencyId::A1),
				[Parachain(2), GeneralKey { data, .. }] if data.to_vec() == b => Some(CurrencyId::B),
				[Parachain(2), GeneralKey { data, .. }] if data.to_vec() == b1 => Some(CurrencyId::B1),
				[Parachain(2), GeneralKey { data, .. }] if data.to_vec() == b2 => Some(CurrencyId::B2),
				[Parachain(3), GeneralKey { data, .. }] if data.to_vec() == c => Some(CurrencyId::C),
				[Parachain(para_id), GeneralKey { data, .. }] if data.to_vec() == d && *para_id == self_para_id => {
					Some(CurrencyId::D)
				}
				_ => None,
			},
			(parents, interior) if parents == 0 => match interior {
				[GeneralKey { data, .. }] if data.to_vec() == a => Some(CurrencyId::A),
				[GeneralKey { data, .. }] if data.to_vec() == b => Some(CurrencyId::B),
				[GeneralKey { data, .. }] if data.to_vec() == a1 => Some(CurrencyId::A1),
				[GeneralKey { data, .. }] if data.to_vec() == b1 => Some(CurrencyId::B1),
				[GeneralKey { data, .. }] if data.to_vec() == b2 => Some(CurrencyId::B2),
				[GeneralKey { data, .. }] if data.to_vec() == c => Some(CurrencyId::C),
				[GeneralKey { data, .. }] if data.to_vec() == d => Some(CurrencyId::D),
				_ => None,
			},
			_ => None,
		}
	}
}
impl Convert<Asset, Option<CurrencyId>> for RelativeCurrencyIdConvert {
	fn convert(a: Asset) -> Option<CurrencyId> {
		if let Asset {
			fun: Fungible(_),
			id: AssetId(id),
		} = a
		{
			Self::convert(id)
		} else {
			Option::None
		}
	}
}

parameter_types! {
	pub SelfLocation: Location = Location::here();
	pub const MaxAssetsForTransfer: usize = 2;
}

pub struct ParentOrParachains;
impl Contains<Location> for ParentOrParachains {
	fn contains(location: &Location) -> bool {
		matches!(
			location.unpack(),
			(0, [Junction::AccountId32 { .. }])
				| (1, [Junction::AccountId32 { .. }])
				| (1, [Parachain(1), Junction::AccountId32 { .. }])
				| (1, [Parachain(2), Junction::AccountId32 { .. }])
				| (1, [Parachain(3), Junction::AccountId32 { .. }])
				| (1, [Parachain(4), Junction::AccountId32 { .. }])
				| (1, [Parachain(100), Junction::AccountId32 { .. }])
		)
	}
}

parameter_type_with_key! {
	pub ParachainMinFee: |location: Location| -> Option<u128> {
		#[allow(clippy::match_ref_pats)] // false positive
		match (location.parents, location.first_interior()) {
			(1, Some(Parachain(2))) => Some(40),
			(1, Some(Parachain(3))) => Some(40),
			_ => None,
		}
	};
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = RelativeCurrencyIdConvert;
	type AccountIdToLocation = AccountIdToLocation;
	type SelfLocation = SelfLocation;
	type LocationsFilter = ParentOrParachains;
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type UniversalLocation = UniversalLocation;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = RelativeReserveProvider;
	type RateLimiter = ();
	type RateLimiterId = ();
}

impl orml_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SovereignOrigin = EnsureRoot<AccountId>;
}

impl orml_xcm_mock_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Balances: pallet_balances,

		MsgQueue: orml_xcm_mock_message_queue,
		CumulusXcm: cumulus_pallet_xcm,

		Tokens: orml_tokens,
		XTokens: orml_xtokens,

		PolkadotXcm: pallet_xcm,
		OrmlXcm: orml_xcm,
	}
);
