//! # Oracle
//! A module to allow oracle operators to feed external data.
//!
//! - [`Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! This module exposes capabilities for oracle operators to feed external
//! offchain data. The raw values can be combined to provide an aggregated
//! value.
//!
//! The data is valid only if feeded by an authorized operator. This module
//! implements `frame_support::traits::InitializeMembers` and `frame_support::
//! traits::ChangeMembers`, to provide a way to manage operators membership.
//! Typically it could be leveraged to `pallet_membership` in FRAME.

#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following two lints since they originate from an external macro (namely decl_storage)
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::unused_unit)]

use codec::{Decode, Encode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{ChangeMembers, Get, InitializeMembers, Time},
	weights::{Pays, Weight},
	Parameter,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
pub use orml_traits::{CombineData, DataFeeder, DataProvider, DataProviderExtended, OnNewData};
use orml_utilities::OrderedSet;
use sp_runtime::{traits::Member, DispatchResult, RuntimeDebug};
use sp_std::{prelude::*, vec};

pub use crate::default_combine_data::DefaultCombineData;

mod default_combine_data;
mod default_weight;
mod mock;
mod tests;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	pub trait WeightInfo {
		fn feed_values(c: u32) -> Weight;
		fn on_finalize() -> Weight;
	}

	pub(crate) type MomentOf<T, I = ()> = <<T as Config<I>>::Time as Time>::Moment;
	pub(crate) type TimestampedValueOf<T, I = ()> = TimestampedValue<<T as Config<I>>::OracleValue, MomentOf<T, I>>;

	#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub struct TimestampedValue<Value, Moment> {
		pub value: Value,
		pub timestamp: Moment,
	}

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// Hook on new data received
		type OnNewData: OnNewData<Self::AccountId, Self::OracleKey, Self::OracleValue>;

		/// Provide the implementation to combine raw values to produce
		/// aggregated value
		type CombineData: CombineData<Self::OracleKey, TimestampedValueOf<Self, I>>;

		/// Time provider
		type Time: Time;

		/// The data key type
		type OracleKey: Parameter + Member;

		/// The data value type
		type OracleValue: Parameter + Member + Ord;

		/// The root operator account id, record all sudo feeds on this account.
		type RootOperatorAccountId: Get<Self::AccountId>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Sender does not have permission
		NoPermission,
		/// Feeder has already feeded at this block
		AlreadyFeeded,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// New feed data is submitted. [sender, values]
		NewFeedData(T::AccountId, Vec<(T::OracleKey, T::OracleValue)>),
	}

	/// Raw values for each oracle operators
	#[pallet::storage]
	#[pallet::getter(fn raw_values)]
	pub type RawValues<T: Config<I>, I: 'static = ()> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, T::OracleKey, TimestampedValueOf<T, I>>;

	/// True if Self::values(key) is up to date, otherwise the value is stale
	#[pallet::storage]
	#[pallet::getter(fn is_updated)]
	pub type IsUpdated<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Twox64Concat, <T as Config<I>>::OracleKey, bool, ValueQuery>;

	/// Combined value, may not be up to date
	#[pallet::storage]
	#[pallet::getter(fn values)]
	pub type Values<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Twox64Concat, <T as Config<I>>::OracleKey, TimestampedValueOf<T, I>>;

	/// If an oracle operator has feed a value in this block
	#[pallet::storage]
	pub(crate) type HasDispatched<T: Config<I>, I: 'static = ()> =
		StorageValue<_, OrderedSet<T::AccountId>, ValueQuery>;

	// TODO: this shouldn't be required https://github.com/paritytech/substrate/issues/6041
	/// The current members of the collective. This is stored sorted (just by
	/// value).
	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub type Members<T: Config<I>, I: 'static = ()> = StorageValue<_, OrderedSet<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonces)]
	pub type Nonces<T: Config<I>, I: 'static = ()> = StorageMap<_, Twox64Concat, T::AccountId, u32>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub members: OrderedSet<T::AccountId>,
		pub phantom: sp_std::marker::PhantomData<I>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
		fn default() -> Self {
			GenesisConfig {
				members: Default::default(),
				phantom: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			<Members<T, I>>::put(self.members.clone());
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<T::BlockNumber> for Pallet<T, I> {
		/// `on_initialize` to return the weight used in `on_finalize`.
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			T::WeightInfo::on_finalize()
		}

		fn on_finalize(_n: T::BlockNumber) {
			// cleanup for next block
			<HasDispatched<T, I>>::kill();
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Feed the external value.
		///
		/// Require authorized operator.
		#[pallet::weight(T::WeightInfo::feed_values(values.len() as u32))]
		pub fn feed_values(
			origin: OriginFor<T>,
			values: Vec<(T::OracleKey, T::OracleValue)>,
		) -> DispatchResultWithPostInfo {
			let feeder = ensure_signed(origin.clone())
				.or_else(|_| ensure_root(origin).map(|_| T::RootOperatorAccountId::get()))?;
			Self::do_feed_values(feeder, values)?;
			Ok(Pays::No.into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn read_raw_values(key: &T::OracleKey) -> Vec<TimestampedValueOf<T, I>> {
		Self::members()
			.0
			.iter()
			.chain(vec![T::RootOperatorAccountId::get()].iter())
			.filter_map(|x| Self::raw_values(x, key))
			.collect()
	}

	/// Returns fresh combined value if has update, or latest combined
	/// value.
	///
	/// Note this will update values storage if has update.
	pub fn get(key: &T::OracleKey) -> Option<TimestampedValueOf<T, I>> {
		if Self::is_updated(key) {
			<Values<T, I>>::get(key)
		} else {
			let timestamped = Self::combined(key)?;
			<Values<T, I>>::insert(key, timestamped.clone());
			IsUpdated::<T, I>::insert(key, true);
			Some(timestamped)
		}
	}

	/// Returns fresh combined value if has update, or latest combined
	/// value.
	///
	/// This is a no-op function which would not change storage.
	pub fn get_no_op(key: &T::OracleKey) -> Option<TimestampedValueOf<T, I>> {
		if Self::is_updated(key) {
			Self::values(key)
		} else {
			Self::combined(key)
		}
	}

	#[allow(clippy::complexity)]
	pub fn get_all_values() -> Vec<(T::OracleKey, Option<TimestampedValueOf<T, I>>)> {
		<Values<T, I>>::iter()
			.map(|(key, _)| key)
			.map(|key| {
				let v = Self::get_no_op(&key);
				(key, v)
			})
			.collect()
	}

	fn combined(key: &T::OracleKey) -> Option<TimestampedValueOf<T, I>> {
		let values = Self::read_raw_values(key);
		T::CombineData::combine_data(key, values, Self::values(key))
	}

	fn do_feed_values(who: T::AccountId, values: Vec<(T::OracleKey, T::OracleValue)>) -> DispatchResult {
		// ensure feeder is authorized
		ensure!(
			Self::members().contains(&who) || who == T::RootOperatorAccountId::get(),
			Error::<T, I>::NoPermission
		);

		// ensure account hasn't dispatched an updated yet
		ensure!(
			HasDispatched::<T, I>::mutate(|set| set.insert(who.clone())),
			Error::<T, I>::AlreadyFeeded
		);

		let now = T::Time::now();
		for (key, value) in &values {
			let timestamped = TimestampedValue {
				value: value.clone(),
				timestamp: now,
			};
			RawValues::<T, I>::insert(&who, &key, timestamped);
			IsUpdated::<T, I>::remove(&key);

			T::OnNewData::on_new_data(&who, &key, &value);
		}
		Self::deposit_event(Event::NewFeedData(who, values));
		Ok(())
	}
}

impl<T: Config<I>, I: 'static> InitializeMembers<T::AccountId> for Pallet<T, I> {
	fn initialize_members(members: &[T::AccountId]) {
		if !members.is_empty() {
			assert!(Members::<T, I>::get().0.is_empty(), "Members are already initialized!");
			Members::<T, I>::put(OrderedSet::from_sorted_set(members.into()));
		}
	}
}

impl<T: Config<I>, I: 'static> ChangeMembers<T::AccountId> for Pallet<T, I> {
	fn change_members_sorted(_incoming: &[T::AccountId], outgoing: &[T::AccountId], new: &[T::AccountId]) {
		// remove session keys and its values
		for removed in outgoing {
			RawValues::<T, I>::remove_prefix(removed);
		}

		Members::<T, I>::put(OrderedSet::from_sorted_set(new.into()));

		// not bothering to track which key needs recompute, just update all
		IsUpdated::<T, I>::remove_all();
	}

	fn set_prime(_prime: Option<T::AccountId>) {
		// nothing
	}
}

impl<T: Config<I>, I: 'static> DataProvider<T::OracleKey, T::OracleValue> for Pallet<T, I> {
	fn get(key: &T::OracleKey) -> Option<T::OracleValue> {
		Self::get(key).map(|timestamped_value| timestamped_value.value)
	}
}
impl<T: Config<I>, I: 'static> DataProviderExtended<T::OracleKey, TimestampedValueOf<T, I>> for Pallet<T, I> {
	fn get_no_op(key: &T::OracleKey) -> Option<TimestampedValueOf<T, I>> {
		Self::get_no_op(key)
	}
	#[allow(clippy::complexity)]
	fn get_all_values() -> Vec<(T::OracleKey, Option<TimestampedValueOf<T, I>>)> {
		Self::get_all_values()
	}
}

impl<T: Config<I>, I: 'static> DataFeeder<T::OracleKey, T::OracleValue, T::AccountId> for Pallet<T, I> {
	fn feed_value(who: T::AccountId, key: T::OracleKey, value: T::OracleValue) -> DispatchResult {
		Self::do_feed_values(who, vec![(key, value)])?;
		Ok(())
	}
}
