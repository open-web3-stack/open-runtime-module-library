//! Unit tests for the non-fungible-token module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{ExtBuilder, NonFungibleTokenModule, Runtime, ALICE, BOB, CLASS_ID, TOKEN_ID, TOKEN_ID_NOT_EXIST};

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
			if let Some(info) = class_info {
				info.total_issuance = <Runtime as Trait>::TokenId::max_value();
			}
		});
		assert_noop!(
			NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()),
			Error::<Runtime>::NumOverflow
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
		NextClassId::<Runtime>::mutate(|id| *id = <Runtime as Trait>::ClassId::max_value());
		assert_ok!(NonFungibleTokenModule::mint(&BOB, CLASS_ID, vec![1], ()));
		assert_noop!(
			NonFungibleTokenModule::transfer(&BOB, &ALICE, (CLASS_ID, TOKEN_ID_NOT_EXIST)),
			Error::<Runtime>::TokenNotFound
		);
		assert_noop!(
			NonFungibleTokenModule::transfer(&ALICE, &ALICE, (CLASS_ID, TOKEN_ID)),
			Error::<Runtime>::NoPermission
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

		assert_noop!(
			NonFungibleTokenModule::burn(&ALICE, (CLASS_ID, TOKEN_ID)),
			Error::<Runtime>::NoPermission
		);

		Classes::<Runtime>::mutate(CLASS_ID, |class_info| {
			if let Some(info) = class_info {
				info.total_issuance = 0;
			}
		});
		assert_noop!(
			NonFungibleTokenModule::burn(&BOB, (CLASS_ID, TOKEN_ID)),
			Error::<Runtime>::NumOverflow
		);
	});
}
