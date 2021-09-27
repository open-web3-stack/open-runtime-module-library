#![cfg_attr(not(feature = "std"), no_std)]

pub mod benches;
pub mod mock;
mod tests;
mod weights;

mod pallet_test {
	use crate::weights::ModuleWeights;
	use frame_support::{pallet_prelude::Get, transactional};

	frame_support::decl_storage! {
		trait Store for Module<T: Config<I>, I: Instance = DefaultInstance> as Test where
			<T as OtherConfig>::OtherEvent: Into<<T as Config<I>>::Event>
		{
			pub Foo get(fn foo): Option<u32>;
			pub Value get(fn value): Option<u32>;
			pub Bar get(fn bar): map hasher(twox_64_concat) u32 => u32;
		}
	}

	frame_support::decl_module! {
		pub struct Module<T: Config<I>, I: Instance = DefaultInstance> for enum Call where
			origin: T::Origin, <T as OtherConfig>::OtherEvent: Into<<T as Config<I>>::Event>
		{
			#[weight = 0]
			#[orml_weight_meter::start(ModuleWeights::<T>::set_value())]
			pub fn set_value(origin, n: u32) -> frame_support::dispatch::DispatchResultWithPostInfo {
				let _sender = frame_system::ensure_signed(origin)?;
				Value::<I>::get();
				Value::<I>::put(n);
				Value::<I>::put(n + 1);
				let _ = Self::set_foo();
				Ok(Some(orml_weight_meter::used_weight()).into())
			}

			#[weight = 0]
			fn dummy(origin, _n: u32) -> frame_support::dispatch::DispatchResult {
				let _sender = frame_system::ensure_none(origin)?;
				Foo::<I>::put(1);
				Ok(())
			}
		}
	}

	impl<T: Config<I>, I: Instance> Module<T, I>
	where
		<T as OtherConfig>::OtherEvent: Into<<T as Config<I>>::Event>,
	{
		#[transactional]
		#[orml_weight_meter::weight(ModuleWeights::<T>::set_foo())]
		pub(crate) fn set_foo() -> frame_support::dispatch::DispatchResult {
			Value::<I>::put(2);

			Foo::<I>::put(1);
			Foo::<I>::get();

			Bar::<I>::mutate(1, |v| {
				*v = 1;
			});
			Bar::<I>::insert(1, 1);

			Bar::<I>::insert(2, 2);
			Bar::<I>::get(1);
			Ok(())
		}
	}

	pub trait OtherConfig {
		type OtherEvent;
	}

	pub trait Config<I: Instance = DefaultInstance>: frame_system::Config + OtherConfig
	where
		Self::OtherEvent: Into<<Self as Config<I>>::Event>,
	{
		type Event;
		type LowerBound: Get<u32>;
		type UpperBound: Get<u32>;
	}
}
