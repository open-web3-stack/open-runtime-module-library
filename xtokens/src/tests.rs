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
fn send_sibling_asset_to_reserve_sibling() {
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
				relay::Event::System(frame_system::Event::<relay::Runtime>::Remarked(_, _))
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
