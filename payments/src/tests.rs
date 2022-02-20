use crate::{
	mock::*,
	types::{PaymentDetail, PaymentState},
	Payment as PaymentStore, PaymentHandler,
};
use frame_support::{assert_noop, assert_ok, storage::with_transaction};
use orml_traits::MultiCurrency;
use sp_runtime::{Percent, TransactionOutcome};

fn last_event() -> Event {
	System::events().pop().expect("Event expected").event
}

#[test]
fn test_pay_works() {
	new_test_ext().execute_with(|| {
		// the payment amount should not be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);

		// should be able to create a payment with available balance
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));
		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentCreated {
				from: PAYMENT_CREATOR,
				asset: CURRENCY_ID,
				amount: 20
			}
			.into()
		);
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);
		// the payment amount should be reserved correctly
		// the amount + incentive should be removed from the sender account
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 78);
		// the incentive amount should be reserved in the sender account
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_CREATOR), 80);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);
		// the transferred amount should be reserved in the recipent account
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 20);

		// the payment should not be overwritten
		assert_noop!(
			Payment::pay(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT, CURRENCY_ID, 20,),
			crate::Error::<Test>::PaymentAlreadyInProcess
		);

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);
	});
}

#[test]
fn test_cancel_works() {
	new_test_ext().execute_with(|| {
		// should be able to create a payment with available balance
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			40,
		));
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 40,
				incentive_amount: 4,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);
		// the payment amount should be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 56);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);

		// cancel should fail when called by user
		assert_noop!(
			Payment::cancel(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT),
			crate::Error::<Test>::InvalidPayment
		);

		// cancel should succeed when caller is the recipent
		assert_ok!(Payment::cancel(Origin::signed(PAYMENT_RECIPENT), PAYMENT_CREATOR));
		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentCancelled {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT
			}
			.into()
		);
		// the payment amount should be released back to creator
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be released from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);
	});
}

#[test]
fn test_release_works() {
	new_test_ext().execute_with(|| {
		// should be able to create a payment with available balance
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			40,
		));
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 40,
				incentive_amount: 4,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);
		// the payment amount should be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 56);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);

		// should succeed for valid payment
		assert_ok!(Payment::release(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT));
		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentReleased {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT
			}
			.into()
		);
		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 60);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 40);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be deleted from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);

		// should be able to create another payment since previous is released
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			40,
		));
		// the payment amount should be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 16);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 40);
	});
}

#[test]
fn test_set_state_payment_works() {
	new_test_ext().execute_with(|| {
		// should be able to create a payment with available balance
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			40,
		));

		// should fail for non whitelisted caller
		assert_noop!(
			Payment::resolve_cancel_payment(Origin::signed(PAYMENT_CREATOR), PAYMENT_CREATOR, PAYMENT_RECIPENT,),
			crate::Error::<Test>::InvalidAction
		);

		// should be able to release a payment
		assert_ok!(Payment::resolve_release_payment(
			Origin::signed(RESOLVER_ACCOUNT),
			PAYMENT_CREATOR,
			PAYMENT_RECIPENT,
		));
		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentReleased {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT
			}
			.into()
		);

		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 60);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 40);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be removed from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);

		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			40,
		));

		// should be able to cancel a payment
		assert_ok!(Payment::resolve_cancel_payment(
			Origin::signed(RESOLVER_ACCOUNT),
			PAYMENT_CREATOR,
			PAYMENT_RECIPENT,
		));
		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentCancelled {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT
			}
			.into()
		);

		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 60);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 40);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be released from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);
	});
}

#[test]
fn test_charging_fee_payment_works() {
	new_test_ext().execute_with(|| {
		// should be able to create a payment with available balance
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT_FEE_CHARGED,
			CURRENCY_ID,
			40,
		));
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT_FEE_CHARGED),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 40,
				incentive_amount: 4,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 4)),
				remark: None
			})
		);
		// the payment amount should be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 52);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 0);

		// should succeed for valid payment
		assert_ok!(Payment::release(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT_FEE_CHARGED
		));
		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 56);
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_CREATOR), 56);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 40);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &FEE_RECIPIENT_ACCOUNT), 4);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);
	});
}

#[test]
fn test_charging_fee_payment_works_when_canceled() {
	new_test_ext().execute_with(|| {
		// should be able to create a payment with available balance
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT_FEE_CHARGED,
			CURRENCY_ID,
			40,
		));
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT_FEE_CHARGED),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 40,
				incentive_amount: 4,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 4)),
				remark: None
			})
		);
		// the payment amount should be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 52);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 0);

		// should succeed for valid payment
		assert_ok!(Payment::cancel(
			Origin::signed(PAYMENT_RECIPENT_FEE_CHARGED),
			PAYMENT_CREATOR
		));
		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 0);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &FEE_RECIPIENT_ACCOUNT), 0);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);
	});
}

