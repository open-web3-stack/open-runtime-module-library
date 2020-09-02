//! Unit tests for the xtokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
fn transfer_to_relay_chain_works() {
	ExtBuilder::default().build().execute_with(|| {});
}
