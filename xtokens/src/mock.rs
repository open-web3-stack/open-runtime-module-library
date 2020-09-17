//! Mocks for the xtokens module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use frame_system as system;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use sp_std::cell::RefCell;

use super::*;

type AccountId = u128;
pub type Balance = u128;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod xtokens {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		orml_tokens<T>,
		xtokens<T>,
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
	type ModuleToIndex = ();
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

#[repr(u8)]
#[derive(Encode, Decode, Serialize, Deserialize, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
pub enum CurrencyId {
	Owned = 0,
	BTC,
	DOT,
}
impl Into<Vec<u8>> for CurrencyId {
	fn into(self) -> Vec<u8> {
		vec![self as u8]
	}
}

impl TryFrom<Vec<u8>> for CurrencyId {
	type Error = ();

	fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
		if v.len() == 1 {
			let num = v[0];
			match num {
				0 => return Ok(CurrencyId::Owned),
				1 => return Ok(CurrencyId::BTC),
				2 => return Ok(CurrencyId::DOT),
				_ => return Err(()),
			};
		}
		Err(())
	}
}

pub fn unknown_currency_id() -> Vec<u8> {
	vec![10]
}

impl orml_tokens::Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type OnReceived = ();
}
pub type Tokens = orml_tokens::Module<Runtime>;

parameter_types! {
	pub const RelayChainCurrencyId: CurrencyId = CurrencyId::DOT;
	pub MockParaId: ParaId = 0.into();
}

impl Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type ToRelayChainBalance = BalanceConvertor;
	type FromRelayChainBalance = BalanceConvertor;
	type CurrencyId = CurrencyId;
	type RelayChainCurrencyId = RelayChainCurrencyId;
	type Currency = Tokens;
	type ParaId = MockParaId;
	type XCMPMessageSender = MockXCMPMessageSender;
	type UpwardMessageSender = MockUpwardMessageSender;
	type UpwardMessage = MockUpwardMessage;
}
pub type XTokens = Module<Runtime>;

thread_local! {
	static XCMP_MESSAGES: RefCell<Option<(ParaId, XCMPTokenMessage<AccountId, Balance>)>> = RefCell::new(None);
}

pub struct MockXCMPMessageSender;
impl MockXCMPMessageSender {
	pub fn msg_sent(dest: ParaId, msg: XCMPTokenMessage<AccountId, Balance>) -> bool {
		XCMP_MESSAGES.with(|v| v.borrow().clone()) == Some((dest, msg))
	}
}
impl XCMPMessageSender<XCMPTokenMessage<AccountId, Balance>> for MockXCMPMessageSender {
	fn send_xcmp_message(dest: ParaId, msg: &XCMPTokenMessage<AccountId, Balance>) -> Result<(), ()> {
		XCMP_MESSAGES.with(|v| *v.borrow_mut() = Some((dest, msg.clone())));
		Ok(())
	}
}

#[derive(Encode, Decode, Eq, PartialEq, Clone)]
pub struct MockUpwardMessage(pub AccountId, pub Balance);
impl BalancesMessage<AccountId, Balance> for MockUpwardMessage {
	fn transfer(dest: AccountId, amount: Balance) -> Self {
		MockUpwardMessage(dest, amount)
	}
}

thread_local! {
	static UPWARD_MESSAGES: RefCell<Option<MockUpwardMessage>> = RefCell::new(None);
}
pub struct MockUpwardMessageSender;
impl MockUpwardMessageSender {
	pub fn msg_sent(msg: MockUpwardMessage) -> bool {
		UPWARD_MESSAGES.with(|v| v.borrow().clone()) == Some(msg)
	}
}
impl UpwardMessageSender<MockUpwardMessage> for MockUpwardMessageSender {
	fn send_upward_message(msg: &MockUpwardMessage, _origin: UpwardMessageOrigin) -> Result<(), ()> {
		UPWARD_MESSAGES.with(|v| *v.borrow_mut() = Some(msg.clone()));
		Ok(())
	}
}

pub struct BalanceConvertor;
impl Convert<u128, u128> for BalanceConvertor {
	fn convert(x: u128) -> u128 {
		x
	}
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

pub const PARA_ONE_ID: u32 = 1;

pub fn para_one_id() -> ParaId {
	PARA_ONE_ID.into()
}

pub fn para_one_account() -> AccountId {
	para_one_id().into_account()
}

pub const PARA_TWO_ID: u32 = 2;

pub fn para_two_id() -> ParaId {
	PARA_TWO_ID.into()
}

pub fn para_two_account() -> AccountId {
	para_two_id().into_account()
}

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

	pub fn one_hundred_for_alice(self) -> Self {
		self.balances(vec![
			(ALICE, CurrencyId::Owned, 100),
			(ALICE, CurrencyId::DOT, 100),
			(ALICE, CurrencyId::BTC, 100),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		XCMP_MESSAGES.with(|v| *v.borrow_mut() = None);
		UPWARD_MESSAGES.with(|v| *v.borrow_mut() = None);

		t.into()
	}
}
