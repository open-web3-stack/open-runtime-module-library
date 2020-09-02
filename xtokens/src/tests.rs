//! Unit tests for the xtokens module.

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn transfer_to_relay_chain_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		assert_ok!(XTokens::transfer_to_relay_chain(Origin::signed(ALICE), BOB, 50));

		assert_eq!(Tokens::free_balance(CurrencyId::DOT, &ALICE), 50);
		assert!(MockUpwardMessageSender::msg_sent(MockUpwardMessage(BOB, 50)));

		// let event =
		// TestEvent::xtokens(RawEvent::TransferredToRelayChain(ALICE, BOB,
		// 50)); assert!(System::events().iter().any(|record| record.event ==
		// event));
	});
}

#[test]
fn transfer_to_relay_chain_fails_if_insufficient_balance() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			XTokens::transfer_to_relay_chain(Origin::signed(ALICE), BOB, 50),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
	});
}

#[test]
fn transfer_relay_chain_tokens_to_parachain_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::RelayChain, vec![0]);
		assert_ok!(XTokens::transfer_to_parachain(
			Origin::signed(ALICE),
			x_currency_id.clone(),
			para_one_id(),
			BOB,
			50
		));

		assert_eq!(Tokens::free_balance(CurrencyId::DOT, &ALICE), 50);
		assert!(MockUpwardMessageSender::msg_sent(MockUpwardMessage(
			para_one_account(),
			50
		)));
		assert!(MockXCMPMessageSender::msg_sent(
			para_one_id(),
			XCMPTokenMessage::Transfer(x_currency_id.clone(), para_one_id(), BOB, 50)
		));

		// let event =
		// TestEvent::xtokens(RawEvent::TransferredToParachain(x_currency_id,
		// ALICE, para_one_id(), BOB, 50)); assert!(System::events().iter().
		// any(|record| record.event == event));
	});
}

#[test]
fn transfer_relay_chain_tokens_to_parachain_fails_if_insufficient_balance() {
	ExtBuilder::default().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::RelayChain, vec![0]);
		assert_noop!(
			XTokens::transfer_to_parachain(Origin::signed(ALICE), x_currency_id, para_one_id(), BOB, 50),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
	});
}

#[test]
fn transfer_owned_tokens_to_parachain_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(MockParaId::get()), CurrencyId::Owned.into());
		assert_ok!(XTokens::transfer_to_parachain(
			Origin::signed(ALICE),
			x_currency_id.clone(),
			para_one_id(),
			BOB,
			50
		));

		assert_eq!(Tokens::free_balance(CurrencyId::Owned, &ALICE), 50);
		assert_eq!(Tokens::free_balance(CurrencyId::Owned, &para_one_account()), 50);
		assert!(MockXCMPMessageSender::msg_sent(
			para_one_id(),
			XCMPTokenMessage::Transfer(x_currency_id, para_one_id(), BOB, 50)
		));
	});
}

#[test]
fn transfer_owned_tokens_to_parachain_fails_if_unrecognized_currency_id() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(MockParaId::get()), unknown_currency_id());
		assert_noop!(
			XTokens::transfer_to_parachain(Origin::signed(ALICE), x_currency_id, para_one_id(), BOB, 50),
			Error::<Runtime>::InvalidCurrencyId
		);
	});
}

#[test]
fn transfer_owned_tokens_to_parachain_fails_if_insufficient_balance() {
	ExtBuilder::default().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(MockParaId::get()), CurrencyId::Owned.into());
		assert_noop!(
			XTokens::transfer_to_parachain(Origin::signed(ALICE), x_currency_id, para_one_id(), BOB, 50),
			orml_tokens::Error::<Runtime>::BalanceTooLow,
		);
	});
}

#[test]
fn transfer_known_non_owned_tokens_to_parachain_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(para_one_id()), CurrencyId::BTC.into());
		assert_ok!(XTokens::transfer_to_parachain(
			Origin::signed(ALICE),
			x_currency_id.clone(),
			para_two_id(),
			BOB,
			50
		));

		assert_eq!(Tokens::free_balance(CurrencyId::BTC, &ALICE), 50);
		assert!(MockXCMPMessageSender::msg_sent(
			para_one_id(),
			XCMPTokenMessage::Transfer(x_currency_id, para_two_id(), BOB, 50)
		));
	});
}

#[test]
fn transfer_known_non_owned_tokens_fails_if_insufficient_balance() {
	ExtBuilder::default().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(para_one_id()), CurrencyId::BTC.into());
		assert_noop!(
			XTokens::transfer_to_parachain(Origin::signed(ALICE), x_currency_id, para_two_id(), BOB, 50),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
	});
}

#[test]
fn transfer_unknown_non_owned_tokens_to_parachain_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		<UnknownBalances<Runtime>>::insert(ALICE, unknown_currency_id(), 100);

		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(para_one_id()), unknown_currency_id());
		assert_ok!(XTokens::transfer_to_parachain(
			Origin::signed(ALICE),
			x_currency_id.clone(),
			para_two_id(),
			BOB,
			50
		));

		assert_eq!(XTokens::unknown_balances(ALICE, unknown_currency_id()), 50);
		assert!(MockXCMPMessageSender::msg_sent(
			para_one_id(),
			XCMPTokenMessage::Transfer(x_currency_id, para_two_id(), BOB, 50)
		));
	});
}

#[test]
fn transfer_unknown_non_owned_tokens_fails_if_insufficient_balance() {
	ExtBuilder::default().build().execute_with(|| {
		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(para_one_id()), unknown_currency_id());
		assert_noop!(
			XTokens::transfer_to_parachain(Origin::signed(ALICE), x_currency_id, para_two_id(), BOB, 50),
			Error::<Runtime>::InsufficientBalance
		);
	});
}
