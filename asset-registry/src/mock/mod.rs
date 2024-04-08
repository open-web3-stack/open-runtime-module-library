#![cfg(test)]

use super::*;

use mock::para::AssetRegistry;
use sp_io::TestExternalities;
use sp_runtime::{traits::Convert, AccountId32, BuildStorage};
use xcm_simulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};

pub mod para;
pub mod relay;

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);

pub type CurrencyId = u32;

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		match id {
			0 => Some(Parent.into()),
			_ => AssetRegistry::multilocation(&id).unwrap_or_default(),
		}
	}
}
impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(l: MultiLocation) -> Option<CurrencyId> {
		if l == MultiLocation::parent() {
			return Some(0);
		}

		if let Some(asset_id) = AssetRegistry::location_to_asset_id(&l) {
			return Some(asset_id);
		}
		None
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
		new_ext = para_ext(1, None),
	}
}

decl_test_parachain! {
	pub struct ParaB {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(2, None),
	}
}

decl_test_parachain! {
	pub struct ParaC {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(3, None),
	}
}

decl_test_parachain! {
	pub struct ParaG {
		Runtime = para::Runtime,
		XcmpMessageHandler = para::XcmpQueue,
		DmpMessageHandler = para::DmpQueue,
		new_ext = para_ext(4, Some((
			vec![(
				0,
				AssetMetadata::<Balance, para::CustomMetadata, para::StringLimit>::encode(&AssetMetadata {
				decimals: 12,
				name: BoundedVec::truncate_from("para G native token".as_bytes().to_vec()),
				symbol: BoundedVec::truncate_from("paraG".as_bytes().to_vec()),
				existential_deposit: 0,
				location: None,
				additional: para::CustomMetadata {
					fee_per_second: 1_000_000_000_000,
				},
			}), None),
			(
				1,
				AssetMetadata::<Balance, para::CustomMetadata, para::StringLimit>::encode(&AssetMetadata {
				decimals: 12,
				name: BoundedVec::truncate_from("para G foreign token".as_bytes().to_vec()),
				symbol: BoundedVec::truncate_from("paraF".as_bytes().to_vec()),
				existential_deposit: 0,
				location: None,
				additional: para::CustomMetadata {
					fee_per_second: 1_000_000_000_000,
				},
			}), None),
			(
				2,
				AssetMetadata::<Balance, para::CustomMetadata, para::StringLimit>::encode(&AssetMetadata {
				decimals: 12,
				name: BoundedVec::truncate_from("para G native token 2".as_bytes().to_vec()),
				symbol: BoundedVec::truncate_from("paraG2".as_bytes().to_vec()),
				existential_deposit: 0,
				location: None,
				additional: para::CustomMetadata {
					fee_per_second: 1_000_000_000_000,
				},
			}), Some(L1Asset::Ethereum(array_bytes::hex2array("0x0123456789012345678901234567890123456789").unwrap())))], 5
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

pub fn para_ext(para_id: u32, asset_data: Option<(Vec<(u32, Vec<u8>, Option<L1Asset>)>, u32)>) -> TestExternalities {
	use para::{Runtime, System};

	let mut t = frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap();

	let parachain_info_config = parachain_info::GenesisConfig::<Runtime> {
		_config: Default::default(),
		parachain_id: para_id.into(),
	};
	parachain_info_config.assimilate_storage(&mut t).unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		tokens_endowment: vec![(ALICE, 0, 1_000)],
		created_tokens_for_staking: vec![],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	if let Some((assets, _)) = asset_data {
		GenesisConfig::<Runtime> { assets: assets }
			.assimilate_storage(&mut t)
			.unwrap();
	}

	let mut ext = TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
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
