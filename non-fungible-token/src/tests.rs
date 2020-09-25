//! Unit tests for the non-fungible-token module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	ExtBuilder, NonFungibleTokenModule, Runtime, ALICE, BOB, CLASS_ID, CLASS_ID_NOT_EXIST, TOKEN_ID, TOKEN_ID_NOT_EXIST,
};

#[test]
fn create_class_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
	});
}

#[test]
fn create_class_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		NextClassId::<Runtime>::mutate(|id| *id = <Runtime as Trait>::ClassId::max_value());
		assert_noop!(
			NonFungibleTokenModule::create_class(&ALICE, vec![1], ()),
			Error::<Runtime>::NoAvailableClassId
		);
	});
}

#[test]
fn mint_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
	});
}

#[test]
fn mint_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		Classes::<Runtime>::mutate(CLASS_ID, |class_info| {
			class_info.as_mut().unwrap().total_issuance = <Runtime as Trait>::TokenId::max_value();
		});
		// can't use assert_noop. modify tokenid.
		assert_eq!(
			NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()),
			Err(Error::<Runtime>::NumOverflow.into())
		);

		NextTokenId::<Runtime>::mutate(|id| *id = <Runtime as Trait>::TokenId::max_value());
		assert_noop!(
			NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()),
			Error::<Runtime>::NoAvailableTokenId
		);
	});
}

#[test]
fn transfer_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::transfer(&BOB, &BOB, (CLASS_ID, TOKEN_ID)));
		assert_ok!(NonFungibleTokenModule::transfer(&BOB, &ALICE, (CLASS_ID, TOKEN_ID)));
		assert_ok!(NonFungibleTokenModule::transfer(&ALICE, &BOB, (CLASS_ID, TOKEN_ID)));
	});
}

#[test]
fn transfer_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
		assert_noop!(
			NonFungibleTokenModule::transfer(&BOB, &ALICE, (CLASS_ID, TOKEN_ID_NOT_EXIST)),
			Error::<Runtime>::NoPermission
		);
		assert_noop!(
			NonFungibleTokenModule::transfer(&ALICE, &BOB, (CLASS_ID, TOKEN_ID)),
			Error::<Runtime>::NoPermission
		);
		// can't use assert_noop. modify tokenid.
		assert_eq!(
			NonFungibleTokenModule::mint(&BOB, CLASS_ID_NOT_EXIST, vec![1], ()),
			Err(Error::<Runtime>::ClassNotFound.into())
		);
	});
}

#[test]
fn burn_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::burn(&BOB, (CLASS_ID, TOKEN_ID)));
	});
}

#[test]
fn burn_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
		assert_noop!(
			NonFungibleTokenModule::burn(&BOB, (CLASS_ID, TOKEN_ID_NOT_EXIST)),
			Error::<Runtime>::TokenNotFound
		);

		// can't use assert_noop. remove token.
		assert_eq!(
			NonFungibleTokenModule::burn(&ALICE, (CLASS_ID, TOKEN_ID)),
			Err(Error::<Runtime>::NoPermission.into())
		);
	});

	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));

		Classes::<Runtime>::mutate(CLASS_ID, |class_info| {
			class_info.as_mut().unwrap().total_issuance = 0;
		});
		// can't use assert_noop. remove token.
		assert_eq!(
			NonFungibleTokenModule::burn(&BOB, (CLASS_ID, TOKEN_ID)),
			Err(Error::<Runtime>::NumOverflow.into())
		);
	});
}

#[test]
fn destroy_class_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::burn(&BOB, (CLASS_ID, TOKEN_ID)));
		assert_ok!(NonFungibleTokenModule::destroy_class(&ALICE, CLASS_ID));
	});
}

#[test]
fn destroy_class_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(NonFungibleTokenModule::create_class(&ALICE, vec![1], ()));
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
		assert_noop!(
			NonFungibleTokenModule::destroy_class(&ALICE, CLASS_ID_NOT_EXIST),
			Error::<Runtime>::ClassNotFound
		);

		assert_noop!(
			NonFungibleTokenModule::destroy_class(&BOB, CLASS_ID),
			Error::<Runtime>::NoPermission
		);

		assert_noop!(
			NonFungibleTokenModule::destroy_class(&ALICE, CLASS_ID),
			Error::<Runtime>::CannotDestroyClass
		);

		assert_ok!(NonFungibleTokenModule::burn(&BOB, (CLASS_ID, TOKEN_ID)));
		assert_ok!(NonFungibleTokenModule::destroy_class(&ALICE, CLASS_ID));
		assert_eq!(Classes::<Runtime>::contains_key(CLASS_ID), false);
	});
}
