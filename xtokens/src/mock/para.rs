use super::{Amount, Balance, CurrencyId, CurrencyIdConvert, ParachainXcmRouter};
use crate as orml_xtokens;

use frame_support::{
	construct_runtime, match_types, parameter_types,
	traits::{ConstU128, ConstU32, ConstU64, Everything, Get, Nothing},
	weights::{constants::WEIGHT_PER_SECOND, Weight},
};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{Convert, IdentityLookup, Zero},
	AccountId32,
};

use cumulus_primitives_core::{ChannelStatus, GetChannelInfo, ParaId};
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds, LocationInverter,
	ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_executor::{traits::WeightTrader, Assets, Config, XcmExecutor};

use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key};
use orml_xcm_support::{IsNativeConcrete, MultiCurrencyAdapter, MultiNativeAsset};

pub type AccountId = AccountId32;

impl frame_system::Config for Runtime {
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
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
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = Everything;
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = WEIGHT_PER_SECOND / 4;
	pub const ReservedDmpWeight: Weight = WEIGHT_PER_SECOND / 4;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToCallOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	RelayChainAsNative<RelayChainOrigin, Origin>,
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	XcmPassthrough<Origin>,
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

pub type XcmRouter = ParachainXcmRouter<ParachainInfo>;
pub type Barrier = (TakeWeightCredit, AllowTopLevelPaidExecutionFrom<Everything>);

/// A trader who believes all tokens are created equal to "weight" of any chain,
/// which is not true, but good enough to mock the fee payment of XCM execution.
///
/// This mock will always trade `n` amount of weight to `n` amount of tokens.
pub struct AllTokensAreCreatedEqualToWeight(MultiLocation);
impl WeightTrader for AllTokensAreCreatedEqualToWeight {
	fn new() -> Self {
		Self(MultiLocation::parent())
	}

	fn buy_weight(&mut self, weight: Weight, payment: Assets) -> Result<Assets, XcmError> {
		let asset_id = payment
			.fungible
			.iter()
			.next()
			.expect("Payment must be something; qed")
			.0;
		let required = MultiAsset {
			id: asset_id.clone(),
			fun: Fungible(weight as u128),
		};

		if let MultiAsset {
			fun: _,
			id: Concrete(ref id),
		} = &required
		{
			self.0 = id.clone();
		}

		let unused = payment.checked_sub(required).map_err(|_| XcmError::TooExpensive)?;
		Ok(unused)
	}

	fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
		if weight.is_zero() {
			None
		} else {
			Some((self.0.clone(), weight as u128).into())
		}
	}
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<ConstU64<10>, Call, ConstU32<100>>;
	type Trader = AllTokensAreCreatedEqualToWeight;
	type ResponseHandler = ();
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
}

pub struct ChannelInfo;
impl GetChannelInfo for ChannelInfo {
	fn get_channel_status(_id: ParaId) -> ChannelStatus {
		ChannelStatus::Ready(10, 10)
	}
	fn get_channel_max(_id: ParaId) -> Option<usize> {
		Some(usize::max_value())
	}
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ChannelInfo;
	type VersionWrapper = ();
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToCallOrigin;
	type WeightInfo = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<ConstU64<10>, Call, ConstU32<100>>;
	type LocationInverter = LocationInverter<Ancestry>;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(Junction::AccountId32 {
			network: NetworkId::Any,
			id: account.into(),
		})
		.into()
	}
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
	pub const MaxAssetsForTransfer: usize = 3;
}

match_types! {
	pub type ParentOrParachains: impl Contains<MultiLocation> = {
		MultiLocation { parents: 0, interior: X1(Junction::AccountId32 { .. }) } |
		MultiLocation { parents: 1, interior: X1(Junction::AccountId32 { .. }) } |
		MultiLocation { parents: 1, interior: X2(Parachain(1), Junction::AccountId32 { .. }) } |
		MultiLocation { parents: 1, interior: X2(Parachain(2), Junction::AccountId32 { .. }) } |
		MultiLocation { parents: 1, interior: X2(Parachain(3), Junction::AccountId32 { .. }) } |
		MultiLocation { parents: 1, interior: X2(Parachain(4), Junction::AccountId32 { .. }) } |
		MultiLocation { parents: 1, interior: X2(Parachain(100), Junction::AccountId32 { .. }) }
	};
}

parameter_type_with_key! {
	pub ParachainMinFee: |location: MultiLocation| -> Option<u128> {
		#[allow(clippy::match_ref_pats)] // false positive
		match (location.parents, location.first_interior()) {
			(1, Some(Parachain(2))) => Some(40),
			_ => None,
		}
	};
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert;
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = SelfLocation;
	type MultiLocationsFilter = ParentOrParachains;
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<ConstU64<10>, Call, ConstU32<100>>;
	type BaseXcmWeight = ConstU64<100_000_000>;
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = AbsoluteReserveProvider;
}

impl orml_xcm::Config for Runtime {
	type Event = Event;
	type SovereignOrigin = EnsureRoot<AccountId>;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

		ParachainInfo: parachain_info::{Pallet, Storage, Config},
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>},
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>},
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin},

		Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
		XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>},

		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin},
		OrmlXcm: orml_xcm::{Pallet, Call, Event<T>},
	}
);
