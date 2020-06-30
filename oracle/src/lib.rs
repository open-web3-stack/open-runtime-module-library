#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following two lints since they originate from an external macro (namely decl_storage)
#![allow(clippy::string_lit_as_bytes)]

mod default_combine_data;
mod mock;
mod tests;

use codec::{Decode, Encode};
pub use default_combine_data::DefaultCombineData;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{ChangeMembers, Get, InitializeMembers, Time},
	weights::{DispatchClass, Pays},
	IterableStorageMap, Parameter,
};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	traits::Member,
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity, ValidTransaction,
	},
	DispatchResult, RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*, vec};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_none, ensure_root, ensure_signed};
pub use orml_traits::{CombineData, DataProvider, DataProviderExtended, OnNewData, OnRedundantCall};
use orml_utilities::OrderedSet;

use sp_application_crypto::{KeyTypeId, RuntimeAppPublic};
pub const ORACLE: KeyTypeId = KeyTypeId(*b"orac");

mod app_sr25519 {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, super::ORACLE);
}

sp_application_crypto::with_pair! {
	/// An oracle keypair using sr25519 as its crypto.
	pub type AuthorityPair = app_sr25519::Pair;
}

/// An oracle signature using sr25519 as its crypto.
pub type AuthoritySignature = app_sr25519::Signature;

/// An oracle identifier using sr25519 as its crypto.
pub type AuthorityId = app_sr25519::Public;

type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;
pub type TimestampedValueOf<T> = TimestampedValue<<T as Trait>::OracleValue, MomentOf<T>>;

/// Number of blocks before an unconfirmed unsigned transaction expires.
pub const EXTRINSIC_LONGVITY: u32 = 100;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type OnNewData: OnNewData<Self::AccountId, Self::OracleKey, Self::OracleValue>;
	type CombineData: CombineData<Self::OracleKey, TimestampedValueOf<Self>>;
	type Time: Time;
	type OracleKey: Parameter + Member;
	type OracleValue: Parameter + Member + Ord;

	/// A configuration for base priority of unsigned transactions.
	///
	/// This is exposed so that it can be tuned for particular runtime, when
	/// multiple pallets send unsigned transactions.
	type UnsignedPriority: Get<TransactionPriority>;

	/// The identifier type for an authority.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord;
}

decl_storage! {
	trait Store for Module<T: Trait> as Oracle {

		/// Raw values for each oracle operators
		pub RawValues get(fn raw_values): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) T::OracleKey => Option<TimestampedValueOf<T>>;

		/// True if Self::values(key) is up to date, otherwise the value is stale
		pub IsUpdated get(fn is_updated): map hasher(twox_64_concat) T::OracleKey => bool;

		/// Combined value, may not be up to date
		pub Values get(fn values): map hasher(twox_64_concat) T::OracleKey => Option<TimestampedValueOf<T>>;

		/// If an oracle operator has feed a value in this block
		HasDispatched: OrderedSet<T::AccountId>;

		// TODO: this shouldn't be required https://github.com/paritytech/substrate/issues/6041
		/// The current members of the collective. This is stored sorted (just by value).
		pub Members get(fn members) config(): OrderedSet<T::AccountId>;

		/// Session key for oracle operators
		pub SessionKeys get(fn session_keys) config(): map hasher(twox_64_concat) T::AccountId => Option<T::AuthorityId>;

		pub Nonces get(fn nonces): map hasher(twox_64_concat) T::AccountId => u32;
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		NoPermission,
		UpdateAlreadyDispatched,
	}
}

#[repr(u8)]
pub enum ValidityError {
	NoPermission,
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = (0, DispatchClass::Operational, Pays::No)]
		pub fn feed_values(
			origin,
			values: Vec<(T::OracleKey, T::OracleValue)>,
			#[compact] index: u32,
			_block: T::BlockNumber,
			// since signature verification is done in `validate_unsigned`
			// we can skip doing it here again.
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) {
			ensure_none(origin.clone()).or_else(|_| ensure_root(origin))?;
			// validate_unsigned already unsure index is valid
			let who = Self::members().0[index as usize].clone();
			Self::do_feed_values(who, values);
		}

		#[weight = 10_000_000]
		pub fn set_session_key(origin, key: T::AuthorityId) {
			let who = ensure_signed(origin)?;
			ensure!(Self::members().contains(&who), Error::<T>::NoPermission);

			SessionKeys::<T>::insert(who, key);
		}

		fn on_finalize(_n: T::BlockNumber) {
			// cleanup for next block
			<HasDispatched<T>>::kill();
		}
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::OracleKey,
		<T as Trait>::OracleValue,
	{
		/// New feed data is submitted (sender, values)
		NewFeedData(AccountId, Vec<(OracleKey, OracleValue)>),
	}
);

impl<T: Trait> Module<T> {
	pub fn read_raw_values(key: &T::OracleKey) -> Vec<TimestampedValueOf<T>> {
		Self::members()
			.0
			.iter()
			.filter_map(|x| Self::raw_values(x, key))
			.collect()
	}

