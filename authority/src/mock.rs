//! Mocks for the authority module.

#![cfg(test)]

use frame_support::{
	parameter_types,
	traits::{OnFinalize, OnInitialize},
};
use frame_system::{ensure_root, ensure_signed, EnsureRoot};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BadOrigin, Block as BlockT, IdentityLookup},
	Perbill,
};

use super::*;
pub use crate as authority;

pub type AccountId = u128;
pub type BlockNumber = u64;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: u32 = Perbill::from_percent(80) * MaximumBlockWeight::get();
}
impl pallet_scheduler::Trait for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<u128>;
	type MaxScheduledPerBlock = ();
	type WeightInfo = ();
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum MockAsOriginId {
	Root,
	Account1,
	Account2,
}

pub struct AuthorityConfigImpl;

impl AuthorityConfig<Origin, OriginCaller, BlockNumber> for AuthorityConfigImpl {
	fn check_schedule_dispatch(origin: Origin, _priority: Priority) -> DispatchResult {
		let origin: Result<frame_system::RawOrigin<u128>, _> = origin.into();
		match origin {
			Ok(frame_system::RawOrigin::Root)
			| Ok(frame_system::RawOrigin::Signed(1))
			| Ok(frame_system::RawOrigin::Signed(2)) => Ok(()),
			_ => Err(BadOrigin.into()),
		}
	}
	fn check_fast_track_schedule(
		origin: Origin,
		_initial_origin: &OriginCaller,
		_new_delay: BlockNumber,
	) -> DispatchResult {
		ensure_root(origin)?;
		Ok(())
	}
	fn check_delay_schedule(origin: Origin, initial_origin: &OriginCaller) -> DispatchResult {
		ensure_root(origin.clone()).or_else(|_| {
			if origin.caller() == initial_origin {
				Ok(())
			} else {
				Err(BadOrigin.into())
			}
		})
	}
	fn check_cancel_schedule(origin: Origin, initial_origin: &OriginCaller) -> DispatchResult {
		ensure_root(origin.clone()).or_else(|_| {
			if origin.caller() == initial_origin {
				Ok(())
			} else {
				Err(BadOrigin.into())
			}
		})
	}
}

impl AsOriginId<Origin, OriginCaller> for MockAsOriginId {
	fn into_origin(self) -> OriginCaller {
		match self {
			MockAsOriginId::Root => Origin::root().caller().clone(),
			MockAsOriginId::Account1 => Origin::signed(1).caller().clone(),
			MockAsOriginId::Account2 => Origin::signed(2).caller().clone(),
		}
	}
	fn check_dispatch_from(&self, origin: Origin) -> DispatchResult {
		ensure_root(origin.clone()).or_else(|_| {
			if let OriginCaller::authority(ref sign) = origin.caller() {
				if sign.origin == Box::new(Origin::root().caller().clone()) {
					return Ok(());
				} else {
					return Err(BadOrigin.into());
				}
			}

			let ok = match self {
				MockAsOriginId::Root => false,
				MockAsOriginId::Account1 => ensure_signed(origin)? == 1,
				MockAsOriginId::Account2 => ensure_signed(origin)? == 2,
			};
			return if ok { Ok(()) } else { Err(BadOrigin.into()) };
		})
	}
}

impl Trait for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Scheduler = Scheduler;
	type Call = Call;
	type AsOriginId = MockAsOriginId;
	type AuthorityConfig = AuthorityConfigImpl;
	type WeightInfo = ();
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, u64, ()>;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Authority: authority::{Module, Call, Origin<T>, Event<T>},
		Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>},
	}
);

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		t.into()
	}
}

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		Scheduler::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Scheduler::on_initialize(System::block_number());
	}
}
