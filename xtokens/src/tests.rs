#![cfg(test)]

use super::*;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_err, assert_noop, assert_ok, traits::Currency};
use mock::*;
use orml_traits::{ConcreteFungibleAsset, MultiCurrency};
use parity_scale_codec::Encode;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use xcm::{v3::OriginKind::SovereignAccount, VersionedXcm};
use xcm_simulator::TestExt;

fn para_a_account() -> AccountId32 {
	ParaId::from(1).into_account_truncating()
}

fn para_b_account() -> AccountId32 {
	ParaId::from(2).into_account_truncating()
}

fn para_d_account() -> AccountId32 {
	ParaId::from(4).into_account_truncating()
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

fn sibling_d_account() -> AccountId32 {
	Sibling::from(4).into_account_truncating()
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
						network: None,
						id: BOB.into(),
					})
				)
				.into()
			),
			WeightLimit::Unlimited
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
						network: None,
						id: BOB.into(),
					})
				)
				.into()
			),
			WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
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
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Unlimited
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
						network: None,
						id: ALICE.into(),
					},
				)
					.into()
			),
			WeightLimit::Unlimited
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
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Unlimited
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
fn send_sibling_asset_to_reserve_sibling_with_distinct_fee() {
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
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Unlimited
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
fn send_sibling_asset_to_reserve_sibling_with_distinct_fee_index_works() {
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
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
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
fn sending_sibling_asset_to_reserve_sibling_with_relay_fee_works() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::C, &ALICE, 1_000));
	});

	ParaC::execute_with(|| {
		assert_ok!(ParaTeleportTokens::deposit(CurrencyId::C, &sibling_a_account(), 1_000));
	});

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	let fee_amount: u128 = 200;
	let weight: u128 = 50;
	let dest_weight: u128 = 40;

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_multicurrencies(
			Some(ALICE).into(),
			vec![(CurrencyId::C, 450), (CurrencyId::R, fee_amount)],
			1,
			Box::new(
				(
					Parent,
					Parachain(3),
					Junction::AccountId32 {
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Limited((weight as u64).into()),
		));
		assert_eq!(550, ParaTokens::free_balance(CurrencyId::C, &ALICE));
		assert_eq!(1000 - fee_amount, ParaTokens::free_balance(CurrencyId::R, &ALICE));
	});

	Relay::execute_with(|| {
		assert_eq!(
			1000 - (fee_amount - dest_weight),
			RelayBalances::free_balance(&para_a_account())
		);
	});

	ParaC::execute_with(|| {
		assert_eq!(
			fee_amount - dest_weight * 4,
			ParaTeleportTokens::free_balance(CurrencyId::R, &sibling_a_account())
		);

		assert_eq!(450, ParaTeleportTokens::free_balance(CurrencyId::C, &BOB));
		assert_eq!(0, ParaTeleportTokens::free_balance(CurrencyId::R, &BOB));
	});
}

