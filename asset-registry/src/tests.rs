#![cfg(test)]

use super::*;
use crate as orml_asset_registry;
use crate::tests::para::{AdminAssetTwo, AssetRegistry, CustomMetadata, RuntimeOrigin, Tokens, TreasuryAccount};
use frame_support::{assert_noop, assert_ok};
use mock::{para::RuntimeCall, *};
use orml_traits::MultiCurrency;
use polkadot_parachain_primitives::primitives::Sibling;

use sp_runtime::{
	traits::{AccountIdConversion, BadOrigin, Dispatchable},
	AccountId32,
};
use xcm_simulator::TestExt;

type OldMultiLocation = xcm::v2::MultiLocation;
type OldJunctions = xcm::v2::Junctions;
type OldJunction = xcm::v2::Junction;

fn treasury_account() -> AccountId32 {
	TreasuryAccount::get()
}

fn sibling_a_account() -> AccountId32 {
	Sibling::from(1).into_account_truncating()
}

fn sibling_b_account() -> AccountId32 {
	Sibling::from(2).into_account_truncating()
}

fn sibling_c_account() -> AccountId32 {
	Sibling::from(3).into_account_truncating()
}

// Not used in any unit tests, but it's super helpful for debugging. Let's
// keep it here.
#[allow(dead_code)]
fn print_events<Runtime: frame_system::Config>(name: &'static str) {
	println!("------ {:?} events -------", name);
	frame_system::Pallet::<Runtime>::events()
		.iter()
		.for_each(|r| println!("> {:?}", r.event));
}

fn dummy_metadata() -> AssetMetadata<
	<para::Runtime as orml_asset_registry::Config>::Balance,
	CustomMetadata,
	<para::Runtime as orml_asset_registry::Config>::StringLimit,
> {
	AssetMetadata {
		decimals: 12,
		name: BoundedVec::truncate_from("para A native token".as_bytes().to_vec()),
		symbol: BoundedVec::truncate_from("paraA".as_bytes().to_vec()),
		existential_deposit: 0,
		location: Some(
			MultiLocation::new(
				1,
				X2(Parachain(1), Junction::from(BoundedVec::try_from(vec![0]).unwrap())),
			)
			.into(),
		),
		additional: CustomMetadata {
			fee_per_second: 1_000_000_000_000,
		},
	}
}

#[test]
fn genesis_issuance_should_work() {
	TestNet::reset();

	ParaG::execute_with(|| {
		let metadata1 = AssetMetadata {
			decimals: 12,
			name: BoundedVec::truncate_from("para G native token".as_bytes().to_vec()),
			symbol: BoundedVec::truncate_from("paraG".as_bytes().to_vec()),
			existential_deposit: 0,
			location: None,
			additional: CustomMetadata {
				fee_per_second: 1_000_000_000_000,
			},
		};
		let metadata2 = AssetMetadata {
			decimals: 12,
			name: BoundedVec::truncate_from("para G foreign token".as_bytes().to_vec()),
			symbol: BoundedVec::truncate_from("paraF".as_bytes().to_vec()),
			existential_deposit: 0,
			location: None,
			additional: CustomMetadata {
				fee_per_second: 1_000_000_000_000,
			},
		};
		assert_eq!(AssetRegistry::metadata(4).unwrap(), metadata1);
		assert_eq!(AssetRegistry::metadata(5).unwrap(), metadata2);
		assert_eq!(LastAssetId::<para::Runtime>::get(), 5);
	});
}

#[test]
/// test that the asset registry can be used in xcm transfers
fn send_self_parachain_asset_to_sibling() {
	TestNet::reset();

	let mut metadata = dummy_metadata();

	ParaB::execute_with(|| {
		AssetRegistry::register_asset(RuntimeOrigin::root(), metadata.clone(), None).unwrap();
	});

	ParaA::execute_with(|| {
		metadata.location =
			Some(MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(vec![0]).unwrap()))).into());
		AssetRegistry::register_asset(RuntimeOrigin::root(), metadata, None).unwrap();

		assert_ok!(ParaTokens::deposit(CurrencyId::RegisteredAsset(1), &ALICE, 1_000));

		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::RegisteredAsset(1),
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &ALICE), 500);
		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &sibling_b_account()),
			500
		);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &BOB), 460);
		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &treasury_account()),
			40
		);
	});
}

