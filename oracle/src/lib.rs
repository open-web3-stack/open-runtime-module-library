#![cfg_attr(not(feature = "std"), no_std)]

mod default_combine_data;
mod mock;
mod operator_provider;
mod tests;
mod timestamped_value;

use codec::{Decode, Encode};
pub use default_combine_data::DefaultCombineData;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::Dispatchable,
	ensure,
	traits::Time,
	weights::{DispatchClass, DispatchInfo, FunctionOf, TransactionPriority},
	IsSubType, Parameter,
};
pub use operator_provider::OperatorProvider;
use sp_runtime::{
	traits::{Member, SignedExtension},
	DispatchResult,
};
use sp_std::{prelude::*, vec};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};
pub use orml_traits::{CombineData, DataProvider, OnNewData, OnRedundantCall};
use sp_runtime::transaction_validity::{
	InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
};
pub use timestamped_value::TimestampedValue;

type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;
pub type TimestampedValueOf<T> = TimestampedValue<<T as Trait>::OracleValue, MomentOf<T>>;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Call: Parameter + Dispatchable<Origin = <Self as frame_system::Trait>::Origin> + IsSubType<Module<Self>, Self>;
	type OnNewData: OnNewData<Self::AccountId, Self::OracleKey, Self::OracleValue>;
	type OnRedundantCall: OnRedundantCall<Self::AccountId>;
	type OperatorProvider: OperatorProvider<Self::AccountId>;
	type CombineData: CombineData<Self::OracleKey, TimestampedValueOf<Self>>;
	type Time: Time;
	type OracleKey: Parameter + Member;
	type OracleValue: Parameter + Member + Ord;
}

decl_storage! {
	trait Store for Module<T: Trait> as Oracle {
		pub RawValues get(raw_values): double_map hasher(blake2_256) T::OracleKey, hasher(blake2_256) T::AccountId => Option<TimestampedValueOf<T>>;
		pub HasUpdate get(has_update): map hasher(blake2_256) T::OracleKey => bool;
		pub Values get(values): map hasher(blake2_256) T::OracleKey => Option<TimestampedValueOf<T>>;
		HasDispatched: Vec<T::AccountId>;
	}
}

decl_error! {
	// Oracle module errors
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
	#[derive(Encode, Decode)]
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		#[weight = FunctionOf(|_: (&T::OracleKey, &T::OracleValue)| 0, DispatchClass::Operational, false)]
		pub fn feed_value(origin, key: T::OracleKey, value: T::OracleValue) {
			let who = ensure_signed(origin)?;
			Self::_feed_values(who, vec![(key, value)])?;
		}

		#[weight = FunctionOf(|_: (&Vec<(T::OracleKey, T::OracleValue)>,)| 0, DispatchClass::Operational, false)]
		pub fn feed_values(origin, values: Vec<(T::OracleKey, T::OracleValue)>) {
			let who = ensure_signed(origin)?;
			Self::_feed_values(who, values)?;
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
		T::OperatorProvider::operators()
			.iter()
			.filter_map(|x| <RawValues<T>>::get(key, x))
			.collect()
	}

	/// Returns fresh combined value if has update, or latest combined value.
	///
	/// Note this will update values storage if has update.
	pub fn get(key: &T::OracleKey) -> Option<TimestampedValueOf<T>> {
		if <HasUpdate<T>>::take(key) {
			let timestamped = Self::combined(key)?;
			<Values<T>>::insert(key, timestamped.clone());
			return Some(timestamped);
		}
		<Values<T>>::get(key)
	}

	/// Returns fresh combined value if has update, or latest combined value.
	///
	/// This is a no-op function which would not change storage.
	pub fn get_no_op(key: &T::OracleKey) -> Option<TimestampedValueOf<T>> {
		if Self::has_update(key) {
			Self::combined(key)
		} else {
			Self::values(key)
		}
	}

	fn combined(key: &T::OracleKey) -> Option<TimestampedValueOf<T>> {
		let values = Self::read_raw_values(key);
		T::CombineData::combine_data(key, values, Self::values(key))
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct CheckOperator<T: Trait + Send + Sync>(sp_std::marker::PhantomData<T>);

impl<T: Trait + Send + Sync> sp_std::fmt::Debug for CheckOperator<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "CheckOperator")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl<T: Trait + Send + Sync> SignedExtension for CheckOperator<T> {
	const IDENTIFIER: &'static str = "CheckOperator";
	type AccountId = T::AccountId;
	type Call = <T as Trait>::Call;
	type AdditionalSigned = ();
	type Pre = ();
	type DispatchInfo = DispatchInfo;

	fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
		Ok(())
	}

	fn validate(
		&self,
		who: &T::AccountId,
		call: &Self::Call,
		_info: Self::DispatchInfo,
		_len: usize,
	) -> TransactionValidity {
		let call = match call.is_sub_type() {
			Some(call) => call,
			None => return Ok(ValidTransaction::default()),
		};

		if let Call::<T>::feed_value(..) | Call::<T>::feed_values(..) = call {
			ensure!(
				T::OperatorProvider::can_feed_data(who),
				TransactionValidityError::Invalid(InvalidTransaction::Custom(ValidityError::NoPermission as u8))
			);

			return Ok(ValidTransaction {
				priority: TransactionPriority::max_value(),
				..Default::default()
			});
		}
		return Ok(ValidTransaction::default());
	}
}

impl<T: Trait> DataProvider<T::OracleKey, T::OracleValue> for Module<T> {
	fn get(key: &T::OracleKey) -> Option<T::OracleValue> {
		Self::get(key).map(|timestamped_value| timestamped_value.value)
	}
}

impl<T: Trait> Module<T> {
	fn _feed_values(who: T::AccountId, values: Vec<(T::OracleKey, T::OracleValue)>) -> DispatchResult {
		ensure!(T::OperatorProvider::can_feed_data(&who), Error::<T>::NoPermission);

		// ensure account hasn't dispatched an updated yet
		let mut accounts = <HasDispatched<T>>::get();
		if accounts.contains(&who) {
			T::OnRedundantCall::multiple_calls_per_block(&who);
			return Err(Error::<T>::UpdateAlreadyDispatched.into());
		}
		accounts.push(who.clone());
		<HasDispatched<T>>::put(accounts);

		let now = T::Time::now();

		for (key, value) in &values {
			let timestamped = TimestampedValue {
				value: value.clone(),
				timestamp: now,
			};
			<RawValues<T>>::insert(&key, &who, timestamped);
			<HasUpdate<T>>::insert(&key, true);

			T::OnNewData::on_new_data(&who, &key, &value);
		}

		Self::deposit_event(RawEvent::NewFeedData(who, values));

		Ok(())
	}
}