#[test]
fn sending_sibling_asset_to_reserve_sibling_with_relay_fee_works_with_relative_self_location() {
	TestNet::reset();

	ParaD::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::C, &ALICE, 1_000));
	});

	ParaC::execute_with(|| {
		assert_ok!(ParaTeleportTokens::deposit(CurrencyId::C, &sibling_d_account(), 1_000));
	});

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_d_account(), 1_000);
	});

	let fee_amount: u128 = 200;
	let weight: u128 = 50;
	let dest_weight: u128 = 40;

	ParaD::execute_with(|| {
		assert_ok!(ParaRelativeXTokens::transfer_multicurrencies(
			Some(ALICE).into(),
			vec![(CurrencyId::C, 450), (CurrencyId::R, fee_amount)],
			1,
			Box::new(
				(
					Parent,
					Parachain(3),
					Junction::AccountId32 {
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Limited((weight as u64).into()),
		));
		assert_eq!(550, ParaRelativeTokens::free_balance(CurrencyId::C, &ALICE));
		assert_eq!(
			1000 - fee_amount,
			ParaRelativeTokens::free_balance(CurrencyId::R, &ALICE)
		);
	});

	Relay::execute_with(|| {
		assert_eq!(
			1000 - (fee_amount - dest_weight),
			RelayBalances::free_balance(&para_d_account())
		);
	});

	ParaC::execute_with(|| {
		assert_eq!(
			fee_amount - dest_weight * 4,
			ParaTeleportTokens::free_balance(CurrencyId::R, &sibling_d_account())
		);

		assert_eq!(450, ParaTeleportTokens::free_balance(CurrencyId::C, &BOB));
		assert_eq!(0, ParaTeleportTokens::free_balance(CurrencyId::R, &BOB));
	});
}

#[test]
fn sending_sibling_asset_to_reserve_sibling_with_relay_fee_not_enough() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::C, &ALICE, 1_000));
	});

	ParaC::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::C, &sibling_a_account(), 1_000));
	});

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	let fee_amount: u128 = 159;
	let weight: u128 = 50;
	let dest_weight: u128 = 40;

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer_multicurrencies(
			Some(ALICE).into(),
			vec![(CurrencyId::C, 450), (CurrencyId::R, fee_amount)],
			1,
			Box::new(
				(
					Parent,
					Parachain(3),
					Junction::AccountId32 {
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Limited((weight as u64).into()),
		));
		assert_eq!(550, ParaTokens::free_balance(CurrencyId::C, &ALICE));
		assert_eq!(1000 - fee_amount, ParaTokens::free_balance(CurrencyId::R, &ALICE));
	});

	Relay::execute_with(|| {
		assert_eq!(
			1000 - (fee_amount - dest_weight),
			RelayBalances::free_balance(&para_a_account())
		);
	});

	ParaC::execute_with(|| {
		// after first xcm succeed, sibling_a amount = 159-120=39
		// second xcm failed, so sibling_a amount stay same.
		assert_eq!(39, ParaTokens::free_balance(CurrencyId::R, &sibling_a_account()));

		// second xcm failed, so recipient account don't receive any token of B and R.
		assert_eq!(0, ParaTokens::free_balance(CurrencyId::C, &BOB));
		assert_eq!(0, ParaTokens::free_balance(CurrencyId::R, &BOB));
	});
}

#[test]
fn transfer_asset_with_relay_fee_failed() {
	TestNet::reset();

	// `SelfReserve` with relay-chain as fee not supported.
	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::A, 450), (CurrencyId::R, 100)],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::InvalidAsset
		);
	});

	// `NonReserve` with relay-chain as fee not supported.
	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::C, 450), (CurrencyId::R, 100)],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::InvalidAsset
		);
	});

	// `ToReserve` with relay-chain as fee supported.
	// But user fee is less than `MinXcmFee`
	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::C, 450), (CurrencyId::R, 39)],
				1,
				Box::new(
					(
						Parent,
						Parachain(3),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::FeeNotEnough
		);
	});

	// `MinXcmFee` not defined for destination chain
	ParaB::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::A, 450), (CurrencyId::R, 100)],
				1,
				Box::new(
					(
						Parent,
						Parachain(1),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::MinXcmFeeNotDefined
		);
	});
}

