#![cfg(test)]

use super::*;
use crate as orml_xtokens;

use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_io::TestExternalities;
use sp_runtime::{AccountId32, BoundedVec, BuildStorage};
use xcm_builder::{CreateMatcher, MatchXcm};
use xcm_executor::traits::{ShouldExecute, WeightTrader};
use xcm_executor::Assets;

use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, ProcessMessageError, TestExt};

pub mod para;
pub mod para_relative_view;
pub mod para_teleport;
pub mod relay;
pub mod teleport_currency_adapter;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	parity_scale_codec::MaxEncodedLen,
	TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	/// Relay chain token.
	R,
	/// Parachain A token.
	A,
	/// Parachain A A1 token.
	A1,
	/// Parachain B token.
	B,
	/// Parachain B B1 token
	B1,
	/// Parachain B B2 token
	B2,
	/// Parachain C token
	C,
	/// Parachain D token
	D,
}

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		match id {
			CurrencyId::R => Some(Parent.into()),
			CurrencyId::A => Some(
				(
					Parent,
					Parachain(1),
					Junction::from(BoundedVec::try_from(b"A".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::A1 => Some(
				(
					Parent,
					Parachain(1),
					Junction::from(BoundedVec::try_from(b"A1".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::B => Some(
				(
					Parent,
					Parachain(2),
					Junction::from(BoundedVec::try_from(b"B".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::B1 => Some(
				(
					Parent,
					Parachain(2),
					Junction::from(BoundedVec::try_from(b"B1".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::B2 => Some(
				(
					Parent,
					Parachain(2),
					Junction::from(BoundedVec::try_from(b"B2".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::C => Some(
				(
					Parent,
					Parachain(3),
					Junction::from(BoundedVec::try_from(b"C".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::D => Some(
				(
					Parent,
					Parachain(4),
					Junction::from(BoundedVec::try_from(b"D".to_vec()).unwrap()),
				)
					.into(),
			),
		}
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(l: MultiLocation) -> Option<CurrencyId> {
		let mut a: Vec<u8> = "A".into();
		a.resize(32, 0);
		let mut a1: Vec<u8> = "A1".into();
		a1.resize(32, 0);
		let mut b: Vec<u8> = "B".into();
		b.resize(32, 0);
		let mut b1: Vec<u8> = "B1".into();
		b1.resize(32, 0);
		let mut b2: Vec<u8> = "B2".into();
		b2.resize(32, 0);
		let mut c: Vec<u8> = "C".into();
		c.resize(32, 0);
		let mut d: Vec<u8> = "D".into();
		d.resize(32, 0);
		if l == MultiLocation::parent() {
			return Some(CurrencyId::R);
		}
		match l {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(1), GeneralKey { data, .. }) if data.to_vec() == a => Some(CurrencyId::A),
				X2(Parachain(1), GeneralKey { data, .. }) if data.to_vec() == a1 => Some(CurrencyId::A1),
				X2(Parachain(2), GeneralKey { data, .. }) if data.to_vec() == b => Some(CurrencyId::B),
				X2(Parachain(2), GeneralKey { data, .. }) if data.to_vec() == b1 => Some(CurrencyId::B1),
				X2(Parachain(2), GeneralKey { data, .. }) if data.to_vec() == b2 => Some(CurrencyId::B2),
				X2(Parachain(3), GeneralKey { data, .. }) if data.to_vec() == c => Some(CurrencyId::C),
				X2(Parachain(4), GeneralKey { data, .. }) if data.to_vec() == d => Some(CurrencyId::D),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey { data, .. }) if data.to_vec() == a => Some(CurrencyId::A),
				X1(GeneralKey { data, .. }) if data.to_vec() == b => Some(CurrencyId::B),
				X1(GeneralKey { data, .. }) if data.to_vec() == a1 => Some(CurrencyId::A1),
				X1(GeneralKey { data, .. }) if data.to_vec() == b1 => Some(CurrencyId::B1),
				X1(GeneralKey { data, .. }) if data.to_vec() == b2 => Some(CurrencyId::B2),
				X1(GeneralKey { data, .. }) if data.to_vec() == c => Some(CurrencyId::C),
				X1(GeneralKey { data, .. }) if data.to_vec() == d => Some(CurrencyId::D),
				_ => None,
			},
			_ => None,
		}
	}
}
impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(a: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset {
			fun: Fungible(_),
			id: Concrete(id),
		} = a
		{
			Self::convert(id)
		} else {
			Option::None
		}
	}
}

pub type Balance = u128;
pub type Amount = i128;

decl_test_parachain! {
	pub struct ParaA {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::MsgQueue,
		DmpMessageHandler = para::MsgQueue,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::MsgQueue,
		DmpMessageHandler = para::MsgQueue,
		new_ext = para_ext(2),
	}
}

decl_test_parachain! {
	pub struct ParaC {
		Runtime = para_teleport::Runtime,
		XcmpMessageHandler = para_teleport::MsgQueue,
		DmpMessageHandler = para_teleport::MsgQueue,
		new_ext = para_teleport_ext(3),
	}
}

// This parachain is identical to the others but using relative view for self
// tokens
decl_test_parachain! {
	pub struct ParaD {
		Runtime = para_relative_view::Runtime,
		XcmpMessageHandler = para::MsgQueue,
		DmpMessageHandler = para::MsgQueue,
		new_ext = para_ext(4),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay::Runtime,
		RuntimeCall = relay::RuntimeCall,
		RuntimeEvent = relay::RuntimeEvent,
		XcmConfig = relay::XcmConfig,
		MessageQueue = relay::MessageQueue,
		System = relay::System,
		new_ext = relay_ext(),
	}
}

decl_test_network! {
	pub struct TestNet {
		relay_chain = Relay,
		parachains = vec![
			(1, ParaA),
			(2, ParaB),
			(3, ParaC),
			(4, ParaD),
		],
	}
}

pub type RelayBalances = pallet_balances::Pallet<relay::Runtime>;
pub type ParaTokens = orml_tokens::Pallet<para::Runtime>;
pub type ParaXTokens = orml_xtokens::Pallet<para::Runtime>;

pub type ParaRelativeTokens = orml_tokens::Pallet<para_relative_view::Runtime>;
pub type ParaRelativeXTokens = orml_xtokens::Pallet<para_relative_view::Runtime>;

pub type ParaTeleportTokens = orml_tokens::Pallet<para_teleport::Runtime>;

pub fn para_ext(para_id: u32) -> TestExternalities {
	use para::{MsgQueue, Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, CurrencyId::R, 1_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		MsgQueue::set_para_id(para_id.into());
	});
	ext
}

pub fn para_teleport_ext(para_id: u32) -> TestExternalities {
	use para_teleport::{MsgQueue, Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, CurrencyId::R, 1_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		MsgQueue::set_para_id(para_id.into());
	});
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay::{Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, 1_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// A trader who believes all tokens are created equal to "weight" of any chain,
/// which is not true, but good enough to mock the fee payment of XCM execution.
///
/// This mock will always trade `n` amount of weight to `n` amount of tokens.
pub struct AllTokensAreCreatedEqualToWeight(MultiLocation);
impl WeightTrader for AllTokensAreCreatedEqualToWeight {
	fn new() -> Self {
		Self(MultiLocation::parent())
	}

	fn buy_weight(&mut self, weight: Weight, payment: Assets, _context: &XcmContext) -> Result<Assets, XcmError> {
		let asset_id = payment
			.fungible
			.iter()
			.next()
			.expect("Payment must be something; qed")
			.0;
		let required = MultiAsset {
			id: asset_id.clone(),
			fun: Fungible(weight.ref_time() as u128),
		};

		if let MultiAsset {
			fun: _,
			id: Concrete(ref id),
		} = &required
		{
			self.0 = id.clone();
		}

		let unused = payment.checked_sub(required).map_err(|_| XcmError::TooExpensive)?;
		Ok(unused)
	}

	fn refund_weight(&mut self, weight: Weight, _context: &XcmContext) -> Option<MultiAsset> {
		if weight.is_zero() {
			None
		} else {
			Some((self.0.clone(), weight.ref_time() as u128).into())
		}
	}
}

/// Allows execution from all origins taking payment into account.
///
/// Only allows for `TeleportAsset`, `WithdrawAsset`, `ClaimAsset` and
/// `ReserveAssetDeposit` XCMs because they are the only ones that place assets
/// in the Holding Register to pay for execution. This is almost equal to
/// [`xcm_builder::AllowTopLevelPaidExecutionFrom<T>`] except that it allows for
/// multiple assets and is not generic to allow all origins.
/// This is necessary after the change in `polkadot-sdk` which prevents
/// multicurrency transfers. Here is the relevant issue: https://github.com/paritytech/polkadot-sdk/issues/1638
pub struct AllowTopLevelPaidExecution;
impl ShouldExecute for AllowTopLevelPaidExecution {
	fn should_execute<RuntimeCall>(
		_origin: &MultiLocation,
		instructions: &mut [Instruction<RuntimeCall>],
		max_weight: Weight,
		_properties: &mut xcm_executor::traits::Properties,
	) -> Result<(), ProcessMessageError> {
		let end = instructions.len().min(5);
		instructions[..end]
			.matcher()
			.match_next_inst(|inst| match inst {
				ReceiveTeleportedAsset(..) | ReserveAssetDeposited(..) => Ok(()),
				WithdrawAsset(..) => Ok(()),
				ClaimAsset { .. } => Ok(()),
				_ => Err(ProcessMessageError::BadFormat),
			})?
			.skip_inst_while(|inst| matches!(inst, ClearOrigin))?
			.match_next_inst(|inst| {
				let res = match inst {
					BuyExecution {
						weight_limit: Limited(ref mut weight),
						..
					} if weight.all_gte(max_weight) => {
						*weight = max_weight;
						Ok(())
					}
					BuyExecution {
						ref mut weight_limit, ..
					} if weight_limit == &Unlimited => {
						*weight_limit = Limited(max_weight);
						Ok(())
					}
					_ => Err(ProcessMessageError::Overweight(max_weight)),
				};
				res
			})?;

		Ok(())
	}
}