#[test]
/// test that the asset registry can be used in xcm transfers
fn send_sibling_asset_to_non_reserve_sibling() {
	TestNet::reset();

	// send from paraA send paraB's token to paraC

	ParaA::execute_with(|| {
		AssetRegistry::register_asset(
			RuntimeOrigin::root(),
			AssetMetadata {
				location: Some(
					MultiLocation::new(
						1,
						X2(Parachain(2), Junction::from(BoundedVec::try_from(vec![0]).unwrap())),
					)
					.into(),
				),
				..dummy_metadata()
			},
			None,
		)
		.unwrap();
		assert_ok!(ParaTokens::deposit(CurrencyId::RegisteredAsset(1), &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		AssetRegistry::register_asset(
			RuntimeOrigin::root(),
			AssetMetadata {
				location: Some(
					MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(vec![0]).unwrap()))).into(),
				),
				..dummy_metadata()
			},
			None,
		)
		.unwrap();
		assert_ok!(ParaTokens::deposit(
			CurrencyId::RegisteredAsset(1),
			&sibling_a_account(),
			1_000
		));
	});

	ParaC::execute_with(|| {
		AssetRegistry::register_asset(
			RuntimeOrigin::root(),
			AssetMetadata {
				location: Some(
					MultiLocation::new(
						1,
						X2(Parachain(2), Junction::from(BoundedVec::try_from(vec![0]).unwrap())),
					)
					.into(),
				),
				..dummy_metadata()
			},
			None,
		)
		.unwrap();
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::RegisteredAsset(1),
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(3),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &ALICE), 500);
	});

	// check reserve accounts
	ParaB::execute_with(|| {
		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &sibling_a_account()),
			500
		);
		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &sibling_c_account()),
			460
		);
	});

	ParaC::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &BOB), 420);
	});
}

#[test]
/// tests the SequentialId AssetProcessor
fn test_sequential_id_normal_behavior() {
	TestNet::reset();

	ParaA::execute_with(|| {
		let metadata1 = dummy_metadata();

		let metadata2 = AssetMetadata {
			name: BoundedVec::truncate_from("para A native token 2".as_bytes().to_vec()),
			symbol: BoundedVec::truncate_from("paraA2".as_bytes().to_vec()),
			location: Some(
				MultiLocation::new(
					1,
					X2(Parachain(1), Junction::from(BoundedVec::try_from(vec![1]).unwrap())),
				)
				.into(),
			),
			..dummy_metadata()
		};
		AssetRegistry::register_asset(RuntimeOrigin::root(), metadata1.clone(), None).unwrap();
		AssetRegistry::register_asset(RuntimeOrigin::root(), metadata2.clone(), None).unwrap();

		assert_eq!(AssetRegistry::metadata(1).unwrap(), metadata1);
		assert_eq!(AssetRegistry::metadata(2).unwrap(), metadata2);
	});
}

#[test]
fn test_sequential_id_with_invalid_id_returns_error() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(AssetRegistry::register_asset(
			RuntimeOrigin::root(),
			dummy_metadata(),
			Some(1)
		));
		assert_noop!(
			AssetRegistry::register_asset(RuntimeOrigin::root(), dummy_metadata(), Some(1)),
			Error::<para::Runtime>::InvalidAssetId
		);
	});
}

