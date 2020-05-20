#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following two lints since they originate from an external macro (namely decl_storage)
#![allow(clippy::string_lit_as_bytes)]

mod default_combine_data;
mod mock;
mod tests;
mod timestamped_value;

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::Dispatchable,
	ensure,
	traits::{ChangeMembers, Get, InitializeMembers, Time},
	weights::{DispatchClass, FunctionOf, Pays},
	IsSubType, IterableStorageMap, Parameter,
};
use sp_runtime::{
	traits::{DispatchInfoOf, Member, SignedExtension},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionValidity, TransactionValidityError, ValidTransaction,
	},
	DispatchResult,
};
use sp_std::{fmt, prelude::*, result, vec};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
pub use default_combine_data::DefaultCombineData;
use frame_system::{self as system, ensure_none, ensure_signed};
pub use orml_traits::{CombineData, DataProvider, DataProviderExtended, OnNewData, OnRedundantCall};
use orml_utilities::OrderedSet;
pub use timestamped_value::TimestampedValue;

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

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Call: Parameter + Dispatchable<Origin = <Self as frame_system::Trait>::Origin> + IsSubType<Module<Self>, Self>;
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

		#[weight = FunctionOf(0, DispatchClass::Operational, Pays::No)]
		pub fn feed_values(
			origin,
			values: Vec<(T::OracleKey, T::OracleValue)>,
			#[compact] index: u32,
			// since signature verification is done in `validate_unsigned`
			// we can skip doing it here again.
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) {
			ensure_none(origin)?;
			let who = Self::members().0[index as usize].clone();
			Self::_feed_values(who, values)?;
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
			.map(|key| (key.clone(), Self::get_no_op(&key)))
			.collect()
	}

	fn combined(key: &T::OracleKey) -> Option<TimestampedValueOf<T>> {
		let values = Self::read_raw_values(key);
		T::CombineData::combine_data(key, values, Self::values(key))
	}

	fn _feed_values(who: T::AccountId, values: Vec<(T::OracleKey, T::OracleValue)>) -> DispatchResult {
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

		Ok(())
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
		Self::_feed_values(who, vec![(key, value)])
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, Default)]
pub struct CheckOperator<T: Trait + Send + Sync>(sp_std::marker::PhantomData<T>);

impl<T: Trait + Send + Sync> CheckOperator<T> {
	pub fn new() -> Self {
		Self(sp_std::marker::PhantomData)
	}
}

impl<T: Trait + Send + Sync> fmt::Debug for CheckOperator<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "CheckOperator")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
		Ok(())
	}
}

impl<T: Trait + Send + Sync> SignedExtension for CheckOperator<T> {
	const IDENTIFIER: &'static str = "CheckOperator";
	type AccountId = T::AccountId;
	type Call = <T as Trait>::Call;
	type AdditionalSigned = ();
	type Pre = ();

	fn additional_signed(&self) -> result::Result<(), TransactionValidityError> {
		Ok(())
	}

	fn validate_unsigned(call: &Self::Call, _info: &DispatchInfoOf<Self::Call>, _len: usize) -> TransactionValidity {
		let call = match call.is_sub_type() {
			Some(call) => call,
			None => return Ok(ValidTransaction::default()),
		};

		if let Call::feed_values(value, index, signature) = call {
			let members = Module::<T>::members();
			let who = members.0.get(*index as usize);
			if let Some(who) = who {
				let nonce = Module::<T>::nonces(&who);

				let signature_valid = Module::<T>::session_keys(&who)
					.map(|session_key| (nonce, value).using_encoded(|payload| session_key.verify(&payload, &signature)))
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

				ValidTransaction::with_tag_prefix("Oracle")
					.priority(T::UnsignedPriority::get())
					.longevity(256)
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
