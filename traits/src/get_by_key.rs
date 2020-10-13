pub trait GetByKey<Key, Value> {
	fn get(k: &Key) -> Value;
}

#[macro_export]
macro_rules! get_by_key_type {
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

#[cfg(test)]
mod tests {
	use super::*;

	get_by_key_type! {
		pub Test: |k: u32| -> u32 {
			match k {
				1 => 1,
				_ => 2,
			}
		};
	}

	#[test]
	fn get_by_key_should_work() {
		assert_eq!(Test::get(&1), 1);
		assert_eq!(Test::get(&2), 2);
		assert_eq!(Test::get(&3), 2);
	}
}