	/// Returns fresh combined value if has update, or latest combined value.
	///
	/// Note this will update values storage if has update.
	pub fn get(key: &T::OracleKey) -> Option<TimestampedValueOf<T>> {
		if Self::is_updated(key) {
			<Values<T>>::get(key)
		} else {
			let timestamped = Self::combined(key)?;
			<Values<T>>::insert(key, timestamped.clone());
			IsUpdated::<T>::insert(key, true);
			Some(timestamped)
		}
	}

	/// Returns fresh combined value if has update, or latest combined value.
	///
	/// This is a no-op function which would not change storage.
	pub fn get_no_op(key: &T::OracleKey) -> Option<TimestampedValueOf<T>> {
		if Self::is_updated(key) {
			Self::values(key)
		} else {
			Self::combined(key)
		}
	}

	pub fn get_all_values() -> Vec<(T::OracleKey, Option<TimestampedValueOf<T>>)> {
		<Values<T>>::iter()
			.map(|(key, _)| key)
			.map(|key| {
				let v = Self::get_no_op(&key);
				(key, v)
			})
			.collect()
	}

	fn combined(key: &T::OracleKey) -> Option<TimestampedValueOf<T>> {
		let values = Self::read_raw_values(key);
		T::CombineData::combine_data(key, values, Self::values(key))
	}

	fn do_feed_values(who: T::AccountId, values: Vec<(T::OracleKey, T::OracleValue)>) {
		let now = T::Time::now();

		for (key, value) in &values {
			let timestamped = TimestampedValue {
				value: value.clone(),
				timestamp: now,
			};
			RawValues::<T>::insert(&who, &key, timestamped);
			IsUpdated::<T>::remove(&key);

			T::OnNewData::on_new_data(&who, &key, &value);
		}

		Self::deposit_event(RawEvent::NewFeedData(who, values));
	}
}

impl<T: Trait> InitializeMembers<T::AccountId> for Module<T> {
	fn initialize_members(members: &[T::AccountId]) {
		if !members.is_empty() {
			assert!(Members::<T>::get().0.is_empty(), "Members are already initialized!");
			Members::<T>::put(OrderedSet::from_sorted_set(members.into()));
		}
	}
}

impl<T: Trait> ChangeMembers<T::AccountId> for Module<T> {
	fn change_members_sorted(_incoming: &[T::AccountId], outgoing: &[T::AccountId], new: &[T::AccountId]) {
		// remove session keys and its values
		for removed in outgoing {
			SessionKeys::<T>::remove(removed);
			RawValues::<T>::remove_prefix(removed);
			Nonces::<T>::remove(removed);
		}

		Members::<T>::put(OrderedSet::from_sorted_set(new.into()));

		// not bothering to track which key needs recompute, just update all
		IsUpdated::<T>::remove_all();
	}

	fn set_prime(_prime: Option<T::AccountId>) {
		// nothing
	}
}

impl<T: Trait> DataProvider<T::OracleKey, T::OracleValue> for Module<T> {
	fn get(key: &T::OracleKey) -> Option<T::OracleValue> {
		Self::get(key).map(|timestamped_value| timestamped_value.value)
	}
}

impl<T: Trait> DataProviderExtended<T::OracleKey, T::OracleValue, T::AccountId> for Module<T> {
	fn feed_value(who: T::AccountId, key: T::OracleKey, value: T::OracleValue) -> DispatchResult {
		Self::do_feed_values(who, vec![(key, value)]);
		Ok(())
	}
}

impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		if let Call::feed_values(value, index, block, signature) = call {
			let now = <frame_system::Module<T>>::block_number();

			if now > *block + EXTRINSIC_LONGVITY.into() {
				return Err(InvalidTransaction::Stale.into());
			}
			if now < *block {
				return Err(InvalidTransaction::Future.into());
			}

			let members = Module::<T>::members();
			let who = members.0.get(*index as usize);
			if let Some(who) = who {
				let nonce = Module::<T>::nonces(&who);

				let signature_valid = Module::<T>::session_keys(&who)
					.map(|session_key| {
						(nonce, block, value).using_encoded(|payload| session_key.verify(&payload, &signature))
					})
					.unwrap_or(false);

				if !signature_valid {
					return InvalidTransaction::BadProof.into();
				}

				// ensure account hasn't dispatched an updated yet
				let ok = HasDispatched::<T>::mutate(|set| set.insert(who.clone()));
				if !ok {
					// we already received a feed for this operator
					return Err(InvalidTransaction::Stale.into());
				}

				Nonces::<T>::insert(who, nonce + 1);

				// make priority less likely to overflow.
				// this ensures tx sent later overrides old one
				let add_priority = TryInto::<TransactionPriority>::try_into(*block % 1000.into()).unwrap_or(0);

				ValidTransaction::with_tag_prefix("orml-oracle")
					.priority(T::UnsignedPriority::get().saturating_add(add_priority))
					.and_provides((who, nonce))
					.longevity(EXTRINSIC_LONGVITY.into())
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::BadProof.into()
			}
		} else {
			InvalidTransaction::Call.into()
		}
	}
}
