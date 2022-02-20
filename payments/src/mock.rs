use crate as payment;
use crate::PaymentDetail;
use frame_support::{
	parameter_types,
	traits::{Contains, Everything, GenesisBuild, OnFinalize, OnInitialize},
};
use frame_system as system;
use orml_traits::parameter_type_with_key;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Percent,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type Balance = u128;

pub type AccountId = u8;
pub const PAYMENT_CREATOR: AccountId = 10;
pub const PAYMENT_RECIPENT: AccountId = 11;
pub const CURRENCY_ID: u128 = 1;
pub const RESOLVER_ACCOUNT: AccountId = 12;
pub const FEE_RECIPIENT_ACCOUNT: AccountId = 20;
pub const PAYMENT_RECIPENT_FEE_CHARGED: AccountId = 21;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Config<T>, Storage, Event<T>},
		Payment: payment::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: u128| -> Balance {
		0u128
	};
}
parameter_types! {
	pub const MaxLocks: u32 = 50;
}

pub struct MockDustRemovalWhitelist;
impl Contains<AccountId> for MockDustRemovalWhitelist {
	fn contains(_a: &AccountId) -> bool {
		false
	}
}

impl orml_tokens::Config for Test {
	type Amount = i64;
	type Balance = Balance;
	type CurrencyId = u128;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = MockDustRemovalWhitelist;
}

pub struct MockDisputeResolver;
impl crate::types::DisputeResolver<AccountId> for MockDisputeResolver {
	fn get_origin() -> AccountId {
		RESOLVER_ACCOUNT
	}
}

pub struct MockFeeHandler;
impl crate::types::FeeHandler<Test> for MockFeeHandler {
	fn apply_fees(_from: &AccountId, to: &AccountId, _remark: &PaymentDetail<Test>) -> (AccountId, Percent) {
		match to {
			&PAYMENT_RECIPENT_FEE_CHARGED => (FEE_RECIPIENT_ACCOUNT, Percent::from_percent(10)),
			_ => (FEE_RECIPIENT_ACCOUNT, Percent::from_percent(0)),
		}
	}
}

parameter_types! {
	pub const IncentivePercentage: Percent = Percent::from_percent(10);
	pub const MaxRemarkLength: u32 = 50;
	pub const CancelBufferBlockLength: u64 = 600;
}

impl payment::Config for Test {
	type Event = Event;
	type Asset = Tokens;
	type DisputeResolver = MockDisputeResolver;
	type IncentivePercentage = IncentivePercentage;
	type FeeHandler = MockFeeHandler;
	type MaxRemarkLength = MaxRemarkLength;
	type CancelBufferBlockLength = CancelBufferBlockLength;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![(PAYMENT_CREATOR, CURRENCY_ID, 100)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext: sp_io::TestExternalities = t.into();
	// need to set block number to 1 to test events
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
	}
}
