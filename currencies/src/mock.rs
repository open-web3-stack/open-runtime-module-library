//! Mocks for the currencies module.

#![cfg(test)]

use frame_support::{impl_outer_origin, parameter_types};
use pallet_balances;
use primitives::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use tokens;

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime where system = frame_system {}
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

pub type AccountId = u64;
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
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
}

type CurrencyId = u32;
type Balance = u64;

parameter_types! {
	pub const ExistentialDeposit: u64 = 0;
	pub const TransferFee: u64 = 0;
	pub const CreationFee: u64 = 2;
}

impl pallet_balances::Trait for Runtime {
	type Balance = Balance;
	type OnFreeBalanceZero = ();
	type OnNewAccount = ();
	type OnReapAccount = ();
	type TransferPayment = ();
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

pub type PalletBalances = pallet_balances::Module<Runtime>;

impl tokens::Trait for Runtime {
	type Event = ();
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
	type ExistentialDeposit = ExistentialDeposit;
	type DustRemoval = ();
}

pub const NATIVE_CURRENCY_ID: CurrencyId = 1;
pub const X_TOKEN_ID: CurrencyId = 2;

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
}

impl Trait for Runtime {
	type Event = ();
	type MultiCurrency = tokens::Module<Runtime>;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}
pub type Currencies = Module<Runtime>;
pub type NativeCurrency = NativeCurrencyOf<Runtime>;
pub type AdaptedBasicCurrency = BasicCurrencyAdapter<Runtime, PalletBalances, Balance>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const EVA: AccountId = 5;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	// whether the configs are for `pallet_balances` or not
	is_for_pallet_balances: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			is_for_pallet_balances: false,
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 100),
			(BOB, NATIVE_CURRENCY_ID, 100),
			(ALICE, X_TOKEN_ID, 100),
			(BOB, X_TOKEN_ID, 100),
		])
	}

	pub fn make_for_pallet_balances(mut self) -> Self {
		self.is_for_pallet_balances = true;
		self
	}

	pub fn build(self) -> runtime_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		if self.is_for_pallet_balances {
			pallet_balances::GenesisConfig::<Runtime> {
				balances: self
					.endowed_accounts
					.into_iter()
					.filter(|(_, currency_id, _)| *currency_id == X_TOKEN_ID)
					.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
					.collect::<Vec<_>>(),
				vesting: vec![],
			}
			.assimilate_storage(&mut t)
			.unwrap();
		} else {
			tokens::GenesisConfig::<Runtime> {
				endowed_accounts: self.endowed_accounts,
			}
			.assimilate_storage(&mut t)
			.unwrap();
		}

		t.into()
	}
}