#[test]
/// tests FixedRateAssetRegistryTrader
fn test_fixed_rate_asset_trader() {
	TestNet::reset();

	let metadata = dummy_metadata();

	ParaB::execute_with(|| {
		AssetRegistry::register_asset(RuntimeOrigin::root(), metadata.clone(), None).unwrap();
	});

	ParaA::execute_with(|| {
		let para_a_metadata = AssetMetadata {
			location: Some(MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(vec![0]).unwrap()))).into()),
			..metadata.clone()
		};
		AssetRegistry::register_asset(RuntimeOrigin::root(), para_a_metadata, None).unwrap();

		assert_ok!(ParaTokens::deposit(CurrencyId::RegisteredAsset(1), &ALICE, 1_000));

		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::RegisteredAsset(1),
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited,
		));
	});

	let expected_fee = 40;
	let expected_transfer_1_amount = 500 - expected_fee;
	ParaB::execute_with(|| {
		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &BOB),
			expected_transfer_1_amount
		);

		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &treasury_account()),
			expected_fee
		);

		// now double the fee rate
		AssetRegistry::update_asset(
			RuntimeOrigin::root(),
			1,
			None,
			None,
			None,
			None,
			None,
			Some(CustomMetadata {
				fee_per_second: metadata.additional.fee_per_second * 2,
			}),
		)
		.unwrap();
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::RegisteredAsset(1),
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited,
		));
	});

	// we doubled the fee rate, so subtract twice the original fee
	let expected_transfer_2_amount = 500 - 2 * expected_fee;

	ParaB::execute_with(|| {
		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &BOB),
			expected_transfer_1_amount + expected_transfer_2_amount
		);

		assert_eq!(
			ParaTokens::free_balance(CurrencyId::RegisteredAsset(1), &treasury_account()),
			expected_fee * 3 // 1 for the first transfer, then twice for the second one
		);
	});
}

#[test]
fn test_register_duplicate_location_returns_error() {
	TestNet::reset();

	ParaA::execute_with(|| {
		let metadata = dummy_metadata();

		assert_ok!(AssetRegistry::register_asset(
			RuntimeOrigin::root(),
			metadata.clone(),
			None
		));
		let register_asset = RuntimeCall::AssetRegistry(crate::Call::<para::Runtime>::register_asset {
			metadata,
			asset_id: None,
		});
		assert_noop!(
			register_asset.dispatch(RuntimeOrigin::root()),
			Error::<para::Runtime>::ConflictingLocation
		);
	});
}

#[test]
fn test_register_duplicate_asset_id_returns_error() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(AssetRegistry::register_asset(
			RuntimeOrigin::root(),
			dummy_metadata(),
			Some(1)
		));
		assert_noop!(
			AssetRegistry::do_register_asset_without_asset_processor(dummy_metadata(), 1),
			Error::<para::Runtime>::ConflictingAssetId
		);
	});
}

#[test]
fn test_update_metadata_works() {
	TestNet::reset();

	ParaA::execute_with(|| {
		let old_metadata = dummy_metadata();
		assert_ok!(AssetRegistry::register_asset(
			RuntimeOrigin::root(),
			old_metadata.clone(),
			None
		));

		let new_metadata = AssetMetadata {
			decimals: 11,
			name: BoundedVec::truncate_from("para A native token2".as_bytes().to_vec()),
			symbol: BoundedVec::truncate_from("paraA2".as_bytes().to_vec()),
			existential_deposit: 1,
			location: Some(
				MultiLocation::new(
					1,
					X2(Parachain(1), Junction::from(BoundedVec::try_from(vec![1]).unwrap())),
				)
				.into(),
			),
			additional: CustomMetadata {
				fee_per_second: 2_000_000_000_000,
			},
		};
		assert_ok!(AssetRegistry::update_asset(
			RuntimeOrigin::root(),
			1,
			Some(new_metadata.decimals),
			Some(new_metadata.name.clone()),
			Some(new_metadata.symbol.clone()),
			Some(new_metadata.existential_deposit),
			Some(new_metadata.location.clone()),
			Some(new_metadata.additional.clone())
		));

		let old_location: MultiLocation = old_metadata.location.unwrap().try_into().unwrap();
		let new_location: MultiLocation = new_metadata.location.clone().unwrap().try_into().unwrap();

		// check that the old location was removed and the new one added
		assert_eq!(AssetRegistry::location_to_asset_id(old_location), None);
		assert_eq!(AssetRegistry::location_to_asset_id(new_location), Some(1));

		assert_eq!(AssetRegistry::metadata(1).unwrap(), new_metadata);
	});
}

