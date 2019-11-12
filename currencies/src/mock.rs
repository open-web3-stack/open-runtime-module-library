//! Mocks for the currencies module.

#![cfg(test)]

use primitives::H256;
use sr_primitives::{testing::Header, traits::IdentityLookup, Perbill};
use srml_balances;
use srml_support::{impl_outer_origin, parameter_types};

use tokens;

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
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
impl system::Trait for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = ();
	type Hash = H256;
	type Hashing = ::sr_primitives::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}

type CurrencyId = u32;
type Balance = u64;

parameter_types! {
	pub const ExistentialDeposit: u64 = 0;
	pub const TransferFee: u64 = 0;
	pub const CreationFee: u64 = 2;
}

impl srml_balances::Trait for Runtime {
	type Balance = Balance;
	type OnFreeBalanceZero = ();
	type OnNewAccount = ();
	type Event = ();
	type DustRemoval = ();
	type TransferPayment = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

pub type SrmlBalances = srml_balances::Module<Runtime>;

impl tokens::Trait for Runtime {
	type Event = ();
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
}

pub const NATIVE_CURRENCY_ID: CurrencyId = 1;
pub const X_TOKEN_ID: CurrencyId = 2;

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
}

impl Trait for Runtime {
	type Event = ();
	type MultiCurrency = tokens::Module<Runtime>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type NativeCurrency = NativeCurrency;
}
pub type Currencies = Module<Runtime>;
pub type NativeCurrency = NativeCurrencyOf<Runtime>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const EVA: AccountId = 5;

pub struct ExtBuilder {
	currency_ids: Vec<CurrencyId>,
	endowed_accounts: Vec<AccountId>,
	initial_balance: Balance,
	// whether the configs are for `srml_balances` or not
	is_for_srml_balances: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			currency_ids: vec![NATIVE_CURRENCY_ID, X_TOKEN_ID],
			endowed_accounts: vec![0],
			initial_balance: 0,
			is_for_srml_balances: false,
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, account_ids: Vec<AccountId>, initial_balance: Balance) -> Self {
		self.endowed_accounts = account_ids;
		self.initial_balance = initial_balance;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![ALICE, BOB], 100)
	}

	pub fn make_for_srml_balances(mut self) -> Self {
		self.is_for_srml_balances = true;
		self
	}

	pub fn build(self) -> runtime_io::TestExternalities {
		let mut t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		if self.is_for_srml_balances {
			srml_balances::GenesisConfig::<Runtime> {
				balances: self
					.endowed_accounts
					.iter()
					.map(|acc| (acc.clone(), self.initial_balance))
					.collect(),
				vesting: vec![],
			}
			.assimilate_storage(&mut t)
			.unwrap();
		} else {
			tokens::GenesisConfig::<Runtime> {
				tokens: self.currency_ids,
				initial_balance: self.initial_balance,
				endowed_accounts: self.endowed_accounts,
			}
			.assimilate_storage(&mut t)
			.unwrap();
		}

		t.into()
	}
}