#[test]
fn test_pay_with_remark_works() {
	new_test_ext().execute_with(|| {
		// should be able to create a payment with available balance
		assert_ok!(Payment::pay_with_remark(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
			vec![1u8; 10].try_into().unwrap()
		));
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: Some(vec![1u8; 10].try_into().unwrap())
			})
		);
		// the payment amount should be reserved correctly
		// the amount + incentive should be removed from the sender account
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 78);
		// the incentive amount should be reserved in the sender account
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_CREATOR), 80);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);
		// the transferred amount should be reserved in the recipent account
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 20);

		// the payment should not be overwritten
		assert_noop!(
			Payment::pay(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT, CURRENCY_ID, 20,),
			crate::Error::<Test>::PaymentAlreadyInProcess
		);

		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentCreated {
				from: PAYMENT_CREATOR,
				asset: CURRENCY_ID,
				amount: 20
			}
			.into()
		);
	});
}

#[test]
fn test_do_not_overwrite_logic_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));

		assert_noop!(
			Payment::pay(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT, CURRENCY_ID, 20,),
			crate::Error::<Test>::PaymentAlreadyInProcess
		);

		// set payment state to NeedsReview
		PaymentStore::<Test>::insert(
			PAYMENT_CREATOR,
			PAYMENT_RECIPENT,
			PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::NeedsReview,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None,
			},
		);

		// the payment should not be overwritten
		assert_noop!(
			Payment::pay(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT, CURRENCY_ID, 20,),
			crate::Error::<Test>::PaymentNeedsReview
		);
	});
}

#[test]
fn test_request_refund() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));

		assert_ok!(Payment::request_refund(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT
		));

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::RefundRequested(601u64.into()),
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);

		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentCreatorRequestedRefund {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT,
				expiry: 601u64.into()
			}
			.into()
		);
	});
}

#[test]
fn test_claim_refund() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));

		// cannot claim refund unless payment is in requested refund state
		assert_noop!(
			Payment::claim_refund(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT),
			crate::Error::<Test>::RefundNotRequested
		);

		assert_ok!(Payment::request_refund(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT
		));

		// cannot cancel before the dispute period has passed
		assert_noop!(
			Payment::claim_refund(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT),
			crate::Error::<Test>::DisputePeriodNotPassed
		);

		run_to_block(700);
		assert_ok!(Payment::claim_refund(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT));

		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentCancelled {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT
			}
			.into()
		);
		// the payment amount should be released back to creator
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be released from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);
	});
}

#[test]
fn test_dispute_refund() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));

		// cannot dispute if refund is not requested
		assert_noop!(
			Payment::dispute_refund(Origin::signed(PAYMENT_RECIPENT), PAYMENT_CREATOR),
			crate::Error::<Test>::InvalidAction
		);
		// creator requests a refund
		assert_ok!(Payment::request_refund(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT
		));
		// recipient disputes the refund request
		assert_ok!(Payment::dispute_refund(
			Origin::signed(PAYMENT_RECIPENT),
			PAYMENT_CREATOR
		));
		// payment cannot be claimed after disputed
		assert_noop!(
			Payment::claim_refund(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT),
			crate::Error::<Test>::PaymentNeedsReview
		);

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::NeedsReview,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);

		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentRefundDisputed {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT,
			}
			.into()
		);
	});
}

#[test]
fn test_request_payment() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::request_payment(
			Origin::signed(PAYMENT_RECIPENT),
			PAYMENT_CREATOR,
			CURRENCY_ID,
			20,
		));

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 0_u128,
				state: PaymentState::PaymentRequested,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);

		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentRequestCreated {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT,
			}
			.into()
		);
	});
}

#[test]
fn test_requested_payment_cannot_be_released() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::request_payment(
			Origin::signed(PAYMENT_RECIPENT),
			PAYMENT_CREATOR,
			CURRENCY_ID,
			20,
		));

		// requested payment cannot be released
		assert_noop!(
			Payment::release(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT),
			crate::Error::<Test>::InvalidAction
		);
	});
}

