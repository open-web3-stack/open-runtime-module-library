//! Unit tests for the prices module.

#![cfg(test)]

use super::*;
use mock::{ExtBuilder, PricesModule};

#[test]
fn get_price_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(PricesModule::get_price(1, 2), Some(Price::from_rational(2, 1)));
	});
}

#[test]
fn price_is_none_should_not_panic() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(PricesModule::get_price(3, 3), None);
		assert_eq!(PricesModule::get_price(3, 1), None);
		assert_eq!(PricesModule::get_price(1, 3), None);
	});
}

#[test]
fn price_is_zero_should_not_panic() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(PricesModule::get_price(0, 0), None);
		assert_eq!(PricesModule::get_price(1, 0), Some(Price::from_parts(0)));
	});
}
