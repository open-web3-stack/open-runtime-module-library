use sp_runtime::DispatchResult;

/// Data provider with ability to provide data with no-op, and provide all data.
pub trait DataFeeder<Key, Value, AccountId>: DataProvider<Key, Value> {
	/// Provide a new value for a given key from an operator
	fn feed_value(who: AccountId, key: Key, value: Value) -> DispatchResult;
}

/// A simple trait to provide data
pub trait DataProvider<Key, Value> {
	/// Get data by key
	fn get(key: &Key) -> Option<Value>;
}

/// A simple trait to provide data from a given ProviderId
pub trait MultiDataProvider<ProviderId, Key, Value> {
	/// Provide a new value for given key and ProviderId from an operator
	fn get(source: ProviderId, key: &Key) -> Option<Value>;
}

#[allow(dead_code)] // rust cannot defect usage in macro_rules
fn median<T: Ord + Clone>(mut items: Vec<T>) -> Option<T> {
	if items.is_empty() {
		return None;
	}

	let mid_index = items.len() / 2;
	// Won't panic as guarded items not empty case.
	let (_, median, _) = items.partition_at_index(mid_index);
	Some(median.clone())
}

#[macro_export]
macro_rules! create_median_value_data_provider {
	($name:ident, $key:ty, $value:ty, [$( $provider:ty ),*]) => {
		pub struct $name;
		impl DataProvider<$key, $value> for $name {
			fn get(key: &$key) -> Option<$value> {
				let mut values = vec![];
				$(
					if let Some(v) = <$provider>::get(&key) {
						values.push(v);
					}
				)*
				median(values)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_std::cell::RefCell;

	thread_local! {
		static MOCK_PRICE_1: RefCell<Option<u8>> = RefCell::new(None);
		static MOCK_PRICE_2: RefCell<Option<u8>> = RefCell::new(None);
		static MOCK_PRICE_3: RefCell<Option<u8>> = RefCell::new(None);
		static MOCK_PRICE_4: RefCell<Option<u8>> = RefCell::new(None);
	}

	macro_rules! mock_data_provider {
		($provider:ident, $price:ident) => {
			pub struct $provider;
			impl $provider {
				fn set_price(price: Option<u8>) {
					$price.with(|v| *v.borrow_mut() = price)
				}
			}
			impl DataProvider<u8, u8> for $provider {
				fn get(_: &u8) -> Option<u8> {
					$price.with(|v| *v.borrow())
				}
			}
		};
	}

	mock_data_provider!(Provider1, MOCK_PRICE_1);
	mock_data_provider!(Provider2, MOCK_PRICE_2);
	mock_data_provider!(Provider3, MOCK_PRICE_3);
	mock_data_provider!(Provider4, MOCK_PRICE_4);

	create_median_value_data_provider!(Providers, u8, u8, [Provider1, Provider2, Provider3, Provider4]);

	#[test]
	fn median_value_data_provider_works() {
		assert_eq!(Providers::get(&0), None);

		let data = vec![
			(vec![None, None, None, Some(1)], Some(1)),
			(vec![None, None, Some(2), Some(1)], Some(2)),
			(vec![Some(5), Some(2), None, Some(7)], Some(5)),
			(vec![Some(5), Some(13), Some(2), Some(7)], Some(7)),
		];

		for (values, target) in data {
			Provider1::set_price(values[0]);
			Provider2::set_price(values[1]);
			Provider3::set_price(values[2]);
			Provider4::set_price(values[3]);

			assert_eq!(Providers::get(&0), target);
		}
	}
}
