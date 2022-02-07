#![cfg(test)]

use super::*;
use crate as orml_xtokens;

use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_io::TestExternalities;
use sp_runtime::AccountId32;

use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub mod para;
pub mod relay;

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
		}
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(l: MultiLocation) -> Option<CurrencyId> {
		let a: Vec<u8> = "A".into();
		let a1: Vec<u8> = "A1".into();
		let b: Vec<u8> = "B".into();
		let b1: Vec<u8> = "B1".into();
		if l == MultiLocation::parent() {
			return Some(CurrencyId::R);
		}
		match l {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(1), GeneralKey(k)) if k == a => Some(CurrencyId::A),
				X2(Parachain(1), GeneralKey(k)) if k == a1 => Some(CurrencyId::A1),
				X2(Parachain(2), GeneralKey(k)) if k == b => Some(CurrencyId::B),
				X2(Parachain(2), GeneralKey(k)) if k == b1 => Some(CurrencyId::B1),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey(k)) if k == a => Some(CurrencyId::A),
				X1(GeneralKey(k)) if k == b => Some(CurrencyId::B),
				X1(GeneralKey(k)) if k == a1 => Some(CurrencyId::A1),
				X1(GeneralKey(k)) if k == b1 => Some(CurrencyId::B1),
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
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(3),
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
		],
	}
}

pub type RelayBalances = pallet_balances::Pallet<relay::Runtime>;
pub type ParaTokens = orml_tokens::Pallet<para::Runtime>;
pub type ParaXTokens = orml_xtokens::Pallet<para::Runtime>;

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
