#![cfg(test)]

use super::*;
use codec::Encode;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_err, assert_noop, assert_ok, traits::Currency};
use mock::*;
use orml_traits::MultiCurrency;
use polkadot_parachain::primitives::{AccountIdConversion, Sibling};
use sp_runtime::AccountId32;
use xcm::v0::{Error as XcmError, Junction, NetworkId, Order};
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
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			500,
			(
				Parent,
				Junction::AccountId32 {
					network: NetworkId::Kusama,
					id: BOB.into(),
				},
			)
				.into(),
			30,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&BOB), 470);
	});
}

#[test]
fn cannot_lost_fund_on_send_failed() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::A,
			500,
			(
				Parent,
				Parachain(100),
				Junction::AccountId32 {
					network: NetworkId::Kusama,
					id: BOB.into(),
				},
			)
				.into(),
			30,
		));
		assert!(para::System::events().iter().any(|r| matches!(
			r.event,
			para::Event::XTokens(Event::<para::Runtime>::TransferFailed(
				_,
				_,
				_,
				_,
				XcmError::CannotReachDestination(_, _)
			))
		)));

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
			(
				Parent,
				Parachain(2),
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
			30,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&para_b_account()), 470);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &BOB), 440);
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
			(
				Parent,
				Parachain(2),
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
			30,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 470);
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
			(
				Parent,
				Parachain(3),
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
			30
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	// check reserve accounts
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_c_account()), 470);
	});

	ParaC::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 440);
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
			(
				Parent,
				Parachain(2),
				Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into(),
				},
			)
				.into(),
			30,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 500);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 470);
	});
}

#[test]
fn transfer_no_reserve_assets_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				MultiAsset::ConcreteFungible {
					id: GeneralKey("B".into()).into(),
					amount: 100
				},
				(
					Parent,
					Parachain(2),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into()
					}
				)
					.into(),
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
				MultiAsset::ConcreteFungible {
					id: (Parent, Parachain(1), GeneralKey("A".into())).into(),
					amount: 100
				},
				(
					Parent,
					Parachain(1),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into()
					}
				)
					.into(),
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
				MultiAsset::ConcreteFungible {
					id: (Parent, Parachain(1), GeneralKey("A".into())).into(),
					amount: 100,
				},
				(Junction::AccountId32 {
					network: NetworkId::Any,
					id: BOB.into()
				})
				.into(),
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
		use xcm::v0::OriginKind::SovereignAccount;

		let call = relay::Call::System(frame_system::Call::<relay::Runtime>::remark_with_event(vec![1, 1, 1]));
		assert_ok!(para::OrmlXcm::send_as_sovereign(
			para::Origin::root(),
			Junction::Parent.into(),
			WithdrawAsset {
				assets: vec![MultiAsset::ConcreteFungible {
					id: MultiLocation::Null,
					amount: 1_000_000_000_000
				}],
				effects: vec![Order::BuyExecution {
					fees: MultiAsset::All,
					weight: 10_000_000,
					debt: 10_000_000,
					halt_on_error: true,
					xcm: vec![Transact {
						origin_type: SovereignAccount,
						require_weight_at_most: 1_000_000_000,
						call: call.encode().into(),
					}],
				}]
			}
		));
	});

	Relay::execute_with(|| {
		relay::System::events().iter().any(|r| {
			if let relay::Event::System(frame_system::Event::<relay::Runtime>::Remarked(_, _)) = r.event {
				true
			} else {
				false
			}
		});
	})
}

#[test]
fn send_as_sovereign_fails_if_bad_origin() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000_000_000_000);
	});

	ParaA::execute_with(|| {
		use xcm::v0::OriginKind::SovereignAccount;

		let call = relay::Call::System(frame_system::Call::<relay::Runtime>::remark_with_event(vec![1, 1, 1]));
		assert_err!(
			para::OrmlXcm::send_as_sovereign(
				para::Origin::signed(ALICE),
				Junction::Parent.into(),
				WithdrawAsset {
					assets: vec![MultiAsset::ConcreteFungible {
						id: MultiLocation::Null,
						amount: 1_000_000_000_000
					}],
					effects: vec![Order::BuyExecution {
						fees: MultiAsset::All,
						weight: 10_000_000,
						debt: 10_000_000,
						halt_on_error: true,
						xcm: vec![Transact {
							origin_type: SovereignAccount,
							require_weight_at_most: 1_000_000_000,
							call: call.encode().into(),
						}],
					}]
				}
			),
			DispatchError::BadOrigin,
		);
	});
}