#[test]
fn transfer_no_reserve_assets_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		let asset_id: AssetId = X1(Junction::from(BoundedVec::try_from(b"B".to_vec()).unwrap())).into();
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				Box::new((asset_id, 100).into()),
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into()
						}
					)
						.into()
				),
				WeightLimit::Unlimited
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
				Box::new(MultiAsset::sibling_parachain_asset(1, b"A".to_vec().try_into().unwrap(), 100).into()),
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(1),
							Junction::AccountId32 {
								network: None,
								id: BOB.into()
							}
						)
					)
					.into()
				),
				WeightLimit::Unlimited
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
				Box::new(MultiAsset::sibling_parachain_asset(1, b"A".to_vec().try_into().unwrap(), 100).into()),
				Box::new(
					MultiLocation::new(
						0,
						X1(Junction::AccountId32 {
							network: None,
							id: BOB.into()
						})
					)
					.into()
				),
				WeightLimit::Unlimited
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
		let call = relay::RuntimeCall::System(frame_system::Call::<relay::Runtime>::remark_with_event {
			remark: vec![1, 1, 1],
		});
		let assets: MultiAsset = (Here, 1_000_000_000_000u128).into();
		assert_ok!(para::OrmlXcm::send_as_sovereign(
			para::RuntimeOrigin::root(),
			Box::new(Parent.into()),
			Box::new(VersionedXcm::from(Xcm(vec![
				WithdrawAsset(assets.clone().into()),
				BuyExecution {
					fees: assets,
					weight_limit: Limited(2_000_000_000.into())
				},
				Instruction::Transact {
					origin_kind: SovereignAccount,
					require_weight_at_most: 1_000_000_000.into(),
					call: call.encode().into(),
				}
			])))
		));
	});

	Relay::execute_with(|| {
		assert!(relay::System::events().iter().any(|r| {
			matches!(
				r.event,
				relay::RuntimeEvent::System(frame_system::Event::<relay::Runtime>::Remarked { sender: _, hash: _ })
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
		let call = relay::RuntimeCall::System(frame_system::Call::<relay::Runtime>::remark_with_event {
			remark: vec![1, 1, 1],
		});
		let assets: MultiAsset = (Here, 1_000_000_000_000u128).into();
		assert_err!(
			para::OrmlXcm::send_as_sovereign(
				para::RuntimeOrigin::signed(ALICE),
				Box::new(Parent.into()),
				Box::new(VersionedXcm::from(Xcm(vec![
					WithdrawAsset(assets.clone().into()),
					BuyExecution {
						fees: assets,
						weight_limit: Limited(10_000_000.into())
					},
					Instruction::Transact {
						origin_kind: SovereignAccount,
						require_weight_at_most: 1_000_000_000.into(),
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
								network: None,
								id: BOB.into(),
							}
						)
					)
					.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::ZeroFee
		);
	});
}

#[test]
fn send_with_insufficient_fee_traps_assets() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));

		// ParaB charges 40, but we specify 30 as fee. Assets will be trapped
		// Call succeed in paraA
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Unlimited
		));
	});

	// In paraB, assets have been trapped due to he failed execution
	ParaB::execute_with(|| {
		assert!(para::System::events().iter().any(|r| {
			matches!(
				r.event,
				para::RuntimeEvent::PolkadotXcm(pallet_xcm::Event::<para::Runtime>::AssetsTrapped { .. })
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
				100,
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
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::XcmExecutionFailed
		);
	});
}

#[test]
fn specifying_more_than_assets_limit_should_error() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B2, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &sibling_a_account(), 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B2, &sibling_a_account(), 1_000));
	});

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![
					(CurrencyId::B, 450),
					(CurrencyId::B1, 200),
					(CurrencyId::R, 5000),
					(CurrencyId::B2, 500)
				],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::TooManyAssetsBeingSent
		);
	});
}

#[test]
fn sending_non_fee_assets_with_different_reserve_should_fail() {
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
				vec![(CurrencyId::B, 450), (CurrencyId::R, 5000), (CurrencyId::A, 450)],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
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
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::AssetIndexNonExistent
		);
	});
}

#[test]
fn send_with_zero_amount() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer(
				Some(ALICE).into(),
				CurrencyId::B,
				0,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::ZeroAmount
		);

		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::B, 0), (CurrencyId::B1, 50)],
				1,
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::ZeroAmount
		);
	});

	// TODO: should have more tests after https://github.com/paritytech/polkadot/issues/4996
}

#[test]
fn send_self_parachain_asset_to_sibling_relative_parachain() {
	TestNet::reset();

	ParaD::execute_with(|| {
		assert_ok!(ParaRelativeTokens::deposit(CurrencyId::D, &ALICE, 1_000));

		assert_ok!(ParaRelativeXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::D,
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
			WeightLimit::Unlimited
		));

		assert_eq!(ParaRelativeTokens::free_balance(CurrencyId::D, &ALICE), 500);
		assert_eq!(
			ParaRelativeTokens::free_balance(CurrencyId::D, &sibling_b_account()),
			500
		);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::D, &BOB), 460);
	});
}

#[test]
fn send_sibling_asset_to_reserve_sibling_with_relative_view() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::D, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &sibling_d_account(), 1_000));
	});

	ParaD::execute_with(|| {
		assert_ok!(ParaRelativeTokens::deposit(CurrencyId::D, &sibling_a_account(), 1_000));
		assert_ok!(ParaRelativeTokens::deposit(CurrencyId::A, &BOB, 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::D,
			500,
			Box::new(
				(
					Parent,
					Parachain(4),
					Junction::AccountId32 {
						network: None,
						id: BOB.into(),
					},
				)
					.into()
			),
			WeightLimit::Unlimited
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::D, &ALICE), 500);
	});

	ParaD::execute_with(|| {
		assert_eq!(
			ParaRelativeTokens::free_balance(CurrencyId::D, &sibling_a_account()),
			500
		);
		assert_eq!(ParaRelativeTokens::free_balance(CurrencyId::D, &BOB), 460);

		assert_ok!(ParaRelativeXTokens::transfer(
			Some(BOB).into(),
			CurrencyId::A,
			500,
			Box::new(
				(
					Parent,
					Parachain(1),
					Junction::AccountId32 {
						network: None,
						id: ALICE.into(),
					},
				)
					.into()
			),
			WeightLimit::Unlimited
		));
		assert_eq!(ParaRelativeTokens::free_balance(CurrencyId::A, &BOB), 500);
	});

	ParaA::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_d_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 460);
	});
}

