#[doc(hidden)]
pub use codec;
#[doc(hidden)]
pub use frame_support;
use frame_support::Parameter;
#[doc(hidden)]
pub use paste;
#[doc(hidden)]
pub use scale_info;

pub trait ParameterStore {
	type AggregratedKeyValue: AggregratedKeyValue;

	fn get<KV, K>(key: K) -> Option<K::Value>
	where
		KV: AggregratedKeyValue,
		K: Key + Into<<KV as AggregratedKeyValue>::AggregratedKey>,
		<KV as AggregratedKeyValue>::AggregratedKey:
			Into<<<Self as ParameterStore>::AggregratedKeyValue as AggregratedKeyValue>::AggregratedKey>,
		<<Self as ParameterStore>::AggregratedKeyValue as AggregratedKeyValue>::AggregratedValue:
			TryInto<<KV as AggregratedKeyValue>::AggregratedValue>,
		<KV as AggregratedKeyValue>::AggregratedValue: TryInto<K::WrappedValue>;
}

pub trait Key {
	type Value;
	type WrappedValue: Into<Self::Value>;
}

pub trait AggregratedKeyValue: Parameter {
	type AggregratedKey: Parameter + codec::MaxEncodedLen;
	type AggregratedValue: Parameter + codec::MaxEncodedLen;

	fn into_parts(self) -> (Self::AggregratedKey, Option<Self::AggregratedValue>);
}

/// Define parameters key value types.
/// Example:
///
/// ```
/// define_parameters! {
///     pub Pallet = {
///         Key1: u64 = 0,
///         Key2(u32): u32 = 1,
///         Key3((u8, u8)): u128 = 2,
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_parameters {
	(
		$vis:vis $name:ident = {
			$(
				$key_name:ident $( ($key_para: ty) )? : $value_type:ty = $index:expr
			),+ $(,)?
		}
	) => {
		$crate::parameters::paste::item! {
			#[derive(
				Clone,
				PartialEq,
				Eq,
				$crate::parameters::codec::Encode,
				$crate::parameters::codec::Decode,
				$crate::parameters::codec::MaxEncodedLen,
				$crate::parameters::frame_support::RuntimeDebug,
				$crate::parameters::scale_info::TypeInfo
			)]
			$vis enum $name {
				$(
					#[codec(index = $index)]
					$key_name($key_name, Option<$value_type>),
				)*
			}

			#[derive(
				Clone,
				PartialEq,
				Eq,
				$crate::parameters::codec::Encode,
				$crate::parameters::codec::Decode,
				$crate::parameters::codec::MaxEncodedLen,
				$crate::parameters::frame_support::RuntimeDebug,
				$crate::parameters::scale_info::TypeInfo
			)]
			$vis enum [<$name Key>] {
				$(
					#[codec(index = $index)]
					$key_name($key_name),
				)*
			}

			#[derive(
				Clone,
				PartialEq,
				Eq,
				$crate::parameters::codec::Encode,
				$crate::parameters::codec::Decode,
				$crate::parameters::codec::MaxEncodedLen,
				$crate::parameters::frame_support::RuntimeDebug,
				$crate::parameters::scale_info::TypeInfo
			)]
			$vis enum [<$name Value>] {
				$(
					#[codec(index = $index)]
					$key_name($value_type),
				)*
			}

			impl $crate::parameters::AggregratedKeyValue for $name {
				type AggregratedKey = [<$name Key>];
				type AggregratedValue = [<$name Value>];

				fn into_parts(self) -> (Self::AggregratedKey, Option<Self::AggregratedValue>) {
					match self {
						$(
							$name::$key_name(key, value) => ([<$name Key>]::$key_name(key), value.map([<$name Value>]::$key_name)),
						)*
					}
				}
			}

			$(
				#[derive(
					Clone,
					PartialEq,
					Eq,
					$crate::parameters::codec::Encode,
					$crate::parameters::codec::Decode,
					$crate::parameters::codec::MaxEncodedLen,
					$crate::parameters::frame_support::RuntimeDebug,
					$crate::parameters::scale_info::TypeInfo
				)]
				$vis struct $key_name( $(pub $key_para)? );

				impl $crate::parameters::Key for $key_name {
					type Value = $value_type;
					type WrappedValue = [<$key_name Value>];
				}

				impl From<$key_name> for [<$name Key>] {
					fn from(key: $key_name) -> Self {
						[<$name Key>]::$key_name(key)
					}
				}

				$vis struct [<$key_name Value>](pub $value_type);

				impl From<[<$key_name Value>]> for [<$name Value>] {
					fn from(value: [<$key_name Value>]) -> Self {
						[<$name Value>]::$key_name(value.0)
					}
				}

				impl From<($key_name, $value_type)> for $name {
					fn from((key, value): ($key_name, $value_type)) -> Self {
						$name::$key_name(key, Some(value))
					}
				}

				impl From<$key_name> for $name {
					fn from(key: $key_name) -> Self {
						$name::$key_name(key, None)
					}
				}

				impl TryFrom<[<$name Value>]> for [<$key_name Value>] {
					type Error = ();

					fn try_from(value: [<$name Value>]) -> Result<Self, Self::Error> {
						match value {
							[<$name Value>]::$key_name(value) => Ok([<$key_name Value>](value)),
							_ => Err(()),
						}
					}
				}

				impl From<[<$key_name Value>]> for $value_type {
					fn from(value: [<$key_name Value>]) -> Self {
						value.0
					}
				}
			)*
		}
	};
}