#[test]
fn test_requested_payment_can_be_cancelled_by_requestor() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::request_payment(
			Origin::signed(PAYMENT_RECIPENT),
			PAYMENT_CREATOR,
			CURRENCY_ID,
			20,
		));

		assert_ok!(Payment::cancel(Origin::signed(PAYMENT_RECIPENT), PAYMENT_CREATOR));

		// the request should be removed from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);
	});
}

#[test]
fn test_accept_and_pay() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::request_payment(
			Origin::signed(PAYMENT_RECIPENT),
			PAYMENT_CREATOR,
			CURRENCY_ID,
			20,
		));

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 0_u128,
				state: PaymentState::PaymentRequested,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: None
			})
		);

		assert_ok!(Payment::accept_and_pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
		));

		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 80);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 20);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be deleted from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);

		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentRequestCompleted {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT,
			}
			.into()
		);
	});
}

#[test]
fn test_accept_and_pay_should_fail_for_non_payment_requested() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));

		assert_noop!(
			Payment::accept_and_pay(Origin::signed(PAYMENT_CREATOR), PAYMENT_RECIPENT,),
			crate::Error::<Test>::InvalidAction
		);
	});
}

#[test]
fn test_accept_and_pay_should_charge_fee_correctly() {
	new_test_ext().execute_with(|| {
		assert_ok!(Payment::request_payment(
			Origin::signed(PAYMENT_RECIPENT_FEE_CHARGED),
			PAYMENT_CREATOR,
			CURRENCY_ID,
			20,
		));

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT_FEE_CHARGED),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 0_u128,
				state: PaymentState::PaymentRequested,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 2)),
				remark: None
			})
		);

		assert_ok!(Payment::accept_and_pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT_FEE_CHARGED,
		));

		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 78);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 20);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &FEE_RECIPIENT_ACCOUNT), 2);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be deleted from storage
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT_FEE_CHARGED),
			None
		);

		assert_eq!(
			last_event(),
			crate::Event::<Test>::PaymentRequestCompleted {
				from: PAYMENT_CREATOR,
				to: PAYMENT_RECIPENT_FEE_CHARGED,
			}
			.into()
		);
	});
}

#[test]
#[should_panic(expected = "Require transaction not called within with_transaction")]
fn test_create_payment_does_not_work_without_transaction() {
	new_test_ext().execute_with(|| {
		assert_ok!(<Payment as PaymentHandler<Test>>::create_payment(
			PAYMENT_CREATOR,
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
			PaymentState::Created,
			Percent::from_percent(0),
			None,
		));
	});
}

#[test]
fn test_create_payment_works() {
	new_test_ext().execute_with(|| {
		// the payment amount should not be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);

		// should be able to create a payment with available balance within a
		// transaction
		assert_ok!(with_transaction(|| TransactionOutcome::Commit({
			<Payment as PaymentHandler<Test>>::create_payment(
				PAYMENT_CREATOR,
				PAYMENT_RECIPENT,
				CURRENCY_ID,
				20,
				PaymentState::Created,
				Percent::from_percent(10),
				Some(vec![1u8; 10].try_into().unwrap()),
			)
		})));

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: Some(vec![1u8; 10].try_into().unwrap()),
			})
		);

		// the payment should not be overwritten
		assert_noop!(
			with_transaction(|| TransactionOutcome::Commit({
				<Payment as PaymentHandler<Test>>::create_payment(
					PAYMENT_CREATOR,
					PAYMENT_RECIPENT,
					CURRENCY_ID,
					20,
					PaymentState::Created,
					Percent::from_percent(10),
					Some(vec![1u8; 10].try_into().unwrap()),
				)
			})),
			crate::Error::<Test>::PaymentAlreadyInProcess
		);

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: Some(vec![1u8; 10].try_into().unwrap()),
			})
		);
	});
}

