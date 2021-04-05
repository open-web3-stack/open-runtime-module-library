#![cfg(test)]

use super::*;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_noop, assert_ok, traits::Currency};
use mock::*;
use orml_traits::MultiCurrency;
use polkadot_parachain::primitives::{AccountIdConversion, Sibling};
use sp_runtime::AccountId32;
use xcm::v0::{Junction, NetworkId};
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

#[test]
fn send_relay_chain_asset_to_relay_chain() {
	TestNetwork::reset();

	MockRelay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 100);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaAXtokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			30,
			(
				Parent,
				Junction::AccountId32 {
					network: NetworkId::Polkadot,
					id: BOB.into(),
				},
			)
				.into(),
		));
		assert_eq!(ParaATokens::free_balance(CurrencyId::R, &ALICE), 70);
	});

	MockRelay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 70);
		assert_eq!(RelayBalances::free_balance(&BOB), 30);
	});
}

#[test]
fn send_relay_chain_asset_to_sibling() {
	TestNetwork::reset();

	MockRelay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 100);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaAXtokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			30,
			(
				Parent,
				Parachain { id: 2 },
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
		));
		assert_eq!(ParaATokens::free_balance(CurrencyId::R, &ALICE), 70);
	});

	MockRelay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 70);
		assert_eq!(RelayBalances::free_balance(&para_b_account()), 30);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaBTokens::free_balance(CurrencyId::R, &BOB), 30);
	});
}

#[test]
fn send_sibling_asset_to_reserve_sibling() {
	TestNetwork::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaATokens::deposit(CurrencyId::B, &ALICE, 100));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaBTokens::deposit(CurrencyId::B, &sibling_a_account(), 100));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaAXtokens::transfer(
			Some(ALICE).into(),
			CurrencyId::B,
			30,
			(
				Parent,
				Parachain { id: 2 },
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
		));

		assert_eq!(ParaATokens::free_balance(CurrencyId::B, &ALICE), 70);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaBTokens::free_balance(CurrencyId::B, &sibling_a_account()), 70);
		assert_eq!(ParaBTokens::free_balance(CurrencyId::B, &BOB), 30);
	});
}

#[test]
fn send_sibling_asset_to_non_reserve_sibling() {
	TestNetwork::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaATokens::deposit(CurrencyId::B, &ALICE, 100));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaBTokens::deposit(CurrencyId::B, &sibling_a_account(), 100));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaAXtokens::transfer(
			Some(ALICE).into(),
			CurrencyId::B,
			30,
			(
				Parent,
				Parachain { id: 3 },
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
		));
		assert_eq!(ParaATokens::free_balance(CurrencyId::B, &ALICE), 70);
	});

	// check reserve accounts
	ParaB::execute_with(|| {
		assert_eq!(ParaBTokens::free_balance(CurrencyId::B, &sibling_a_account()), 70);
		assert_eq!(ParaBTokens::free_balance(CurrencyId::B, &sibling_c_account()), 30);
	});

	ParaC::execute_with(|| {
		assert_eq!(ParaCTokens::free_balance(CurrencyId::B, &BOB), 30);
	});
}

#[test]
fn send_self_parachain_asset_to_sibling() {
	TestNetwork::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaATokens::deposit(CurrencyId::A, &ALICE, 100));

		assert_ok!(ParaAXtokens::transfer(
			Some(ALICE).into(),
			CurrencyId::A,
			30,
			(
				Parent,
				Parachain { id: 2 },
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
		));

		assert_eq!(ParaATokens::free_balance(CurrencyId::A, &ALICE), 70);
		assert_eq!(ParaATokens::free_balance(CurrencyId::A, &sibling_b_account()), 30);
	});

	ParaB::execute_with(|| {
		para_b::System::events().iter().for_each(|r| {
			println!(">>> {:?}", r.event);
		});
		assert_eq!(ParaBTokens::free_balance(CurrencyId::A, &BOB), 30);
	});
}

#[test]
fn transfer_no_reserve_assets_fails() {
	TestNetwork::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaAXtokens::transfer_multiasset(
				Some(ALICE).into(),
				MultiAsset::ConcreteFungible {
					id: GeneralKey("B".into()).into(),
					amount: 1
				},
				(
					Parent,
					Parachain { id: 2 },
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into()
					}
				)
					.into()
			),
			Error::<para_a::Runtime>::AssetHasNoReserve
		);
	});
}

#[test]
fn transfer_to_self_chain_fails() {
	TestNetwork::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaAXtokens::transfer_multiasset(
				Some(ALICE).into(),
				MultiAsset::ConcreteFungible {
					id: (Parent, Parachain { id: 1 }, GeneralKey("A".into())).into(),
					amount: 1
				},
				(
					Parent,
					Parachain { id: 1 },
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into()
					}
				)
					.into()
			),
			Error::<para_a::Runtime>::NotCrossChainTransfer
		);
	});
}

#[test]
fn transfer_to_invalid_dest_fails() {
	TestNetwork::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaAXtokens::transfer_multiasset(
				Some(ALICE).into(),
				MultiAsset::ConcreteFungible {
					id: (Parent, Parachain { id: 1 }, GeneralKey("A".into())).into(),
					amount: 1
				},
				(Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into()
				})
				.into()
			),
			Error::<para_a::Runtime>::InvalidDest
		);
	});
}
