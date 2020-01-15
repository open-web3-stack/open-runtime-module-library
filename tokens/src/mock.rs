//! Mocks for the tokens module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use frame_system as system;
use primitives::H256;
use rstd::{cell::RefCell, marker::PhantomData};
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod tokens {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		tokens<T>,
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

type AccountId = u64;
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
	type ModuleToIndex = ();
}
pub type System = system::Module<Runtime>;

type CurrencyId = u32;
pub type Balance = u64;

parameter_types! {
	pub const ExistentialDeposit: u64 = 2;
}

thread_local! {
	static ACCUMULATED_DUST: RefCell<Balance> = RefCell::new(Zero::zero());
}

pub struct MockDustRemoval<Balance>(PhantomData<Balance>);
impl MockDustRemoval<Balance> {
	pub fn accumulated_dust() -> Balance {
		ACCUMULATED_DUST.with(|v| *v.borrow_mut())
	}
}
impl OnDustRemoval<Balance> for MockDustRemoval<Balance> {
	fn on_dust_removal(balance: Balance) {
		ACCUMULATED_DUST.with(|v| *v.borrow_mut() += balance);
	}
}

impl Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
	type ExistentialDeposit = ExistentialDeposit;
	type DustRemoval = MockDustRemoval<Balance>;
}

pub type Tokens = Module<Runtime>;

pub const TEST_TOKEN_ID: CurrencyId = 1;
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![(ALICE, TEST_TOKEN_ID, 100), (BOB, TEST_TOKEN_ID, 100)])
	}

	pub fn build(self) -> runtime_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
