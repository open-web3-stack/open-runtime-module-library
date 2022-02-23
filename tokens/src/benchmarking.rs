// Since ed is zero DustWhitelist is ignored.
// And on_dust is never called

// transfer
// worst case
// Non-zero amount
// from has x to begin with
// All of it is transferred to to
// to should not have existed, and from will not die due to 0 ed... reaping wise
// since ed is 0, there will be no dust clear
// Note that both the accounts are rewritten in anycase
// So essentially the compete is between whether dec_ref is more expensive or accessing DustWhitelist, we assume that dec_ref is more expensive and let from fall to 0 for worst case.


// transfer_all
// worst case:
// Non-zero amount
// Even though keep_alive is irrelevant in the actual functioning, here we will set it to false, bcoz...\n
// 1. keep_alive = false; So that !if branch is taken, in reducible_balance
// -- To should be a new account
// -- From should be an existing account with some balance
// 2. Amount transfered should be greater than ed to create the other account
// - The remaining amount with from should be 0<x<ed, so that the dust is cleared and THEN the account is reaped...

// But what about ed? ED is 0.
// So the remaining amount with from should fall to 0, so that the account is reaped, but the dust cleaner wont be called.
// Note that both the accounts are rewritten in anycase
// So essentially the compete is between whether dec_ref is more expensive or accessing DustWhitelist, we assume that dec_ref is more expensive and let from fall to 0 for worst case.

// From will never be reaped as ed is 0!!!!!!!!!!!!!!!!

// Then the only diff between transfer and transfer_keep_alive is that amount is 1 less than what from starts with
// and that there should be no diff between the results...

// set_balance
// checking just increase should suffice for benchmarking as it is just another write
// (10, 20) -> (40, 90)

// create should just work with non-zero amount...

// mint should work with any existing token for any non-zero amount


#![cfg(feature = "runtime-benchmarks")]

use crate::{*, Pallet as Tokens};
use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_system::RawOrigin;

const MGA_TOKEN_ID: TokenId = 0;
const SEED: u32 = 0;

// We assume that ed is 0 universally.
// With Orml tokens this means that no account will be reaped, even if it falls to 0.
// It also naturally means that on_dust is never called.


benchmarks!{
	// Benchmark `transfer` extrinsic with the worst possible conditions:
    // We can't reap the `from` account as ed is 0. But we can ensure that the `to` is created.
	transfer {
		let caller = whitelisted_caller();


		<MultiTokenCurrencyAdapter<T> as MultiTokenCurrencyExtended<_>>::mint(MGA_TOKEN_ID.into(), &caller, Balance::from(100u128).into())?;

		let recipient: T::AccountId = account("recipient", 0, SEED);
		let recipient_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(recipient.clone());

        // Transfer the entire amount
		let transfer_amount = 100;
	}: transfer(RawOrigin::Signed(caller.clone()), recipient_lookup, MGA_TOKEN_ID.into(), transfer_amount.into())
	verify {
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &caller).into(), Balance::zero());
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &recipient).into(), transfer_amount);
	}

    impl_benchmark_test_suite!(Tokens, crate::mock::ExtBuilder::default().balances(vec![10, 0, 20]).build(), crate::mock::Runtime)    

}

