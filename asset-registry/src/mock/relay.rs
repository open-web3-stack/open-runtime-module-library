use crate::Weight;
use cumulus_primitives_core::ParaId;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU128, ConstU32, ConstU64, Everything, Nothing, ProcessMessage, ProcessMessageError},
	weights::{IdentityFee, WeightMeter},
};
use frame_system::EnsureRoot;
use polkadot_runtime_parachains::{
	configuration,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
	origin, shared,
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32};
use xcm::v3::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, ChildParachainAsNative, ChildParachainConvertsVia,
	CurrencyAdapter as XcmCurrencyAdapter, FixedWeightBounds, IsConcrete, SignedAccountId32AsNative,
	SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
};
use xcm_executor::{Config, XcmExecutor};

pub type AccountId = AccountId32;
pub type Balance = u128;

impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
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
	type FreezeIdentifier = [u8; 8];
	type MaxHolds = ();
	type MaxFreezes = ();
}

impl shared::Config for Runtime {}

impl configuration::Config for Runtime {
	type WeightInfo = configuration::TestWeightInfo;
}

parameter_types! {
	pub KsmLocation: MultiLocation = Here.into();
	pub const KusamaNetwork: NetworkId = NetworkId::Kusama;
	pub UniversalLocation: InteriorMultiLocation = Here;
}

pub type SovereignAccountOf = (
	ChildParachainConvertsVia<ParaId, AccountId>,
	AccountId32Aliases<KusamaNetwork, AccountId>,
);

pub type LocalAssetTransactor =
	XcmCurrencyAdapter<Balances, IsConcrete<KsmLocation>, SovereignAccountOf, AccountId, ()>;

type LocalOriginConverter = (
	SovereignSignedViaLocation<SovereignAccountOf, RuntimeOrigin>,
	ChildParachainAsNative<origin::Origin, RuntimeOrigin>,
	SignedAccountId32AsNative<KusamaNetwork, RuntimeOrigin>,
);

pub type XcmRouter = super::RelayChainXcmRouter;
pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

parameter_types! {
	pub const BaseXcmWeight: Weight = Weight::from_parts(10, 10);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	type IsReserve = ();
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type Trader = UsingComponents<IdentityFee<Balance>, KsmLocation, AccountId, Balances, ()>;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = XcmPallet;
	type AssetLocker = XcmPallet;
	type AssetExchanger = ();
	type PalletInstancesInfo = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, KusamaNetwork>;

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::here();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(MultiLocation::here());
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	// Anyone can execute XCM messages locally...
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<BaseXcmWeight, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = IsConcrete<KsmLocation>;
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
}

impl origin::Config for Runtime {}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Weight::from_parts(1_000_000_000, 1_000_000);
	pub const MessageQueueHeapSize: u32 = 65_536;
	pub const MessageQueueMaxStale: u32 = 16;
}

pub struct MessageProcessor;
impl ProcessMessage for MessageProcessor {
	type Origin = AggregateMessageOrigin;

	fn process_message(
		message: &[u8],
		origin: Self::Origin,
		meter: &mut WeightMeter,
		id: &mut [u8; 32],
	) -> Result<bool, ProcessMessageError> {
		let para = match origin {
			AggregateMessageOrigin::Ump(UmpQueueId::Para(para)) => para,
		};
		xcm_builder::ProcessXcmMessage::<Junction, xcm_executor::XcmExecutor<XcmConfig>, RuntimeCall>::process_message(
			message,
			Junction::Parachain(para.into()),
			meter,
			id,
		)
	}
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Size = u32;
	type HeapSize = MessageQueueHeapSize;
	type MaxStale = MessageQueueMaxStale;
	type ServiceWeight = MessageQueueServiceWeight;
	type MessageProcessor = MessageProcessor;
	type QueueChangeHandler = ();
	type QueuePausedQuery = ();
	type WeightInfo = ();
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		ParasOrigin: origin::{Pallet, Origin},
		MessageQueue: pallet_message_queue::{Pallet, Event<T>},
		XcmPallet: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin},
	}
);
