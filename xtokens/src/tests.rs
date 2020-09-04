//! Unit tests for the xtokens module.

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};

#[test]
fn transfer_to_relay_chain_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(XTokens::transfer_to_relay_chain(Origin::signed(ALICE), BOB, 50));

		assert_eq!(Tokens::free_balance(CurrencyId::DOT, &ALICE), 50);
		assert!(MockUpwardMessageSender::msg_sent(MockUpwardMessage(BOB, 50)));

		let event = TestEvent::xtokens(RawEvent::TransferredToRelayChain(ALICE, BOB, 50));
		assert!(System::events().iter().any(|record| record.event == event));
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
		System::set_block_number(1);

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

		let event = TestEvent::xtokens(RawEvent::TransferredToParachain(
			x_currency_id,
			ALICE,
			para_one_id(),
			BOB,
			50,
		));
		assert!(System::events().iter().any(|record| record.event == event));
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
		System::set_block_number(1);

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
			XCMPTokenMessage::Transfer(x_currency_id.clone(), para_one_id(), BOB, 50)
		));

		let event = TestEvent::xtokens(RawEvent::TransferredToParachain(
			x_currency_id,
			ALICE,
			para_one_id(),
			BOB,
			50,
		));
		assert!(System::events().iter().any(|record| record.event == event));
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
		System::set_block_number(1);

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
			XCMPTokenMessage::Transfer(x_currency_id.clone(), para_two_id(), BOB, 50)
		));

		let event = TestEvent::xtokens(RawEvent::TransferredToParachain(
			x_currency_id,
			ALICE,
			para_two_id(),
			BOB,
			50,
		));
		assert!(System::events().iter().any(|record| record.event == event));
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
		System::set_block_number(1);

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
			XCMPTokenMessage::Transfer(x_currency_id.clone(), para_two_id(), BOB, 50)
		));

		let event = TestEvent::xtokens(RawEvent::TransferredToParachain(
			x_currency_id,
			ALICE,
			para_two_id(),
			BOB,
			50,
		));
		assert!(System::events().iter().any(|record| record.event == event));
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

#[test]
fn handle_downward_message_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let dest: polkadot_core_primitives::AccountId = [0; 32].into();
		let msg = DownwardMessage::TransferInto(dest.clone(), 50, [0; 32]);
		XTokens::handle_downward_message(&msg);

		let dest_account = convert_hack(&dest);
		assert_eq!(Tokens::free_balance(CurrencyId::DOT, &dest_account), 50);

		let event = TestEvent::xtokens(RawEvent::ReceivedTransferFromRelayChain(dest_account, 50));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn handle_xcmp_message_works_for_relay_chain_tokens() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let x_currency_id = XCurrencyId::new(ChainId::RelayChain, vec![0]);
		let msg = XCMPTokenMessage::Transfer(x_currency_id.clone(), MockParaId::get(), ALICE, 50);
		XTokens::handle_xcmp_message(para_one_id(), &msg);

		assert_eq!(Tokens::free_balance(CurrencyId::DOT, &ALICE), 50);

		let event = TestEvent::xtokens(RawEvent::ReceivedTransferFromParachain(
			x_currency_id,
			para_one_id(),
			ALICE,
			50,
		));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn handle_xcmp_message_works_for_owned_parachain_tokens() {
	// transfer from para_one to para_two
	ExtBuilder::default()
		.balances(vec![(para_one_account(), CurrencyId::Owned, 100)])
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			let x_currency_id = XCurrencyId::new(ChainId::ParaChain(MockParaId::get()), CurrencyId::Owned.into());
			let msg = XCMPTokenMessage::Transfer(x_currency_id.clone(), para_two_id(), ALICE, 50);
			XTokens::handle_xcmp_message(para_one_id(), &msg);

			assert_eq!(Tokens::free_balance(CurrencyId::Owned, &para_one_account()), 50);
			assert_eq!(Tokens::free_balance(CurrencyId::Owned, &para_two_account()), 50);

			MockXCMPMessageSender::msg_sent(para_two_id(), msg);

			let event = TestEvent::xtokens(RawEvent::ReceivedTransferFromParachain(
				x_currency_id,
				para_one_id(),
				ALICE,
				50,
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn handle_xcmp_message_works_for_owned_parachain_tokens_and_self_parachain_as_dest() {
	// transfer from para_one to self parachain
	ExtBuilder::default()
		.balances(vec![(para_one_account(), CurrencyId::Owned, 100)])
		.build()
		.execute_with(|| {
			System::set_block_number(1);

			let x_currency_id = XCurrencyId::new(ChainId::ParaChain(MockParaId::get()), CurrencyId::Owned.into());
			let msg = XCMPTokenMessage::Transfer(x_currency_id.clone(), MockParaId::get(), ALICE, 50);
			XTokens::handle_xcmp_message(para_one_id(), &msg);

			assert_eq!(Tokens::free_balance(CurrencyId::Owned, &para_one_account()), 50);
			assert_eq!(Tokens::free_balance(CurrencyId::Owned, &ALICE), 50);

			let event = TestEvent::xtokens(RawEvent::ReceivedTransferFromParachain(
				x_currency_id,
				para_one_id(),
				ALICE,
				50,
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn handle_xcmp_message_works_for_owned_parachain_tokens_with_invalid_currency() {
	ExtBuilder::default()
		.balances(vec![(para_one_account(), CurrencyId::Owned, 100)])
		.build()
		.execute_with(|| {
			fn handle() -> sp_std::result::Result<(), ()> {
				let x_currency_id = XCurrencyId::new(ChainId::ParaChain(MockParaId::get()), unknown_currency_id());
				let msg = XCMPTokenMessage::Transfer(x_currency_id.clone(), MockParaId::get(), ALICE, 50);
				XTokens::handle_xcmp_message(para_one_id(), &msg);
				Err(())
			}
			assert_noop!(handle(), ());
		});
}

#[test]
fn handle_xcmp_message_works_for_non_owned_known_parachain_tokens() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(para_one_id()), CurrencyId::BTC.into());
		let msg = XCMPTokenMessage::Transfer(x_currency_id.clone(), MockParaId::get(), ALICE, 50);
		XTokens::handle_xcmp_message(para_one_id(), &msg);

		assert_eq!(Tokens::free_balance(CurrencyId::BTC, &ALICE), 50);

		let event = TestEvent::xtokens(RawEvent::ReceivedTransferFromParachain(
			x_currency_id,
			para_one_id(),
			ALICE,
			50,
		));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn handle_xcmp_message_works_for_non_owned_unknown_parachain_tokens() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let x_currency_id = XCurrencyId::new(ChainId::ParaChain(para_one_id()), unknown_currency_id());
		let msg = XCMPTokenMessage::Transfer(x_currency_id.clone(), MockParaId::get(), ALICE, 50);
		XTokens::handle_xcmp_message(para_one_id(), &msg);

		assert_eq!(XTokens::unknown_balances(ALICE, unknown_currency_id()), 50);

		let event = TestEvent::xtokens(RawEvent::ReceivedTransferFromParachain(
			x_currency_id,
			para_one_id(),
			ALICE,
			50,
		));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}
