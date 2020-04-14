//! Unit tests for the prices module.

#![cfg(test)]

use super::*;

pub struct MockDataProvider;
impl DataProvider<u32, Price> for MockDataProvider {
	fn get(currency: &u32) -> Option<Price> {
		match currency {
			0 => Some(Price::from_parts(0)),
			1 => Some(Price::from_parts(1)),
			2 => Some(Price::from_parts(2)),
			_ => None,
		}
	}
}

type TestPriceProvider = DefaultPriceProvider<u32, MockDataProvider>;

#[test]
fn get_price_should_work() {
	assert_eq!(TestPriceProvider::get_price(1, 2), Some(Price::from_rational(1, 2)));
	assert_eq!(TestPriceProvider::get_price(2, 1), Some(Price::from_rational(2, 1)));
}

#[test]
fn price_is_none_should_not_panic() {
	assert_eq!(TestPriceProvider::get_price(3, 3), None);
	assert_eq!(TestPriceProvider::get_price(3, 1), None);
	assert_eq!(TestPriceProvider::get_price(1, 3), None);
}

#[test]
fn price_is_zero_should_not_panic() {
	assert_eq!(TestPriceProvider::get_price(0, 0), None);
	assert_eq!(TestPriceProvider::get_price(1, 0), None);
	assert_eq!(TestPriceProvider::get_price(0, 1), Some(Price::from_parts(0)));
}