/// Define aggregrated parameters types.
///
/// Example:
/// ```
/// mod pallet1 {
///     define_parameters! {
///         pub Pallet = {
///             Key1: u64 = 0,
///             Key2(u32): u32 = 1,
///             Key3((u8, u8)): u128 = 2,
///         }
///     }
/// }
///
/// mod pallet2 {
///     define_parameters! {
///         pub Pallet = {
///             Key1: u64 = 0,
///             Key2(u32): u32 = 1,
///             Key3((u8, u8)): u128 = 2,
///         }
///     }
/// }
///
/// define_aggregrated_parameters! {
///     pub AggregratedPallet = {
///         Pallet1: pallet1::Pallet = 0,
///         Pallet2: pallet2::Pallet = 1,
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_aggregrated_parameters {
	(
		$vis:vis $name:ident = {
			$(
				$parameter_name:ident: $parameter_type:ty = $index:expr
			),+ $(,)?
		}
	) => {
		$crate::parameters::paste::item! {
			#[derive(
				Clone,
				PartialEq,
				Eq,
				$crate::parameters::codec::Encode,
				$crate::parameters::codec::Decode,
				$crate::parameters::codec::MaxEncodedLen,
				$crate::parameters::frame_support::RuntimeDebug,
				$crate::parameters::scale_info::TypeInfo
			)]
			$vis enum $name {
				$(
					#[codec(index = $index)]
					$parameter_name($parameter_type),
				)*
			}

			#[derive(
				Clone,
				PartialEq,
				Eq,
				$crate::parameters::codec::Encode,
				$crate::parameters::codec::Decode,
				$crate::parameters::codec::MaxEncodedLen,
				$crate::parameters::frame_support::RuntimeDebug,
				$crate::parameters::scale_info::TypeInfo
			)]
			$vis enum [<$name Key>] {
				$(
					#[codec(index = $index)]
					$parameter_name(<$parameter_type as $crate::parameters::AggregratedKeyValue>::AggregratedKey),
				)*
			}

			#[derive(
				Clone,
				PartialEq,
				Eq,
				$crate::parameters::codec::Encode,
				$crate::parameters::codec::Decode,
				$crate::parameters::codec::MaxEncodedLen,
				$crate::parameters::frame_support::RuntimeDebug,
				$crate::parameters::scale_info::TypeInfo
			)]
			$vis enum [<$name Value>] {
				$(
					#[codec(index = $index)]
					$parameter_name(<$parameter_type as $crate::parameters::AggregratedKeyValue>::AggregratedValue),
				)*
			}

			impl $crate::parameters::AggregratedKeyValue for $name {
				type AggregratedKey = [<$name Key>];
				type AggregratedValue = [<$name Value>];

				fn into_parts(self) -> (Self::AggregratedKey, Option<Self::AggregratedValue>) {
					match self {
						$(
							$name::$parameter_name(parameter) => {
								let (key, value) = parameter.into_parts();
								([<$name Key>]::$parameter_name(key), value.map([<$name Value>]::$parameter_name))
							},
						)*
					}
				}
			}

			$(
				impl From<<$parameter_type as $crate::parameters::AggregratedKeyValue>::AggregratedKey> for [<$name Key>] {
					fn from(key: <$parameter_type as $crate::parameters::AggregratedKeyValue>::AggregratedKey) -> Self {
						[<$name Key>]::$parameter_name(key)
					}
				}

				impl TryFrom<[<$name Value>]> for <$parameter_type as $crate::parameters::AggregratedKeyValue>::AggregratedValue {
					type Error = ();

					fn try_from(value: [<$name Value>]) -> Result<Self, Self::Error> {
						match value {
							[<$name Value>]::$parameter_name(value) => Ok(value),
							_ => Err(()),
						}
					}
				}
			)*
		}
	};
}
