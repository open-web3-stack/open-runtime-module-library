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
use sp_runtime::AccountId32;

#[cfg(not(test))]
const MGA_TOKEN_ID: TokenId = 0;

#[cfg(test)]
const MGA_TOKEN_ID: TokenId = 3;

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

	// Benchmark `transfer_all` extrinsic with the worst possible conditions:
    // We can't reap the `from` account as ed is 0. But we can ensure that the `to` is created.
	// We will use `keep_alive` as `true` to take the !if branch in `reducible_balance`. This will however, still drop the `from` balance to 0 as ed is 0.
	transfer_all {
		let caller = whitelisted_caller();

		<MultiTokenCurrencyAdapter<T> as MultiTokenCurrencyExtended<_>>::mint(MGA_TOKEN_ID.into(), &caller, Balance::from(100u128).into())?;

		let recipient: T::AccountId = account("recipient", 0, SEED);
		let recipient_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(recipient.clone());

	}: transfer_all(RawOrigin::Signed(caller.clone()), recipient_lookup, MGA_TOKEN_ID.into(), true)
	verify {
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &caller).into(), Balance::zero());
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &recipient).into(), 100u128);
	}

	// Exactly the same as `transfer`, since ed is 0.
	// Benchmark `transfer_keep_alive` extrinsic with the worst possible conditions:
    // We can't reap the `from` account as ed is 0. But we can ensure that the `to` is created.
	transfer_keep_alive {
		let caller = whitelisted_caller();

		<MultiTokenCurrencyAdapter<T> as MultiTokenCurrencyExtended<_>>::mint(MGA_TOKEN_ID.into(), &caller, Balance::from(100u128).into())?;

		let recipient: T::AccountId = account("recipient", 0, SEED);
		let recipient_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(recipient.clone());

        // Transfer the entire amount
		let transfer_amount = 100;
	}: transfer_keep_alive(RawOrigin::Signed(caller.clone()), recipient_lookup, MGA_TOKEN_ID.into(), transfer_amount.into())
	verify {
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &caller).into(), Balance::zero());
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &recipient).into(), transfer_amount);
	}

	// Similar to `transfer`, except with root.
	// Benchmark `transfer_keep_alive` extrinsic with the worst possible conditions:
    // We can't reap the `from` account as ed is 0. But we can ensure that the `to` is created.
	force_transfer {
		let from: T::AccountId = account("from", 0, SEED);
		let from_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(from.clone());

		<MultiTokenCurrencyAdapter<T> as MultiTokenCurrencyExtended<_>>::mint(MGA_TOKEN_ID.into(), &from, Balance::from(100u128).into())?;

		let recipient: T::AccountId = account("recipient", 0, SEED);
		let recipient_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(recipient.clone());

        // Transfer the entire amount
		let transfer_amount = 100;
	}: force_transfer(RawOrigin::Root, from_lookup, recipient_lookup, MGA_TOKEN_ID.into(), transfer_amount.into())
	verify {
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &from).into(), Balance::zero());
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &recipient).into(), transfer_amount);
	}

	// Benchmark `set_balance` extrinsic with the worst possible conditions:
	// checking just increase should suffice for benchmarking as it is just another write
	set_balance {

		let who: T::AccountId = account("who", 0, SEED);
		let who_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who.clone());

		Tokens::<T>::set_balance(RawOrigin::Root.into(), who_lookup.clone(), MGA_TOKEN_ID.into(), 10u128.into(), 20u128.into())?;

	}: set_balance(RawOrigin::Root, who_lookup, MGA_TOKEN_ID.into(), 70u128.into(), 90u128.into())
	verify {
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &who).into(), 70u128);
		assert_eq!(Tokens::<T>::reserved_balance(MGA_TOKEN_ID.into(), &who).into(), 90u128);
	}

	// Benchmark `create` extrinsic with the worst possible conditions:
	// any non-zero amount would do
	create {

		let who: T::AccountId = account("who", 0, SEED);
		let who_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who.clone());

		let created_token_id: TokenId = <MultiTokenCurrencyAdapter<T> as MultiTokenCurrencyExtended<_>>::get_next_currency_id().into();

		assert_eq!(Tokens::<T>::free_balance(created_token_id.into(), &who).into(), 0);
		assert_eq!(<MultiTokenCurrencyAdapter<T> as MultiTokenCurrency<_>>::total_issuance(created_token_id.into()).into(), 0);

	}: create(RawOrigin::Root, who_lookup, 1000u128.into())
	verify {
		assert_eq!(Tokens::<T>::free_balance(created_token_id.into(), &who).into(), 1000u128);
		assert_eq!(<MultiTokenCurrencyAdapter<T> as MultiTokenCurrency<_>>::total_issuance(created_token_id.into()).into(), 1000u128);
		let next_token_id: TokenId = <MultiTokenCurrencyAdapter<T> as MultiTokenCurrencyExtended<_>>::get_next_currency_id().into();
		assert_eq!(next_token_id, created_token_id + 1u32);
	}

	// Benchmark `mint` extrinsic with the worst possible conditions:
	// checking just increase should suffice for benchmarking as it is just another write
	mint {

		let who: T::AccountId = account("who", 0, SEED);
		let who_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(who.clone());

		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &who).into(), 0u128);
		let total_issuance_before: Balance = <MultiTokenCurrencyAdapter<T> as MultiTokenCurrency<_>>::total_issuance(MGA_TOKEN_ID.into()).into();

	}: mint(RawOrigin::Root, MGA_TOKEN_ID.into(), who_lookup, 1000u128.into())
	verify {
		assert_eq!(Tokens::<T>::free_balance(MGA_TOKEN_ID.into(), &who).into(), 1000u128);
		assert_eq!( <MultiTokenCurrencyAdapter<T> as MultiTokenCurrency<_>>::total_issuance(MGA_TOKEN_ID.into()).into(), total_issuance_before + 1000u128);
	}


    impl_benchmark_test_suite!(Tokens, crate::mock::ExtBuilder::default().balances(vec![(AccountId32::new([99u8; 32]), 0, 20), (AccountId32::new([99u8; 32]), 1, 20), (AccountId32::new([99u8; 32]), 2, 20), (AccountId32::new([99u8; 32]), 3, 20)]).build(), crate::mock::Runtime)    

}

