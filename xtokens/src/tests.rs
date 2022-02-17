#![cfg(test)]

use super::*;
use codec::Encode;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_err, assert_noop, assert_ok, traits::Currency};
use mock::*;
use orml_traits::{ConcreteFungibleAsset, MultiCurrency};
use polkadot_parachain::primitives::{AccountIdConversion, Sibling};
use sp_runtime::AccountId32;
use xcm_simulator::TestExt;

fn para_a_account() -> AccountId32 {
	ParaId::from(1).into_account()
}

fn para_b_account() -> AccountId32 {
	ParaId::from(2).into_account()
}

fn sibling_a_account() -> AccountId32 {
	use sp_runtime::traits::AccountIdConversion;
	Sibling::from(1).into_account()
}

fn sibling_b_account() -> AccountId32 {
	use sp_runtime::traits::AccountIdConversion;
	Sibling::from(2).into_account()
}

fn sibling_c_account() -> AccountId32 {
	use sp_runtime::traits::AccountIdConversion;
	Sibling::from(3).into_account()
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

#[test]
fn send_relay_chain_asset_to_relay_chain() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X1(Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					})
				)
				.into()
			),
			40,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&BOB), 460);
	});
}

#[test]
fn send_relay_chain_asset_to_relay_chain_with_fee() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_with_fee(
			Some(ALICE).into(),
			CurrencyId::R,
			450,
			50,
			Box::new(
				MultiLocation::new(
					1,
					X1(Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					})
				)
				.into()
			),
			40,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	// It should use 40 for weight, so 460 should reach destination
	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&BOB), 460);
	});
}

#[test]
fn cannot_lost_fund_on_send_failed() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));
		assert_noop!(
			ParaXTokens::transfer(
				Some(ALICE).into(),
				CurrencyId::A,
				500,
				Box::new(
					(
						Parent,
						Parachain(100),
						Junction::AccountId32 {
							network: NetworkId::Kusama,
							id: BOB.into(),
						},
					)
						.into()
				),
				40,
			),
			Error::<para::Runtime>::XcmExecutionFailed
		);

		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 1_000);
	});
}

#[test]
fn send_relay_chain_asset_to_sibling() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1000);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&para_b_account()), 460);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &BOB), 420);
	});
}

#[test]
fn send_relay_chain_asset_to_sibling_with_fee() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1000);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_with_fee(
			Some(ALICE).into(),
			CurrencyId::R,
			410,
			90,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	// It should use 40 weight
	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&para_b_account()), 460);
	});

	// It should use another 40 weight in paraB
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &BOB), 420);
	});
}

#[test]
fn send_sibling_asset_to_reserve_sibling() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &sibling_b_account(), 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &BOB, 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::B,
			500,
			Box::new(
				(
					Parent,
					Parachain(2),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					},
				)
					.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 460);

		assert_ok!(ParaXTokens::transfer(
			Some(BOB).into(),
			CurrencyId::A,
			500,
			Box::new(
				(
					Parent,
					Parachain(1),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: ALICE.into(),
					},
				)
					.into()
			),
			40,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 500);
	});

	ParaA::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 460);
	});
}

#[test]
fn send_sibling_asset_to_reserve_sibling_with_fee() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_with_fee(
			Some(ALICE).into(),
			CurrencyId::B,
			450,
			50,
			Box::new(
				(
					Parent,
					Parachain(2),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					},
				)
					.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	// It should use 40 for weight, so 460 should reach destination
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 460);
	});
}

#[test]
fn send_sibling_asset_to_reserve_sibling_with_distinc_fee() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_multicurrencies(
			Some(ALICE).into(),
			vec![(CurrencyId::B1, 50), (CurrencyId::B, 450)],
			0,
			Box::new(
				(
					Parent,
					Parachain(2),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					},
				)
					.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 550);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B1, &ALICE), 950);
	});

	// It should use 40 for weight, so 450 B and 10 B1 should reach destination
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 550);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B1, &sibling_a_account()), 950);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 450);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B1, &BOB), 10);
	});
}

