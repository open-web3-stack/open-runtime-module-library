//! Mocks for the vesting module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use frame_system::RawOrigin;
use pallet_balances;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod vesting {
	pub use crate::Event;
}
impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		vesting<T>,
		pallet_balances<T>,
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

pub type AccountId = u128;
impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}
pub type System = frame_system::Module<Runtime>;

type Balance = u64;

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MinVestedTransfer: u64 = 5;
}

impl pallet_balances::Trait for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = TestEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
	type MaxLocks = ();
	type WeightInfo = ();
}
pub type PalletBalances = pallet_balances::Module<Runtime>;

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

impl Trait for Runtime {
	type Event = TestEvent;
	type Currency = PalletBalances;
	type MinVestedTransfer = MinVestedTransfer;
	type VestedTransferOrigin = EnsureAliceOrBob;
	type WeightInfo = ();
}
pub type Vesting = Module<Runtime>;

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
			balances: vec![(ALICE, 100), (CHARLIE, 30)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		GenesisConfig::<Runtime> {
			vesting: vec![(CHARLIE, 2, 3, 4, 5)], // who, start, period, period_count, per_period
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
