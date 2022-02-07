//! Mocks for the vesting module.

#![cfg(test)]

use super::*;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{EnsureOrigin, Everything},
};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

use crate as vesting;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

pub type AccountId = u128;
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
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

type Balance = u64;

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

pub struct EnsureAliceOrBob;
impl EnsureOrigin<Origin> for EnsureAliceOrBob {
	type Success = AccountId;

	fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
		Into::<Result<RawOrigin<AccountId>, Origin>>::into(o).and_then(|o| match o {
			RawOrigin::Signed(ALICE) => Ok(ALICE),
			RawOrigin::Signed(BOB) => Ok(BOB),
			r => Err(Origin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> Origin {
		Origin::from(RawOrigin::Signed(Default::default()))
	}
}

parameter_types! {
	pub const MaxVestingSchedule: u32 = 2;
	pub const MinVestedTransfer: u64 = 5;
	pub static MockBlockNumberProvider: u64 = 0;
}

impl BlockNumberProvider for MockBlockNumberProvider {
	type BlockNumber = u64;

	fn current_block_number() -> Self::BlockNumber {
		Self::get()
	}
}

impl Config for Runtime {
	type Event = Event;
	type Currency = PalletBalances;
	type MinVestedTransfer = MinVestedTransfer;
	type VestedTransferOrigin = EnsureAliceOrBob;
	type WeightInfo = ();
	type MaxVestingSchedules = MaxVestingSchedule;
	type BlockNumberProvider = MockBlockNumberProvider;
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
		Vesting: vesting::{Pallet, Storage, Call, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![(ALICE, 100), (CHARLIE, 50)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		vesting::GenesisConfig::<Runtime> {
			vesting: vec![
				// who, start, period, period_count, per_period
				(CHARLIE, 2, 3, 1, 5),
				(CHARLIE, 2 + 3, 3, 3, 5),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