#[test]
fn test_reserve_payment_amount_works() {
	new_test_ext().execute_with(|| {
		// the payment amount should not be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);

		// should be able to create a payment with available balance within a
		// transaction
		assert_ok!(with_transaction(|| TransactionOutcome::Commit({
			<Payment as PaymentHandler<Test>>::create_payment(
				PAYMENT_CREATOR,
				PAYMENT_RECIPENT,
				CURRENCY_ID,
				20,
				PaymentState::Created,
				Percent::from_percent(10),
				Some(vec![1u8; 10].try_into().unwrap()),
			)
		})));

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: Some(vec![1u8; 10].try_into().unwrap()),
			})
		);

		assert_ok!(with_transaction(|| TransactionOutcome::Commit({
			<Payment as PaymentHandler<Test>>::reserve_payment_amount(
				&PAYMENT_CREATOR,
				&PAYMENT_RECIPENT,
				PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT).unwrap(),
			)
		})));
		// the payment amount should be reserved correctly
		// the amount + incentive should be removed from the sender account
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 78);
		// the incentive amount should be reserved in the sender account
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_CREATOR), 80);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);
		// the transferred amount should be reserved in the recipent account
		assert_eq!(Tokens::total_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 20);

		// the payment should not be overwritten
		assert_noop!(
			with_transaction(|| TransactionOutcome::Commit({
				<Payment as PaymentHandler<Test>>::create_payment(
					PAYMENT_CREATOR,
					PAYMENT_RECIPENT,
					CURRENCY_ID,
					20,
					PaymentState::Created,
					Percent::from_percent(10),
					Some(vec![1u8; 10].try_into().unwrap()),
				)
			})),
			crate::Error::<Test>::PaymentAlreadyInProcess
		);

		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT),
			Some(PaymentDetail {
				asset: CURRENCY_ID,
				amount: 20,
				incentive_amount: 2,
				state: PaymentState::Created,
				resolver_account: RESOLVER_ACCOUNT,
				fee_detail: Some((FEE_RECIPIENT_ACCOUNT, 0)),
				remark: Some(vec![1u8; 10].try_into().unwrap()),
			})
		);
	});
}

#[test]
fn test_settle_payment_works_for_cancel() {
	new_test_ext().execute_with(|| {
		// the payment amount should not be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);

		// should be able to create a payment with available balance within a
		// transaction
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));

		assert_ok!(with_transaction(|| TransactionOutcome::Commit({
			<Payment as PaymentHandler<Test>>::settle_payment(
				PAYMENT_CREATOR,
				PAYMENT_RECIPENT,
				Percent::from_percent(0),
			)
		})));

		// the payment amount should be released back to creator
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be released from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);
	});
}

#[test]
fn test_settle_payment_works_for_release() {
	new_test_ext().execute_with(|| {
		// the payment amount should not be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 0);

		// should be able to create a payment with available balance within a
		// transaction
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT,
			CURRENCY_ID,
			20,
		));

		assert_ok!(with_transaction(|| TransactionOutcome::Commit({
			<Payment as PaymentHandler<Test>>::settle_payment(
				PAYMENT_CREATOR,
				PAYMENT_RECIPENT,
				Percent::from_percent(100),
			)
		})));

		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 80);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT), 20);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be deleted from storage
		assert_eq!(PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT), None);
	});
}

#[test]
fn test_settle_payment_works_for_70_30() {
	new_test_ext().execute_with(|| {
		// the payment amount should not be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 0);

		// should be able to create a payment with available balance within a
		// transaction
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT_FEE_CHARGED,
			CURRENCY_ID,
			10,
		));

		assert_ok!(with_transaction(|| TransactionOutcome::Commit({
			<Payment as PaymentHandler<Test>>::settle_payment(
				PAYMENT_CREATOR,
				PAYMENT_RECIPENT_FEE_CHARGED,
				Percent::from_percent(70),
			)
		})));

		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 92);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 7);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &FEE_RECIPIENT_ACCOUNT), 1);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be deleted from storage
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT_FEE_CHARGED),
			None
		);
	});
}

#[test]
fn test_settle_payment_works_for_50_50() {
	new_test_ext().execute_with(|| {
		// the payment amount should not be reserved
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 100);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 0);

		// should be able to create a payment with available balance within a
		// transaction
		assert_ok!(Payment::pay(
			Origin::signed(PAYMENT_CREATOR),
			PAYMENT_RECIPENT_FEE_CHARGED,
			CURRENCY_ID,
			10,
		));

		assert_ok!(with_transaction(|| TransactionOutcome::Commit({
			<Payment as PaymentHandler<Test>>::settle_payment(
				PAYMENT_CREATOR,
				PAYMENT_RECIPENT_FEE_CHARGED,
				Percent::from_percent(50),
			)
		})));

		// the payment amount should be transferred
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_CREATOR), 94);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &PAYMENT_RECIPENT_FEE_CHARGED), 5);
		assert_eq!(Tokens::free_balance(CURRENCY_ID, &FEE_RECIPIENT_ACCOUNT), 1);
		assert_eq!(Tokens::total_issuance(CURRENCY_ID), 100);

		// should be deleted from storage
		assert_eq!(
			PaymentStore::<Test>::get(PAYMENT_CREATOR, PAYMENT_RECIPENT_FEE_CHARGED),
			None
		);
	});
}