#[test]
fn send_relative_view_sibling_asset_to_non_reserve_sibling() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::D, &ALICE, 1_000));
	});

	ParaD::execute_with(|| {
		assert_ok!(ParaRelativeTokens::deposit(CurrencyId::D, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::D,
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
			WeightLimit::Unlimited
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::D, &ALICE), 500);
	});

	// check reserve accounts
	ParaD::execute_with(|| {
		assert_eq!(
			ParaRelativeTokens::free_balance(CurrencyId::D, &sibling_a_account()),
			500
		);
		assert_eq!(
			ParaRelativeTokens::free_balance(CurrencyId::D, &sibling_b_account()),
			460
		);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::D, &BOB), 420);
	});
}

#[test]
fn send_relay_chain_asset_to_relative_view_sibling() {
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
						Parachain(4),
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
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&para_d_account()), 460);
	});

	ParaD::execute_with(|| {
		assert_eq!(ParaRelativeTokens::free_balance(CurrencyId::R, &BOB), 420);
	});
}

#[test]
fn unsupported_multilocation_should_be_filtered() {
	TestNet::reset();

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
		assert_ok!(ParaTokens::deposit(CurrencyId::B1, &ALICE, 1_000));
		assert_noop!(
			ParaXTokens::transfer(
				Some(ALICE).into(),
				CurrencyId::B,
				500,
				Box::new(
					(
						Parent,
						Parachain(5), // parachain 4 is not supported list.
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::NotSupportedMultiLocation
		);

		assert_noop!(
			ParaXTokens::transfer_multicurrencies(
				Some(ALICE).into(),
				vec![(CurrencyId::B1, 50), (CurrencyId::B, 450)],
				0,
				Box::new(
					(
						Parent,
						Parachain(5),
						Junction::AccountId32 {
							network: None,
							id: BOB.into(),
						},
					)
						.into()
				),
				WeightLimit::Unlimited
			),
			Error::<para::Runtime>::NotSupportedMultiLocation
		);
	});
}

#[test]
fn send_with_sufficient_weight_limit() {
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Limited(40.into()),
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 500);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 460);
	});
}

#[test]
fn send_with_insufficient_weight_limit() {
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
							network: None,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			WeightLimit::Limited(1.into()),
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 500);
	});

	ParaB::execute_with(|| {
		// no funds should arrive - message will have failed
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 0);
	});
}

#[test]
fn send_multiasset_with_zero_fee_should_yield_an_error() {
	TestNet::reset();

	let asset_id: AssetId = X1(Junction::from(BoundedVec::try_from(b"A".to_vec()).unwrap())).into();
	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset_with_fee(
				Some(ALICE).into(),
				Box::new((asset_id, 100).into()),
				Box::new((asset_id, Fungibility::Fungible(0)).into()),
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(2),
							Junction::AccountId32 {
								network: None,
								id: BOB.into()
							},
						)
					)
					.into()
				),
				WeightLimit::Unlimited,
			),
			Error::<para::Runtime>::InvalidAsset
		);
	});
}

#[test]
fn send_undefined_nft_should_yield_an_error() {
	TestNet::reset();

	let fee_id: AssetId = X1(Junction::from(BoundedVec::try_from(b"A".to_vec()).unwrap())).into();
	let nft_id: AssetId = X1(Junction::GeneralIndex(42)).into();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset_with_fee(
				Some(ALICE).into(),
				Box::new((nft_id, Undefined).into()),
				Box::new((fee_id, 100).into()),
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(2),
							Junction::AccountId32 {
								network: None,
								id: BOB.into()
							},
						)
					)
					.into()
				),
				WeightLimit::Unlimited,
			),
			Error::<para::Runtime>::InvalidAsset
		);
	});
}

#[test]
fn nfts_cannot_be_fee_assets() {
	TestNet::reset();

	let asset_id: AssetId = X1(Junction::from(BoundedVec::try_from(b"A".to_vec()).unwrap())).into();
	let nft_id: AssetId = X1(Junction::GeneralIndex(42)).into();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset_with_fee(
				Some(ALICE).into(),
				Box::new((asset_id, 100).into()),
				Box::new((nft_id, Index(1)).into()),
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(2),
							Junction::AccountId32 {
								network: None,
								id: BOB.into()
							},
						)
					)
					.into()
				),
				WeightLimit::Unlimited,
			),
			Error::<para::Runtime>::InvalidAsset
		);
	});
}
