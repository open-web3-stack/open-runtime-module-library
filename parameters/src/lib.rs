//! # Parameters
//! Offer a centra place to store and configure parameters.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_support::traits::EnsureOriginWithArg;
use orml_traits::parameters::{AggregratedKeyValue, Key, ParameterStore};

mod mock;
mod tests;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type AggregratedKeyValue: AggregratedKeyValue;
		type AdminOrigin: EnsureOriginWithArg<Self::RuntimeOrigin, KeyOf<Self>>;
	}

	type KeyOf<T> = <<T as Config>::AggregratedKeyValue as AggregratedKeyValue>::AggregratedKey;
	type ValueOf<T> = <<T as Config>::AggregratedKeyValue as AggregratedKeyValue>::AggregratedValue;

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Parameter is updated
		Updated { key_value: T::AggregratedKeyValue },
	}

	/// Stored parameters.
	///
	/// map KeyOf<T> => Option<ValueOf<T>>
	#[pallet::storage]
	pub type Parameters<T: Config> = StorageMap<_, Blake2_128Concat, KeyOf<T>, ValueOf<T>, OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set parameter
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn set_parameter(origin: OriginFor<T>, key_value: T::AggregratedKeyValue) -> DispatchResult {
			let (key, value) = key_value.clone().into_parts();

			T::AdminOrigin::ensure_origin(origin, &key)?;

			Parameters::<T>::mutate(key, |v| *v = value);

			Self::deposit_event(Event::Updated { key_value });

			Ok(())
		}
	}
}

impl<T: Config> ParameterStore for Pallet<T> {
	type AggregratedKeyValue = T::AggregratedKeyValue;

	fn get<KV, K>(key: K) -> Option<K::Value>
	where
		KV: AggregratedKeyValue,
		K: Key + Into<<KV as AggregratedKeyValue>::AggregratedKey>,
		<KV as AggregratedKeyValue>::AggregratedKey:
			Into<<<Self as ParameterStore>::AggregratedKeyValue as AggregratedKeyValue>::AggregratedKey>,
		<<Self as ParameterStore>::AggregratedKeyValue as AggregratedKeyValue>::AggregratedValue:
			TryInto<<KV as AggregratedKeyValue>::AggregratedValue>,
		<KV as AggregratedKeyValue>::AggregratedValue: TryInto<K::WrappedValue>,
	{
		let key: <KV as AggregratedKeyValue>::AggregratedKey = key.into();
		let val = Parameters::<T>::get(key.into());
		val.and_then(|v| {
			let val: <KV as AggregratedKeyValue>::AggregratedValue = v.try_into().ok()?;
			let val: K::WrappedValue = val.try_into().ok()?;
			let val = val.into();
			Some(val)
		})
	}
}
