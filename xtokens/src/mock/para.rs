use super::{
	AllowTopLevelPaidExecution, Amount, Balance, CurrencyId, CurrencyIdConvert, ParachainXcmRouter, RateLimiter,
	CHARLIE,
};
use crate as orml_xtokens;

use frame_support::{
	construct_runtime, derive_impl, ensure, parameter_types,
	traits::{ConstU128, ConstU32, Contains, ContainsPair, Everything, Get, Nothing},
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use parity_scale_codec::Encode;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::{
	traits::{Convert, IdentityLookup},
	AccountId32,
};
use sp_std::{cell::RefCell, marker::PhantomData};
use xcm::v4::{prelude::*, Weight};
use xcm_builder::{
	AccountId32Aliases, EnsureXcmOrigin, FixedWeightBounds, NativeAsset, ParentIsPreset, RelayChainAsNative,
	SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor};

use crate::mock::AllTokensAreCreatedEqualToWeight;
use orml_traits::{
	location::{AbsoluteReserveProvider, Reserve},
	parameter_type_with_key, RateLimiterError,
};
use orml_xcm_support::{IsNativeConcrete, MultiCurrencyAdapter};

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
pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecution);

parameter_types! {
	pub const UnitWeightCost: Weight = Weight::from_parts(10, 10);
	pub const BaseXcmWeight: Weight = Weight::from_parts(100_000_000, 100_000_000);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct MultiNativeAsset<ReserveProvider>(PhantomData<ReserveProvider>);
impl<ReserveProvider> ContainsPair<Asset, Location> for MultiNativeAsset<ReserveProvider>
where
	ReserveProvider: Reserve,
{
	fn contains(asset: &Asset, origin: &Location) -> bool {
		if let Some(ref reserve) = ReserveProvider::reserve(asset) {
			if reserve == origin {
				return true;
			}
		}
		// allow parachain to be reserved of relay to bypass https://github.com/paritytech/polkadot-sdk/pull/5660
		if asset.id.0 == Location::parent() {
			return true;
		}
		false
	}
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = NativeAsset;
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
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = ();
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

parameter_types! {
	pub SelfLocation: Location = Location::new(1, [Parachain(MsgQueue::get().into())]);
	pub const MaxAssetsForTransfer: usize = 3;
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
			(1, Some(Parachain(3))) => Some(40),
			_ => None,
		}
	};
}

thread_local! {
	pub static R_ACCUMULATION: RefCell<u128> = RefCell::new(0);
}

pub struct MockRateLimiter;
impl RateLimiter for MockRateLimiter {
	type RateLimiterId = u8;

	fn is_whitelist(_: Self::RateLimiterId, key: impl Encode) -> bool {
		let encoded_charlie = CHARLIE.encode();
		let encoded_key: Vec<u8> = key.encode();
		encoded_key != encoded_charlie
	}

	fn can_consume(_: Self::RateLimiterId, limit_key: impl Encode, value: u128) -> Result<(), RateLimiterError> {
		let encoded_limit_key = limit_key.encode();
		let r_multi_location: Location = CurrencyIdConvert::convert(CurrencyId::R).unwrap();
		let r_asset_id = AssetId(r_multi_location);
		let encoded_r_asset_id = r_asset_id.encode();

		if encoded_limit_key == encoded_r_asset_id {
			let accumulation = R_ACCUMULATION.with(|v| *v.borrow());
			ensure!((accumulation + value) <= 2000, RateLimiterError::ExceedLimit);
		}

		Ok(())
	}

	fn consume(_: Self::RateLimiterId, limit_key: impl Encode, value: u128) {
		let encoded_limit_key = limit_key.encode();
		let r_multi_location: Location = CurrencyIdConvert::convert(CurrencyId::R).unwrap();
		let r_asset_id = AssetId(r_multi_location);
		let encoded_r_asset_id = r_asset_id.encode();

		if encoded_limit_key == encoded_r_asset_id {
			R_ACCUMULATION.with(|v| *v.borrow_mut() += value);
		}
	}
}

parameter_types! {
	pub const XtokensRateLimiterId: u8 = 0;
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert;
	type AccountIdToLocation = AccountIdToLocation;
	type SelfLocation = SelfLocation;
	type LocationsFilter = ParentOrParachains;
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type UniversalLocation = UniversalLocation;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = AbsoluteReserveProvider;
	type RateLimiter = MockRateLimiter;
	type RateLimiterId = XtokensRateLimiterId;
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