#[test]
fn send_sibling_asset_to_reserve_sibling_with_distinc_fee_index_works() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_multicurrencies(
			Some(ALICE).into(),
			vec![(CurrencyId::B, 450), (CurrencyId::B1, 50)],
			1,
			Box::new(
				(
					Parent,
					Parachain(2),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					},
				)
					.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 550);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B1, &ALICE), 950);
	});

	// It should use 40 for weight, so 450 B and 10 B1 should reach destination
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 550);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B1, &sibling_a_account()), 950);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 450);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B1, &BOB), 10);
	});
}

#[test]
fn send_sibling_asset_to_non_reserve_sibling() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::B,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(3),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	// check reserve accounts
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_c_account()), 460);
	});

	ParaC::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 420);
	});
}

#[test]
fn send_sibling_asset_to_non_reserve_sibling_with_fee() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_with_fee(
			Some(ALICE).into(),
			CurrencyId::B,
			410,
			90,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(3),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	// Should use only 40 weight
	// check reserve accounts
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_c_account()), 460);
	});

	// Should use 40 additional weight
	ParaC::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 420);
	});
}

#[test]
fn send_self_parachain_asset_to_sibling() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));

		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::A,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 500);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 460);
	});
}

#[test]
fn send_self_parachain_asset_to_sibling_with_fee() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));

		assert_ok!(ParaXTokens::transfer_with_fee(
			Some(ALICE).into(),
			CurrencyId::A,
			450,
			50,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 500);
	});

	// It should use 40 for weight, so 460 should reach destination
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 460);
	});
}

#[test]
fn send_self_parachain_asset_to_sibling_with_distinct_fee() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::A1, &ALICE, 1_000));

		assert_ok!(ParaXTokens::transfer_multicurrencies(
			Some(ALICE).into(),
			vec![(CurrencyId::A, 450), (CurrencyId::A1, 50)],
			1,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 550);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A1, &ALICE), 950);

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 450);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A1, &sibling_b_account()), 50);
	});

	// It should use 40 for weight, so 450 A + 10 A1 should reach destination
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 450);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A1, &BOB), 10);
	});
}

#[test]
fn transfer_no_reserve_assets_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				Box::new((X1(GeneralKey("B".into())).into(), 100).into()),
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into()
						}
					)
						.into()
				),
				50,
			),
			Error::<para::Runtime>::AssetHasNoReserve
		);
	});
}

#[test]
fn transfer_to_self_chain_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				Box::new(MultiAsset::sibling_parachain_asset(1, "A".into(), 100).into()),
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(1),
							Junction::AccountId32 {
								network: NetworkId::Any,
								id: BOB.into()
							}
						)
					)
					.into()
				),
				50,
			),
			Error::<para::Runtime>::NotCrossChainTransfer
		);
	});
}

#[test]
fn transfer_to_invalid_dest_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				Box::new(MultiAsset::sibling_parachain_asset(1, "A".into(), 100).into()),
				Box::new(
					MultiLocation::new(
						0,
						X1(Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into()
						})
					)
					.into()
				),
				50,
			),
			Error::<para::Runtime>::InvalidDest
		);
	});
}

#[test]
fn send_as_sovereign() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000_000_000_000);
	});

	ParaA::execute_with(|| {
		use xcm::latest::OriginKind::SovereignAccount;

		let call =
			relay::Call::System(frame_system::Call::<relay::Runtime>::remark_with_event { remark: vec![1, 1, 1] });
		let assets: MultiAsset = (Here, 1_000_000_000_000).into();
		assert_ok!(para::OrmlXcm::send_as_sovereign(
			para::Origin::root(),
			Box::new(Parent.into()),
			Box::new(VersionedXcm::from(Xcm(vec![
				WithdrawAsset(assets.clone().into()),
				BuyExecution {
					fees: assets,
					weight_limit: Limited(2_000_000_000)
				},
				Instruction::Transact {
					origin_type: SovereignAccount,
					require_weight_at_most: 1_000_000_000,
					call: call.encode().into(),
				}
			])))
		));
	});

	Relay::execute_with(|| {
		assert!(relay::System::events().iter().any(|r| {
			matches!(
				r.event,
				relay::Event::System(frame_system::Event::<relay::Runtime>::Remarked { sender: _, hash: _ })
			)
		}));
	})
}

