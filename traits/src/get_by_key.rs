/// A trait for querying a value by a key.
pub trait GetByKey<Key, Value> {
	/// Return the value.
	fn get(k: &Key) -> Value;
}

/// A trait for querying a option value by a key.
pub trait GetOptionValueByKey<Key, Value> {
	fn get(k: &Key) -> Option<Value>;
}

/// Default implementation for `GetOptionValueByKey`, return `None` always.
impl<Key, Value> GetOptionValueByKey<Key, Value> for () {
	fn get(_: &Key) -> Option<Value> {
		None
	}
}

/// Create new implementations of the `GetByKey` trait.
///
/// The implementation is typically used like a map or set.
///
/// Example:
/// ```ignore
/// use primitives::CurrencyId;
/// parameter_type_with_key! {
///     pub Rates: |currency_id: CurrencyId| -> u32 {
///         match currency_id {
///             CurrencyId::DOT => 1,
///             CurrencyId::KSM => 2,
///             _ => 3,
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! parameter_type_with_key {
	(
		pub $name:ident: |$k:ident: $key:ty| -> $value:ty $body:block;
	) => {
		pub struct $name;
		impl $crate::get_by_key::GetByKey<$key, $value> for $name {
			fn get($k: &$key) -> $value {
				$body
			}
		}
	};
}

/// Create new implementations of the `GetOptionValueByKey` trait.
///
/// The implementation is typically used like a map or set.
///
/// Example:
/// ```ignore
/// use primitives::CurrencyId;
/// parameter_type_with_key_option! {
///     pub Rates: |currency_id: CurrencyId| -> u32 {
///         match currency_id {
///             CurrencyId::DOT => Some(1),
///             CurrencyId::KSM => Some(2),
///             _ => None,
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! parameter_type_with_key_option {
	(
		pub $name:ident: |$k:ident: $key:ty| -> $value:ty $body:block;
	) => {
		pub struct $name;
		impl $crate::get_by_key::GetOptionValueByKey<$key, $value> for $name {
			fn get($k: &$key) -> Option<$value> {
				$body
			}
		}
	};
}

#[cfg(test)]
mod tests {
	use super::*;

	parameter_type_with_key! {
		pub Test: |k: u32| -> u32 {
			match k {
				1 => 1,
				_ => 2,
			}
		};
	}

	parameter_type_with_key_option! {
		pub Test2: |k: u32| -> u32 {
			match k {
				1 => Some(1),
				_ => None,
			}
		};
	}

	#[test]
	fn get_by_key_should_work() {
		assert_eq!(Test::get(&1), 1);
		assert_eq!(Test::get(&2), 2);
		assert_eq!(Test::get(&3), 2);
	}

	#[test]
	fn get_option_by_key_should_work() {
		assert_eq!(Test2::get(&1), Some(1));
		assert_eq!(Test2::get(&2), None);
	}
}
