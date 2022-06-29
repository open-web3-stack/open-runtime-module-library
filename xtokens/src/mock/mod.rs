#![cfg(test)]

use super::*;
use crate as orml_xtokens;

use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_io::TestExternalities;
use sp_runtime::AccountId32;
use xcm_executor::traits::WeightTrader;
use xcm_executor::Assets;

use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub mod para;
pub mod para_relative_view;
pub mod para_teleport;
pub mod relay;
pub mod teleport_currency_adapter;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, codec::MaxEncodedLen, TypeInfo)]
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
			CurrencyId::A => Some((Parent, Parachain(1), GeneralKey("A".into())).into()),
			CurrencyId::A1 => Some((Parent, Parachain(1), GeneralKey("A1".into())).into()),
			CurrencyId::B => Some((Parent, Parachain(2), GeneralKey("B".into())).into()),
			CurrencyId::B1 => Some((Parent, Parachain(2), GeneralKey("B1".into())).into()),
			CurrencyId::B2 => Some((Parent, Parachain(2), GeneralKey("B2".into())).into()),
			CurrencyId::C => Some((Parent, Parachain(3), GeneralKey("C".into())).into()),
			CurrencyId::D => Some((Parent, Parachain(4), GeneralKey("D".into())).into()),
		}
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(l: MultiLocation) -> Option<CurrencyId> {
		let a: Vec<u8> = "A".into();
		let a1: Vec<u8> = "A1".into();
		let b: Vec<u8> = "B".into();
		let b1: Vec<u8> = "B1".into();
		let b2: Vec<u8> = "B2".into();
		let c: Vec<u8> = "C".into();
		let d: Vec<u8> = "D".into();
		if l == MultiLocation::parent() {
			return Some(CurrencyId::R);
		}
		match l {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(1), GeneralKey(k)) if k == a => Some(CurrencyId::A),
				X2(Parachain(1), GeneralKey(k)) if k == a1 => Some(CurrencyId::A1),
				X2(Parachain(2), GeneralKey(k)) if k == b => Some(CurrencyId::B),
				X2(Parachain(2), GeneralKey(k)) if k == b1 => Some(CurrencyId::B1),
				X2(Parachain(2), GeneralKey(k)) if k == b2 => Some(CurrencyId::B2),
				X2(Parachain(3), GeneralKey(k)) if k == c => Some(CurrencyId::C),
				X2(Parachain(4), GeneralKey(k)) if k == d => Some(CurrencyId::D),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey(k)) if k == a => Some(CurrencyId::A),
				X1(GeneralKey(k)) if k == b => Some(CurrencyId::B),
				X1(GeneralKey(k)) if k == a1 => Some(CurrencyId::A1),
				X1(GeneralKey(k)) if k == b1 => Some(CurrencyId::B1),
				X1(GeneralKey(k)) if k == b2 => Some(CurrencyId::B2),
				X1(GeneralKey(k)) if k == c => Some(CurrencyId::C),
				X1(GeneralKey(k)) if k == d => Some(CurrencyId::D),
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
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(1),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(2),
	}
}

decl_test_parachain! {
	pub struct ParaC {
		Runtime = para_teleport::Runtime,
		XcmpMessageHandler = para_teleport::XcmpQueue,
		DmpMessageHandler = para_teleport::DmpQueue,
		new_ext = para_teleport_ext(3),
	}
}

// This parachain is identical to the others but using relative view for self
// tokens
decl_test_parachain! {
	pub struct ParaD {
		Runtime = para_relative_view::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(4),
	}
}

decl_test_relay_chain! {
	pub struct Relay {
		Runtime = relay::Runtime,
		XcmConfig = relay::XcmConfig,
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
	use para::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	let parachain_info_config = parachain_info::GenesisConfig {
		parachain_id: para_id.into(),
	};
	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, CurrencyId::R, 1_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn para_teleport_ext(para_id: u32) -> TestExternalities {
	use para_teleport::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	let parachain_info_config = parachain_info::GenesisConfig {
		parachain_id: para_id.into(),
	};
	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, CurrencyId::R, 1_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn relay_ext() -> sp_io::TestExternalities {
	use relay::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
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

	fn buy_weight(&mut self, weight: Weight, payment: Assets) -> Result<Assets, XcmError> {
		let asset_id = payment
			.fungible
			.iter()
			.next()
			.expect("Payment must be something; qed")
			.0;
		let required = MultiAsset {
			id: asset_id.clone(),
			fun: Fungible(weight as u128),
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

	fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
		if weight.is_zero() {
			None
		} else {
			Some((self.0.clone(), weight as u128).into())
		}
	}
}