#[test]
fn send_as_sovereign_fails_if_bad_origin() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000_000_000_000);
	});

	ParaA::execute_with(|| {
		use xcm::latest::OriginKind::SovereignAccount;

		let call =
			relay::Call::System(frame_system::Call::<relay::Runtime>::remark_with_event { remark: vec![1, 1, 1] });
		let assets: MultiAsset = (Here, 1_000_000_000_000).into();
		assert_err!(
			para::OrmlXcm::send_as_sovereign(
				para::Origin::signed(ALICE),
				Box::new(Parent.into()),
				Box::new(VersionedXcm::from(Xcm(vec![
					WithdrawAsset(assets.clone().into()),
					BuyExecution {
						fees: assets,
						weight_limit: Limited(10_000_000)
					},
					Instruction::Transact {
						origin_type: SovereignAccount,
						require_weight_at_most: 1_000_000_000,
						call: call.encode().into(),
					}
				])))
			),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn call_size_limit() {
	// Ensures Call enum doesn't allocate more than 200 bytes in runtime
	assert!(
		core::mem::size_of::<crate::Call::<crate::tests::para::Runtime>>() <= 200,
		"size of Call is more than 200 bytes: some calls have too big arguments, use Box to \
		reduce the size of Call.
		If the limit is too strong, maybe consider increasing the limit",
	);

	assert!(
		core::mem::size_of::<orml_xcm::Call::<crate::tests::para::Runtime>>() <= 200,
		"size of Call is more than 200 bytes: some calls have too big arguments, use Box to \
		reduce the size of Call.
		If the limit is too strong, maybe consider increasing the limit",
	);
}

#[test]
fn send_with_zero_fee_should_yield_an_error() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));

		// Transferring with zero fee should fail
		assert_noop!(
			ParaXTokens::transfer_with_fee(
				Some(ALICE).into(),
				CurrencyId::A,
				450,
				0,
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(2),
							Junction::AccountId32 {
								network: NetworkId::Any,
								id: BOB.into(),
							}
						)
					)
					.into()
				),
				40,
			),
			Error::<para::Runtime>::FeeCannotBeZero
		);
	});
}

#[test]
fn send_with_insufficient_fee_traps_assets() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));

		// ParaB charges 40, but we specify 30 as fee. Assets will be trapped
		// Call succedes in paraA
		assert_ok!(ParaXTokens::transfer_with_fee(
			Some(ALICE).into(),
			CurrencyId::A,
			450,
			30,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));
	});

	// In paraB, assets have been trapped due to he failed execution
	ParaB::execute_with(|| {
		assert!(para::System::events().iter().any(|r| {
			matches!(
				r.event,
				para::Event::PolkadotXcm(pallet_xcm::Event::<para::Runtime>::AssetsTrapped(_, _, _))
			)
		}));
	})
}

#[test]
fn send_with_fee_should_handle_overflow() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));

		// Uses saturated add, so xcm execution should fail because we dont have
		// enough tokens
		assert_noop!(
			ParaXTokens::transfer_with_fee(
				Some(ALICE).into(),
				CurrencyId::A,
				u128::MAX,
				1,
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(2),
							Junction::AccountId32 {
								network: NetworkId::Any,
								id: BOB.into(),
							}
						)
					)
					.into()
				),
				40,
			),
			Error::<para::Runtime>::XcmExecutionFailed
		);
	});
}

#[test]
fn specifying_more_than_two_assets_should_error() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &sibling_a_account(), 1_000));
	});

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::B, 450), (CurrencyId::B1, 50), (CurrencyId::R, 5000)],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						},
					)
						.into()
				),
				40,
			),
			Error::<para::Runtime>::TooManyAssetsBeingSent
		);
	});
}

#[test]
fn sending_assets_with_different_reserve_should_fail() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
	});

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::B, 450), (CurrencyId::R, 5000)],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						},
					)
						.into()
				),
				40,
			),
			Error::<para::Runtime>::DistinctReserveForAssetAndFee
		);
	});
}

#[test]
fn specifying_a_non_existent_asset_index_should_fail() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::B, 450)],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						},
					)
						.into()
				),
				40,
			),
			Error::<para::Runtime>::AssetIndexNonExistent
		);
	});
}
