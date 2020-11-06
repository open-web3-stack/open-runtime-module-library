//! # Oracle
//! A module to allow oracle operators to feed external data.
//!
//! - [`Trait`](./trait.Trait.html)
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

mod default_combine_data;
mod default_weight;
mod mock;
mod tests;

pub trait WeightInfo {
	fn feed_values(c: u32) -> Weight;
	fn on_finalize() -> Weight;
}

use codec::{Decode, Encode};
pub use default_combine_data::DefaultCombineData;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::DispatchResultWithPostInfo,
	ensure,
	traits::{ChangeMembers, Get, InitializeMembers, Time},
	weights::{DispatchClass, Pays, Weight},
	IterableStorageMap, Parameter,
};
use frame_system::{ensure_root, ensure_signed};
pub use orml_traits::{CombineData, DataFeeder, DataProvider, DataProviderExtended, OnNewData};
use orml_utilities::OrderedSet;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Member, DispatchResult, RuntimeDebug};
use sp_std::{prelude::*, vec};

type MomentOf<T, I = DefaultInstance> = <<T as Trait<I>>::Time as Time>::Moment;
pub type TimestampedValueOf<T, I = DefaultInstance> = TimestampedValue<<T as Trait<I>>::OracleValue, MomentOf<T, I>>;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Clone, Copy, Ord, PartialOrd)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

	/// Hook on new data received
	type OnNewData: OnNewData<Self::AccountId, Self::OracleKey, Self::OracleValue>;

	/// Provide the implementation to combine raw values to produce aggregated
	/// value
	type CombineData: CombineData<Self::OracleKey, TimestampedValueOf<Self, I>>;

	/// Time provider
	type Time: Time;

	/// The data key type
	type OracleKey: Parameter + Member;

	/// The data value type
	type OracleValue: Parameter + Member + Ord;

	/// The root operator account id, recorad all sudo feeds on this account.
	type RootOperatorAccountId: Get<Self::AccountId>;

	/// Weight information for extrinsics in this module.
	type WeightInfo: WeightInfo;
}

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		/// Sender does not have permission
		NoPermission,
		/// Feeder has already feeded at this block
		AlreadyFeeded,
	}
}

decl_event!(
	pub enum Event<T, I=DefaultInstance> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait<I>>::OracleKey,
		<T as Trait<I>>::OracleValue,
	{
		/// New feed data is submitted. [sender, values]
		NewFeedData(AccountId, Vec<(OracleKey, OracleValue)>),
	}
);

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as Oracle {

		/// Raw values for each oracle operators
		pub RawValues get(fn raw_values): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) T::OracleKey => Option<TimestampedValueOf<T, I>>;

		/// True if Self::values(key) is up to date, otherwise the value is stale
		pub IsUpdated get(fn is_updated): map hasher(twox_64_concat) <T as Trait<I>>::OracleKey => bool;

		/// Combined value, may not be up to date
		pub Values get(fn values): map hasher(twox_64_concat) <T as Trait<I>>::OracleKey => Option<TimestampedValueOf<T, I>>;

		/// If an oracle operator has feed a value in this block
		HasDispatched: OrderedSet<T::AccountId>;

		// TODO: this shouldn't be required https://github.com/paritytech/substrate/issues/6041
		/// The current members of the collective. This is stored sorted (just by value).
		pub Members get(fn members) config(): OrderedSet<T::AccountId>;

		pub Nonces get(fn nonces): map hasher(twox_64_concat) T::AccountId => u32;
	}

	add_extra_genesis {
		config(phantom): sp_std::marker::PhantomData<I>;
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
		type Error = Error<T, I>;

		fn deposit_event() = default;

		/// Feed the external value.
		///
		/// Require authorized operator.
		#[weight = (T::WeightInfo::feed_values(values.len() as u32), DispatchClass::Operational)]
		pub fn feed_values(
			origin,
			values: Vec<(T::OracleKey, T::OracleValue)>,
		) -> DispatchResultWithPostInfo {
			let feeder = ensure_signed(origin.clone()).or_else(|_| ensure_root(origin).map(|_| T::RootOperatorAccountId::get()))?;
			Self::do_feed_values(feeder, values)?;
			Ok(Pays::No.into())
		}

		/// `on_initialize` to return the weight used in `on_finalize`.
		fn on_initialize() -> Weight {
			T::WeightInfo::on_finalize()
		}

		fn on_finalize(_n: T::BlockNumber) {
			// cleanup for next block
			<HasDispatched<T, I>>::kill();
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	pub fn read_raw_values(key: &T::OracleKey) -> Vec<TimestampedValueOf<T, I>> {
		Self::members()
			.0
			.iter()
			.chain(vec![T::RootOperatorAccountId::get()].iter())
			.filter_map(|x| Self::raw_values(x, key))
			.collect()
	}

	/// Returns fresh combined value if has update, or latest combined value.
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

	/// Returns fresh combined value if has update, or latest combined value.
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
		Self::deposit_event(RawEvent::NewFeedData(who, values));
		Ok(())
	}
}

impl<T: Trait<I>, I: Instance> InitializeMembers<T::AccountId> for Module<T, I> {
	fn initialize_members(members: &[T::AccountId]) {
		if !members.is_empty() {
			assert!(Members::<T, I>::get().0.is_empty(), "Members are already initialized!");
			Members::<T, I>::put(OrderedSet::from_sorted_set(members.into()));
		}
	}
}

impl<T: Trait<I>, I: Instance> ChangeMembers<T::AccountId> for Module<T, I> {
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

impl<T: Trait<I>, I: Instance> DataProvider<T::OracleKey, T::OracleValue> for Module<T, I> {
	fn get(key: &T::OracleKey) -> Option<T::OracleValue> {
		Self::get(key).map(|timestamped_value| timestamped_value.value)
	}
}
impl<T: Trait<I>, I: Instance> DataProviderExtended<T::OracleKey, TimestampedValueOf<T, I>> for Module<T, I> {
	fn get_no_op(key: &T::OracleKey) -> Option<TimestampedValueOf<T, I>> {
		Self::get_no_op(key)
	}
	#[allow(clippy::complexity)]
	fn get_all_values() -> Vec<(T::OracleKey, Option<TimestampedValueOf<T, I>>)> {
		Self::get_all_values()
	}
}

impl<T: Trait<I>, I: Instance> DataFeeder<T::OracleKey, T::OracleValue, T::AccountId> for Module<T, I> {
	fn feed_value(who: T::AccountId, key: T::OracleKey, value: T::OracleValue) -> DispatchResult {
		Self::do_feed_values(who, vec![(key, value)])?;
		Ok(())
	}
}
