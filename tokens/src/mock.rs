//! Mocks for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ChangeMembers, ConstU32, ConstU64, ContainsLengthBound, Everything, GenesisBuild, SaturatingCurrencyToVote,
		SortedMembers,
	},
	PalletId,
};
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, IdentityLookup},
	AccountId32, Permill,
};
use sp_std::cell::RefCell;

pub type AccountId = AccountId32;
pub type CurrencyId = u32;
pub type Balance = u64;
pub type ReserveIdentifier = [u8; 8];

pub const DOT: CurrencyId = 1;
pub const BTC: CurrencyId = 2;
pub const ETH: CurrencyId = 3;
pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([2u8; 32]);
pub const DAVE: AccountId = AccountId32::new([3u8; 32]);
pub const TREASURY_ACCOUNT: AccountId = AccountId32::new([4u8; 32]);
pub const ID_1: LockIdentifier = *b"1       ";
pub const ID_2: LockIdentifier = *b"2       ";
pub const ID_3: LockIdentifier = *b"3       ";
pub const RID_1: ReserveIdentifier = [1u8; 8];
pub const RID_2: ReserveIdentifier = [2u8; 8];

use crate as tokens;

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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

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
impl SortedMembers<AccountId> for TenToFourteen {
	fn sorted_members() -> Vec<AccountId> {
		TEN_TO_FOURTEEN.with(|v| v.borrow().clone())
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(new: &AccountId) {
		TEN_TO_FOURTEEN.with(|v| {
			let mut members = v.borrow_mut();
			members.push(new.clone());
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
	pub const ProposalBondMaximum: u64 = 5;
	pub const SpendPeriod: u64 = 2;
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const GetTokenId: CurrencyId = DOT;
	pub const MaxApprovals: u32 = 100;
}

pub type MockCurrencyAdapter = CurrencyAdapter<Runtime, GetTokenId>;
impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = MockCurrencyAdapter;
	type ApproveOrigin = frame_system::EnsureRoot<AccountId>;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type Event = Event;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = ();
	type MaxApprovals = MaxApprovals;
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
	pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
}

impl pallet_elections_phragmen::Config for Runtime {
	type PalletId = ElectionsPhragmenPalletId;
	type Event = Event;
	type Currency = MockCurrencyAdapter;
	type CurrencyToVote = SaturatingCurrencyToVote;
	type ChangeMembers = TestChangeMembers;
	type InitializeMembers = ();
	type CandidacyBond = ConstU64<3>;
	type VotingBondBase = ConstU64<2>;
	type VotingBondFactor = ConstU64<0>;
	type TermDuration = ConstU64<5>;
	type DesiredMembers = ConstU32<2>;
	type DesiredRunnersUp = ConstU32<2>;
	type LoserCandidate = ();
	type KickedMember = ();
	type WeightInfo = ();
}

pub struct MockDustRemovalWhitelist;
impl Contains<AccountId> for MockDustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		*a == DAVE || *a == DustReceiver::get()
	}
}

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		#[allow(clippy::match_ref_pats)] // false positive
		match currency_id {
			&BTC => 1,
			&DOT => 2,
			_ => 0,
		}
	};
}

thread_local! {
	pub static CREATED: RefCell<Vec<(AccountId, CurrencyId)>> = RefCell::new(vec![]);
	pub static KILLED: RefCell<Vec<(AccountId, CurrencyId)>> = RefCell::new(vec![]);
}

pub struct TrackCreatedAccounts;
impl TrackCreatedAccounts {
	pub fn accounts() -> Vec<(AccountId, CurrencyId)> {
		CREATED.with(|accounts| accounts.borrow().clone())
	}

	pub fn reset() {
		CREATED.with(|accounts| {
			accounts.replace(vec![]);
		});
	}
}
impl Happened<(AccountId, CurrencyId)> for TrackCreatedAccounts {
	fn happened((who, currency): &(AccountId, CurrencyId)) {
		CREATED.with(|accounts| {
			accounts.borrow_mut().push((who.clone(), *currency));
		});
	}
}

pub struct TrackKilledAccounts;
impl TrackKilledAccounts {
	pub fn accounts() -> Vec<(AccountId, CurrencyId)> {
		KILLED.with(|accounts| accounts.borrow().clone())
	}

	pub fn reset() {
		KILLED.with(|accounts| {
			accounts.replace(vec![]);
		});
	}
}
impl Happened<(AccountId, CurrencyId)> for TrackKilledAccounts {
	fn happened((who, currency): &(AccountId, CurrencyId)) {
		KILLED.with(|accounts| {
			accounts.borrow_mut().push((who.clone(), *currency));
		});
	}
}

parameter_types! {
	pub DustReceiver: AccountId = PalletId(*b"orml/dst").into_account_truncating();
}

impl Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = TransferDust<Runtime, DustReceiver>;
	type OnNewTokenAccount = TrackCreatedAccounts;
	type OnKilledTokenAccount = TrackKilledAccounts;
	type MaxLocks = ConstU32<2>;
	type MaxReserves = ConstU32<2>;
	type ReserveIdentifier = ReserveIdentifier;
	type DustRemovalWhitelist = MockDustRemovalWhitelist;
}
pub type TreasuryCurrencyAdapter = <Runtime as pallet_treasury::Config>::Currency;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Tokens: tokens::{Pallet, Storage, Event<T>, Config<T>},
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},
		ElectionsPhragmen: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>},
	}
);

pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
	treasury_genesis: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			balances: vec![],
			treasury_genesis: false,
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, mut balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.balances.append(&mut balances);
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		tokens::GenesisConfig::<Runtime> {
			balances: self.balances,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		if self.treasury_genesis {
			GenesisBuild::<Runtime>::assimilate_storage(&pallet_treasury::GenesisConfig::default(), &mut t).unwrap();

			pallet_elections_phragmen::GenesisConfig::<Runtime> {
				members: vec![(TREASURY_ACCOUNT, 10)],
			}
			.assimilate_storage(&mut t)
			.unwrap();
		}

		TrackCreatedAccounts::reset();
		TrackKilledAccounts::reset();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