#[test]
fn test_update_metadata_fails_with_unknown_asset() {
	TestNet::reset();

	ParaA::execute_with(|| {
		let old_metadata = dummy_metadata();
		assert_ok!(AssetRegistry::register_asset(RuntimeOrigin::root(), old_metadata, None));

		assert_noop!(
			AssetRegistry::update_asset(RuntimeOrigin::root(), 4, None, None, None, None, None, None,),
			Error::<para::Runtime>::AssetNotFound
		);
	});
}

#[test]
fn test_existential_deposits() {
	TestNet::reset();

	ParaA::execute_with(|| {
		let metadata = AssetMetadata {
			existential_deposit: 100,
			..dummy_metadata()
		};
		assert_ok!(AssetRegistry::register_asset(RuntimeOrigin::root(), metadata, None));

		assert_ok!(Tokens::set_balance(
			RuntimeOrigin::root(),
			ALICE,
			CurrencyId::RegisteredAsset(1),
			1_000,
			0
		));

		// transferring at existential_deposit succeeds
		assert_ok!(Tokens::transfer(
			Some(ALICE).into(),
			BOB,
			CurrencyId::RegisteredAsset(1),
			100
		));
		// transferring below existential_deposit fails
		assert_noop!(
			Tokens::transfer(Some(ALICE).into(), CHARLIE, CurrencyId::RegisteredAsset(1), 50),
			orml_tokens::Error::<para::Runtime>::ExistentialDeposit
		);
	});
}

#[test]
fn test_asset_authority() {
	TestNet::reset();

	ParaA::execute_with(|| {
		let metadata = dummy_metadata();

		// Assert that root can register an asset with id 1
		assert_ok!(AssetRegistry::register_asset(RuntimeOrigin::root(), metadata, Some(1)));

		// Assert that only Account42 can register asset with id 42
		let metadata = AssetMetadata {
			location: None,
			..dummy_metadata()
		};

		// It fails when signed with root...
		assert_noop!(
			AssetRegistry::register_asset(RuntimeOrigin::root(), metadata.clone(), Some(2)),
			BadOrigin
		);
		// It works when signed with the right account
		assert_ok!(AssetRegistry::register_asset(
			RuntimeOrigin::signed(AdminAssetTwo::get()),
			metadata,
			Some(2)
		));
	});
}

#[test]
fn test_v2_to_v3_incompatible_multilocation() {
	// Assert that V2 and V3 Multilocation both are encoded differently
	assert!(
		OldMultiLocation::new(
			0,
			OldJunctions::X1(OldJunction::GeneralKey(vec![0].try_into().unwrap()))
		)
		.encode() != MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(vec![0]).unwrap()))).encode()
	);
}

#[test]
fn test_decode_bounded_vec() {
	TestNet::reset();

	pub mod unbounded {
		use super::*;

		#[frame_support::storage_alias]
		pub type Metadata<T: orml_asset_registry::Config> = StorageMap<
			orml_asset_registry::Pallet<T>,
			Twox64Concat,
			<T as orml_asset_registry::Config>::AssetId,
			AssetMetadata<
				<T as orml_asset_registry::Config>::Balance,
				<T as orml_asset_registry::Config>::CustomMetadata,
			>,
			OptionQuery,
		>;

		#[derive(TypeInfo, Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
		pub struct AssetMetadata<Balance, CustomMetadata: Parameter + Member + TypeInfo> {
			pub decimals: u32,
			pub name: Vec<u8>,
			pub symbol: Vec<u8>,
			pub existential_deposit: Balance,
			pub location: Option<VersionedMultiLocation>,
			pub additional: CustomMetadata,
		}
	}

	ParaA::execute_with(|| {
		let para_name = "para A native token".as_bytes().to_vec();
		let para_symbol = "paraA".as_bytes().to_vec();
		unbounded::Metadata::<para::Runtime>::insert(
			0,
			unbounded::AssetMetadata {
				decimals: 12,
				name: para_name.clone(),
				symbol: para_symbol.clone(),
				existential_deposit: 0,
				location: None,
				additional: CustomMetadata {
					fee_per_second: 1_000_000_000_000,
				},
			},
		);

		let asset_metadata = Metadata::<para::Runtime>::get(0);
		assert_eq!(
			asset_metadata.map(|m| (m.name.to_vec(), m.symbol.to_vec())),
			Some((para_name, para_symbol))
		);
	});
}
