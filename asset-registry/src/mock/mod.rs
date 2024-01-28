#![cfg(test)]

use super::*;

use mock::para::AssetRegistry;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::bounded::BoundedVec;
use sp_io::TestExternalities;
use sp_runtime::{traits::Convert, AccountId32, BuildStorage};
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};

pub mod para;
pub mod relay;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId32 = AccountId32::new([2u8; 32]);

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
	/// Parachain D token
	D,
	/// Some asset from the asset registry
	RegisteredAsset(u32),
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
			CurrencyId::D => Some(
				(
					Parent,
					Parachain(4),
					Junction::from(BoundedVec::try_from(b"D".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::RegisteredAsset(id) => AssetRegistry::multilocation(&id).unwrap_or_default(),
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
		let d: Vec<u8> = "D".into();
		if l == MultiLocation::parent() {
			return Some(CurrencyId::R);
		}
		let currency_id = match l.clone() {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(1), GeneralKey { data, .. }) if data.to_vec() == a => Some(CurrencyId::A),
				X2(Parachain(1), GeneralKey { data, .. }) if data.to_vec() == a1 => Some(CurrencyId::A1),
				X2(Parachain(2), GeneralKey { data, .. }) if data.to_vec() == b => Some(CurrencyId::B),
				X2(Parachain(2), GeneralKey { data, .. }) if data.to_vec() == b1 => Some(CurrencyId::B1),
				X2(Parachain(2), GeneralKey { data, .. }) if data.to_vec() == b2 => Some(CurrencyId::B2),
				X2(Parachain(4), GeneralKey { data, .. }) if data.to_vec() == d => Some(CurrencyId::D),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey { data, .. }) if data.to_vec() == a => Some(CurrencyId::A),
				X1(GeneralKey { data, .. }) if data.to_vec() == b => Some(CurrencyId::B),
				X1(GeneralKey { data, .. }) if data.to_vec() == a1 => Some(CurrencyId::A1),
				X1(GeneralKey { data, .. }) if data.to_vec() == b1 => Some(CurrencyId::B1),
				X1(GeneralKey { data, .. }) if data.to_vec() == b2 => Some(CurrencyId::B2),
				X1(GeneralKey { data, .. }) if data.to_vec() == d => Some(CurrencyId::D),
				_ => None,
			},
			_ => None,
		};
		currency_id.or_else(|| AssetRegistry::location_to_asset_id(&l).map(CurrencyId::RegisteredAsset))
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
		new_ext = para_ext(1, None),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::MsgQueue,
		DmpMessageHandler = para::MsgQueue,
		new_ext = para_ext(2, None),
	}
}

decl_test_parachain! {
	pub struct ParaC {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::MsgQueue,
		DmpMessageHandler = para::MsgQueue,
		new_ext = para_ext(3, None),
	}
}

decl_test_parachain! {
	pub struct ParaG {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::MsgQueue,
		DmpMessageHandler = para::MsgQueue,
		new_ext = para_ext(4, Some((
			vec![(
				4,
				AssetMetadata::<Balance, para::CustomMetadata, para::StringLimit>::encode(&AssetMetadata {
				decimals: 12,
				name: BoundedVec::truncate_from("para G native token".as_bytes().to_vec()),
				symbol: BoundedVec::truncate_from("paraG".as_bytes().to_vec()),
				existential_deposit: 0,
				location: None,
				additional: para::CustomMetadata {
					fee_per_second: 1_000_000_000_000,
				},
			})),
			(
				5,
				AssetMetadata::<Balance, para::CustomMetadata, para::StringLimit>::encode(&AssetMetadata {
				decimals: 12,
				name: BoundedVec::truncate_from("para G foreign token".as_bytes().to_vec()),
				symbol: BoundedVec::truncate_from("paraF".as_bytes().to_vec()),
				existential_deposit: 0,
				location: None,
				additional: para::CustomMetadata {
					fee_per_second: 1_000_000_000_000,
				},
			}))], 5
		))),
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
			(4, ParaG),
		],
	}
}

pub type ParaTokens = orml_tokens::Pallet<para::Runtime>;
pub type ParaXTokens = orml_xtokens::Pallet<para::Runtime>;

pub fn para_ext(para_id: u32, asset_data: Option<(Vec<(u32, Vec<u8>)>, u32)>) -> TestExternalities {
	use para::{MsgQueue, Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, CurrencyId::R, 1_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	if let Some((assets, last_asset_id)) = asset_data {
		GenesisConfig::<Runtime> { assets, last_asset_id }
			.assimilate_storage(&mut t)
			.unwrap();
	}

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
