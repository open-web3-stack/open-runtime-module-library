#![cfg_attr(not(feature = "std"), no_std)]

pub mod benches;
pub mod mock;
mod tests;
mod weights;

#[frame_support::pallet]
pub mod pallet {
	use crate::weights::ModuleWeights;
	use frame_support::{
		dispatch::{DispatchResult, DispatchResultWithPostInfo},
		pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn foo)]
	pub type Foo<T> = StorageValue<_, u32, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn value)]
	pub type Value<T> = StorageValue<_, u32, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bar)]
	pub type Bar<T> = StorageMap<_, Twox64Concat, u32, u32>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[orml_weight_meter::start(ModuleWeights::<T>::set_value())]
		pub fn set_value(origin: OriginFor<T>, n: u32) -> DispatchResultWithPostInfo {
			let _sender = frame_system::ensure_signed(origin)?;
			Value::<T>::get();
			Value::<T>::put(n);
			Value::<T>::put(n + 1);
			let _ = Self::set_foo();
			Ok(Some(orml_weight_meter::used_weight()).into())
		}

		#[pallet::weight(0)]
		pub fn dummy(origin: OriginFor<T>, _n: u32) -> DispatchResult {
			let _sender = frame_system::ensure_none(origin)?;
			Foo::<T>::put(1);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		#[orml_weight_meter::weight(ModuleWeights::<T>::set_foo())]
		pub(crate) fn set_foo() -> frame_support::dispatch::DispatchResult {
			Value::<T>::put(2);

			Foo::<T>::put(1);
			Foo::<T>::get();

			Bar::<T>::mutate(1, |v| {
				*v = Some(1);
			});
			Bar::<T>::insert(1, 1);

			Bar::<T>::insert(2, 2);
			Bar::<T>::get(1);
			Ok(())
		}

		#[orml_weight_meter::weight(0)]
		pub(crate) fn remove_all_bar() {
			_ = Bar::<T>::clear(10, None);
		}

		#[orml_weight_meter::weight(0)]
		pub(crate) fn remove_all_bar_with_limit() {
			_ = Bar::<T>::clear(10, None);
		}
	}
}
