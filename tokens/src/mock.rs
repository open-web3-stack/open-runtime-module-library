//! Mocks for the tokens module.

#![cfg(test)]

use frame_support::{
	impl_outer_event, impl_outer_origin, parameter_types,
	traits::{ChangeMembers, Contains, ContainsLengthBound, SaturatingCurrencyToVote},
};
use frame_system as system;
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, IdentityLookup},
	AccountId32, ModuleId, Perbill, Percent, Permill,
};
use sp_std::cell::RefCell;

use super::*;

pub type AccountId = AccountId32;
pub type CurrencyId = u32;
pub type Balance = u64;

pub const DOT: CurrencyId = 1;
pub const BTC: CurrencyId = 2;
pub const ETH: CurrencyId = 3;
pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
pub const TREASURY_ACCOUNT: AccountId = AccountId32::new([2u8; 32]);
pub const ID_1: LockIdentifier = *b"1       ";
pub const ID_2: LockIdentifier = *b"2       ";

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod tokens {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		tokens<T>,
		pallet_treasury<T>,
		pallet_elections_phragmen<T>,
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
pub type System = system::Module<Runtime>;

thread_local! {
	static TEN_TO_FOURTEEN: RefCell<Vec<AccountId>> = RefCell::new(vec![
		AccountId32::new([10u8; 32]),
		AccountId32::new([11u8; 32]),
		AccountId32::new([12u8; 32]),
		AccountId32::new([13u8; 32]),
		AccountId32::new([14u8; 32]),
	]);
}

pub struct TenToFourteen;
impl Contains<AccountId> for TenToFourteen {
	fn sorted_members() -> Vec<AccountId> {
		TEN_TO_FOURTEEN.with(|v| v.borrow().clone())
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(new: &AccountId) {
		TEN_TO_FOURTEEN.with(|v| {
			let mut members = v.borrow_mut();
			members.push(*new);
			members.sort();
		})
	}
}

impl ContainsLengthBound for TenToFourteen {
	fn max_len() -> usize {
		TEN_TO_FOURTEEN.with(|v| v.borrow().len())
	}
	fn min_len() -> usize {
		0
	}
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: u64 = 1;
	pub const TipCountdown: u64 = 1;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: u64 = 1;
	pub const DataDepositPerByte: u64 = 1;
	pub const SpendPeriod: u64 = 2;
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");
	pub const GetTokenId: CurrencyId = DOT;
}

impl pallet_treasury::Trait for Runtime {
	type ModuleId = TreasuryModuleId;
	type Currency = CurrencyAdapter<Runtime, GetTokenId>;
	type ApproveOrigin = frame_system::EnsureRoot<AccountId>;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type Tippers = TenToFourteen;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = TestEvent;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = (); // Just gets burned.
	type BountyDepositBase = ();
	type BountyDepositPayoutDelay = ();
	type BountyUpdatePeriod = ();
	type BountyCuratorDeposit = ();
	type BountyValueMinimum = ();
	type MaximumReasonLength = ();
	type WeightInfo = ();
}
pub type Treasury = pallet_treasury::Module<Runtime>;

parameter_types! {
	pub const CandidacyBond: u64 = 3;
}

thread_local! {
	static VOTING_BOND: RefCell<u64> = RefCell::new(2);
	static DESIRED_MEMBERS: RefCell<u32> = RefCell::new(2);
	static DESIRED_RUNNERS_UP: RefCell<u32> = RefCell::new(2);
	static TERM_DURATION: RefCell<u64> = RefCell::new(5);
}

pub struct VotingBond;
impl Get<u64> for VotingBond {
	fn get() -> u64 {
		VOTING_BOND.with(|v| *v.borrow())
	}
}

pub struct DesiredMembers;
impl Get<u32> for DesiredMembers {
	fn get() -> u32 {
		DESIRED_MEMBERS.with(|v| *v.borrow())
	}
}

pub struct DesiredRunnersUp;
impl Get<u32> for DesiredRunnersUp {
	fn get() -> u32 {
		DESIRED_RUNNERS_UP.with(|v| *v.borrow())
	}
}

pub struct TermDuration;
impl Get<u64> for TermDuration {
	fn get() -> u64 {
		TERM_DURATION.with(|v| *v.borrow())
	}
}

thread_local! {
	pub static MEMBERS: RefCell<Vec<AccountId>> = RefCell::new(vec![]);
	pub static PRIME: RefCell<Option<AccountId>> = RefCell::new(None);
}

pub struct TestChangeMembers;
impl ChangeMembers<AccountId> for TestChangeMembers {
	fn change_members_sorted(incoming: &[AccountId], outgoing: &[AccountId], new: &[AccountId]) {
		// new, incoming, outgoing must be sorted.
		let mut new_sorted = new.to_vec();
		new_sorted.sort();
		assert_eq!(new, &new_sorted[..]);

		let mut incoming_sorted = incoming.to_vec();
		incoming_sorted.sort();
		assert_eq!(incoming, &incoming_sorted[..]);

		let mut outgoing_sorted = outgoing.to_vec();
		outgoing_sorted.sort();
		assert_eq!(outgoing, &outgoing_sorted[..]);

		// incoming and outgoing must be disjoint
		for x in incoming.iter() {
			assert!(outgoing.binary_search(x).is_err());
		}

		let mut old_plus_incoming = MEMBERS.with(|m| m.borrow().to_vec());
		old_plus_incoming.extend_from_slice(incoming);
		old_plus_incoming.sort();

		let mut new_plus_outgoing = new.to_vec();
		new_plus_outgoing.extend_from_slice(outgoing);
		new_plus_outgoing.sort();

		assert_eq!(
			old_plus_incoming, new_plus_outgoing,
			"change members call is incorrect!"
		);

		MEMBERS.with(|m| *m.borrow_mut() = new.to_vec());
		PRIME.with(|p| *p.borrow_mut() = None);
	}

	fn set_prime(who: Option<AccountId>) {
		PRIME.with(|p| *p.borrow_mut() = who);
	}
}

parameter_types! {
	pub const ElectionsPhragmenModuleId: LockIdentifier = *b"phrelect";
}

impl pallet_elections_phragmen::Trait for Runtime {
	type ModuleId = ElectionsPhragmenModuleId;
	type Event = TestEvent;
	type Currency = CurrencyAdapter<Runtime, GetTokenId>;
	type CurrencyToVote = SaturatingCurrencyToVote;
	type ChangeMembers = TestChangeMembers;
	type InitializeMembers = ();
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type TermDuration = TermDuration;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type LoserCandidate = ();
	type KickedMember = ();
	type BadReport = ();
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&BTC => 1,
			&DOT => 2,
			_ => 0,
		}
	};
}

parameter_types! {
	pub DustAccount: AccountId = ModuleId(*b"orml/dst").into_account();
}

impl Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = TransferDust<Runtime, DustAccount>;
}
pub type Tokens = Module<Runtime>;
pub type TreasuryCurrencyAdapter = <Runtime as pallet_treasury::Trait>::Currency;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	treasury_genesis: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			treasury_genesis: false,
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
	}

	pub fn one_hundred_for_treasury_account(mut self) -> Self {
		self.treasury_genesis = true;
		self.balances(vec![(TREASURY_ACCOUNT, DOT, 100)])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		if self.treasury_genesis {
			pallet_treasury::GenesisConfig::default()
				.assimilate_storage::<Runtime, _>(&mut t)
				.unwrap();

			pallet_elections_phragmen::GenesisConfig::<Runtime> {
				members: vec![(TREASURY_ACCOUNT, 10)],
			}
			.assimilate_storage(&mut t)
			.unwrap();
		}

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
