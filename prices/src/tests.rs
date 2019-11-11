//! Unit tests for the prices module.

#![cfg(test)]

use super::*;
use mock::{ExtBuilder, PricesModule};

#[test]
fn get_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(PricesModule::get_price(1, 2), Some(2));
		assert_eq!(PricesModule::get_price(1, 3), None);
	});
}
