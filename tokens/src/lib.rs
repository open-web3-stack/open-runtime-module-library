#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter};
use rstd::convert::{TryFrom, TryInto};
use sp_runtime::{
	traits::{AtLeast32Bit, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, StaticLookup, Zero},
	DispatchResult,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};

#[cfg(feature = "std")]
use rstd::collections::btree_map::BTreeMap;

use orml_traits::{
	arithmetic::{self, Signed},
	MultiCurrency, MultiCurrencyExtended, OnDustRemoval,
};

mod mock;
mod tests;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Balance: Parameter + Member + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;
	type Amount: Signed
		+ TryInto<Self::Balance>
		+ TryFrom<Self::Balance>
		+ Parameter
		+ Member
		+ arithmetic::SimpleArithmetic
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize;
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;
	type ExistentialDeposit: Get<Self::Balance>;
	type DustRemoval: OnDustRemoval<Self::Balance>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Tokens {
		/// The total issuance of a token type.
		pub TotalIssuance get(fn total_issuance) build(|config: &GenesisConfig<T>| {
			config
				.endowed_accounts
				.iter()
				.map(|(_, currency_id, initial_balance)| (currency_id, initial_balance))
				.fold(BTreeMap::<T::CurrencyId, T::Balance>::new(), |mut acc, (currency_id, initial_balance)| {
					if let Some(issuance) = acc.get_mut(currency_id) {
						*issuance = issuance.checked_add(initial_balance).expect("total issuance cannot overflow when building genesis");
					} else {
						acc.insert(*currency_id, *initial_balance);
					}
					acc
				})
				.into_iter()
				.collect::<Vec<_>>()
		}): map hasher(twox_64_concat) T::CurrencyId => T::Balance;

		/// The balance of a token type under an account.
		pub Balance get(fn balance): double_map hasher(twox_64_concat) T::CurrencyId, hasher(blake2_128_concat) T::AccountId => T::Balance;
	}
	add_extra_genesis {
		config(endowed_accounts): Vec<(T::AccountId, T::CurrencyId, T::Balance)>;

		build(|config: &GenesisConfig<T>| {
			config.endowed_accounts.iter().for_each(|(account_id, currency_id, initial_balance)| {
				<Balance<T>>::insert(currency_id, account_id, initial_balance);
			})
		})
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::CurrencyId,
		<T as Trait>::Balance
	{
		/// Token transfer success (currency_id, from, to, amount)
		Transferred(CurrencyId, AccountId, AccountId, Balance),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Transfer some balance to another account.
		pub fn transfer(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			#[compact] amount: T::Balance,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			<Self as MultiCurrency<_>>::transfer(currency_id, &from, &to, amount)?;

			Self::deposit_event(RawEvent::Transferred(currency_id, from, to, amount));
		}

		/// Transfer all remaining balance to the given account.
		pub fn transfer_all(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			let balance = Self::balance(currency_id, &from);
			<Self as MultiCurrency<T::AccountId>>::transfer(currency_id, &from, &to, balance)?;

			Self::deposit_event(RawEvent::Transferred(currency_id, from, to, balance));
		}
	}
}

decl_error! {
	/// Error for token module.
	pub enum Error for Module<T: Trait> {
		BalanceTooLow,
		TotalIssuanceOverflow,
		AmountIntoBalanceFailed,
		ExistentialDeposit,
	}
}

impl<T: Trait> Module<T> {
	/// Set balance of `who` to a new value, meanwhile enforce existential rule.
	///
	/// Note this will not maintain total issuance, and the caller is expected to do it.
	fn set_balance(currency_id: T::CurrencyId, who: &T::AccountId, balance: T::Balance) {
		if balance < T::ExistentialDeposit::get() {
			<Balance<T>>::remove(currency_id, who);
			T::DustRemoval::on_dust_removal(balance);
			<TotalIssuance<T>>::mutate(currency_id, |v| *v -= balance);
		} else {
			<Balance<T>>::insert(currency_id, who, balance);
		}
	}
}

impl<T: Trait> MultiCurrency<T::AccountId> for Module<T> {
	type CurrencyId = T::CurrencyId;
	type Balance = T::Balance;

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		<TotalIssuance<T>>::get(currency_id)
	}

	fn balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		<Balance<T>>::get(currency_id, who)
	}

	fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if Self::balance(currency_id, who).checked_sub(&amount).is_some() {
			Ok(())
		} else {
			Err(Error::<T>::BalanceTooLow.into())
		}
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() || from == to {
			return Ok(());
		}

		let from_balance = Self::balance(currency_id, from);
		ensure!(from_balance >= amount, Error::<T>::BalanceTooLow);

		let to_balance = Self::balance(currency_id, to);
		if to_balance + amount < T::ExistentialDeposit::get() {
			return Err(Error::<T>::ExistentialDeposit.into());
		}

		if from != to {
			Self::set_balance(currency_id, from, from_balance - amount);
			Self::set_balance(currency_id, to, to_balance + amount);
		}

		Ok(())
	}

	fn deposit(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}

		ensure!(
			Self::total_issuance(currency_id).checked_add(&amount).is_some(),
			Error::<T>::TotalIssuanceOverflow,
		);

		let balance = Self::balance(currency_id, who);
		// Nothing happens if deposition doesn't meet existential deposit rule,
		// consistent behavior with pallet-balances.
		if balance.is_zero() && amount < T::ExistentialDeposit::get() {
			return Ok(());
		}

		<TotalIssuance<T>>::mutate(currency_id, |v| *v += amount);

		Self::set_balance(currency_id, who, balance + amount);

		Ok(())
	}

	fn withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}

		let balance = Self::balance(currency_id, who);
		ensure!(balance >= amount, Error::<T>::BalanceTooLow);

		<TotalIssuance<T>>::mutate(currency_id, |v| *v -= amount);
		Self::set_balance(currency_id, who, balance - amount);

		Ok(())
	}

	fn slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		if amount.is_zero() {
			return amount;
		}

		let balance = Self::balance(currency_id, who);
		let slashed_amount = balance.min(amount);

		if slashed_amount.is_zero() {
			return amount;
		}

		<TotalIssuance<T>>::mutate(currency_id, |v| *v -= slashed_amount);
		Self::set_balance(currency_id, who, balance - slashed_amount);

		amount - slashed_amount
	}
}

impl<T: Trait> MultiCurrencyExtended<T::AccountId> for Module<T> {
	type Amount = T::Amount;

	fn update_balance(currency_id: Self::CurrencyId, who: &T::AccountId, by_amount: Self::Amount) -> DispatchResult {
		if by_amount.is_zero() {
			return Ok(());
		}

		let by_balance =
			TryInto::<Self::Balance>::try_into(by_amount.abs()).map_err(|_| Error::<T>::AmountIntoBalanceFailed)?;
		if by_amount.is_positive() {
			Self::deposit(currency_id, who, by_balance)
		} else {
			Self::withdraw(currency_id, who, by_balance)
		}
	}
}
