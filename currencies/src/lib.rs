#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	traits::{Currency as PalletCurrency, ExistenceRequirement, Get, WithdrawReason},
};
use rstd::{convert::TryInto, marker};
use sp_runtime::{
	traits::{CheckedSub, StaticLookup},
	DispatchResult,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_root, ensure_signed};

use orml_traits::{arithmetic::Signed, BasicCurrency, BasicCurrencyExtended, MultiCurrency, MultiCurrencyExtended};

mod mock;
mod tests;

type BalanceOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Balance;
type CurrencyIdOf<T> =
	<<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::CurrencyId;

type AmountOf<T> =
	<<T as Trait>::MultiCurrency as MultiCurrencyExtended<<T as frame_system::Trait>::AccountId>>::Amount;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type MultiCurrency: MultiCurrencyExtended<Self::AccountId>;
	type NativeCurrency: BasicCurrencyExtended<Self::AccountId, Balance = BalanceOf<Self>, Amount = AmountOf<Self>>;
	type GetNativeCurrencyId: Get<CurrencyIdOf<Self>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Currencies {}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		Amount = AmountOf<T>,
		Balance = BalanceOf<T>,
		CurrencyId = CurrencyIdOf<T>
	{
		/// Currency transfer success (currency_id, from, to, amount)
		Transferred(CurrencyId, AccountId, AccountId, Balance),
		/// Update balance success (currency_id, who, amount)
		BalanceUpdated(CurrencyId, AccountId, Amount),
	}
);

decl_error! {
	/// Error for currencies module.
	pub enum Error for Module<T: Trait> {
		AmountIntoBalanceFailed,
		BalanceTooLow,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const NativeCurrencyId: CurrencyIdOf<T> = T::GetNativeCurrencyId::get();

		fn deposit_event() = default;

		/// Transfer some balance to another account.
		pub fn transfer(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			#[compact] amount: BalanceOf<T>,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			<Self as MultiCurrency<T::AccountId>>::transfer(currency_id, &from, &to, amount)?;

			Self::deposit_event(RawEvent::Transferred(currency_id, from, to, amount));
		}

		/// Transfer native currency balance from one account to another.
		pub fn transfer_native_currency(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: BalanceOf<T>,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			T::NativeCurrency::transfer(&from, &to, amount)?;

			Self::deposit_event(RawEvent::Transferred(T::GetNativeCurrencyId::get(), from, to, amount));
		}

		/// Update balance of an account. This is a root call.
		pub fn update_balance(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			amount: AmountOf<T>,
		) {
			ensure_root(origin)?;
			let dest = T::Lookup::lookup(who)?;
			<Self as MultiCurrencyExtended<T::AccountId>>::update_balance(currency_id, &dest, amount)?;

			Self::deposit_event(RawEvent::BalanceUpdated(currency_id, dest, amount));
		}
	}
}

impl<T: Trait> Module<T> {}

impl<T: Trait> MultiCurrency<T::AccountId> for Module<T> {
	type CurrencyId = CurrencyIdOf<T>;
	type Balance = BalanceOf<T>;

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::total_issuance()
		} else {
			T::MultiCurrency::total_issuance(currency_id)
		}
	}

	fn balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::balance(who)
		} else {
			T::MultiCurrency::balance(currency_id, who)
		}
	}

	fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::ensure_can_withdraw(who, amount)
		} else {
			T::MultiCurrency::ensure_can_withdraw(currency_id, who, amount)
		}
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::transfer(from, to, amount)
		} else {
			T::MultiCurrency::transfer(currency_id, from, to, amount)
		}
	}

	fn deposit(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::deposit(who, amount)
		} else {
			T::MultiCurrency::deposit(currency_id, who, amount)
		}
	}

	fn withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::withdraw(who, amount)
		} else {
			T::MultiCurrency::withdraw(currency_id, who, amount)
		}
	}

	fn slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::slash(who, amount)
		} else {
			T::MultiCurrency::slash(currency_id, who, amount)
		}
	}
}

impl<T: Trait> MultiCurrencyExtended<T::AccountId> for Module<T> {
	type Amount = AmountOf<T>;

	fn update_balance(currency_id: Self::CurrencyId, who: &T::AccountId, by_amount: Self::Amount) -> DispatchResult {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::update_balance(who, by_amount)
		} else {
			T::MultiCurrency::update_balance(currency_id, who, by_amount)
		}
	}
}

pub struct Currency<T, GetCurrencyId>(marker::PhantomData<T>, marker::PhantomData<GetCurrencyId>);

impl<T, GetCurrencyId> BasicCurrency<T::AccountId> for Currency<T, GetCurrencyId>
where
	T: Trait,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	type Balance = BalanceOf<T>;

	fn total_issuance() -> Self::Balance {
		<Module<T>>::total_issuance(GetCurrencyId::get())
	}

	fn balance(who: &T::AccountId) -> Self::Balance {
		<Module<T>>::balance(GetCurrencyId::get(), who)
	}

	fn ensure_can_withdraw(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Module<T>>::ensure_can_withdraw(GetCurrencyId::get(), who, amount)
	}

	fn transfer(from: &T::AccountId, to: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Module<T> as MultiCurrency<T::AccountId>>::transfer(GetCurrencyId::get(), from, to, amount)
	}

	fn deposit(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Module<T>>::deposit(GetCurrencyId::get(), who, amount)
	}

	fn withdraw(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Module<T>>::withdraw(GetCurrencyId::get(), who, amount)
	}

	fn slash(who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		<Module<T>>::slash(GetCurrencyId::get(), who, amount)
	}
}

impl<T, GetCurrencyId> BasicCurrencyExtended<T::AccountId> for Currency<T, GetCurrencyId>
where
	T: Trait,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	type Amount = AmountOf<T>;

	fn update_balance(who: &T::AccountId, by_amount: Self::Amount) -> DispatchResult {
		<Module<T> as MultiCurrencyExtended<T::AccountId>>::update_balance(GetCurrencyId::get(), who, by_amount)
	}
}

pub type NativeCurrencyOf<T> = Currency<T, <T as Trait>::GetNativeCurrencyId>;

/// Adapt other currency traits implementation to `BasicCurrency`.
pub struct BasicCurrencyAdapter<T, Currency, BalanceConvert>(marker::PhantomData<(T, Currency, BalanceConvert)>);

type PalletBalanceOf<A, Currency> = <Currency as PalletCurrency<A>>::Balance;

// Adapt `frame_support::traits::Currency`
impl<AccountId, T, Currency, BalanceConvert> BasicCurrency<AccountId>
	for BasicCurrencyAdapter<T, Currency, BalanceConvert>
where
	T: Trait,
	Currency: PalletCurrency<AccountId>,
	BalanceConvert: From<PalletBalanceOf<AccountId, Currency>>
		+ Into<PalletBalanceOf<AccountId, Currency>>
		+ From<BalanceOf<T>>
		+ Into<BalanceOf<T>>,
{
	type Balance = BalanceOf<T>;

	fn total_issuance() -> Self::Balance {
		BalanceConvert::from(Currency::total_issuance()).into()
	}

	fn balance(who: &AccountId) -> Self::Balance {
		BalanceConvert::from(Currency::total_balance(who)).into()
	}

	fn ensure_can_withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		let new_balance_pallet = {
			let new_balance = Self::balance(who)
				.checked_sub(&amount)
				.ok_or(Error::<T>::BalanceTooLow)?;
			BalanceConvert::from(new_balance).into()
		};
		let amount_pallet = BalanceConvert::from(amount).into();
		Currency::ensure_can_withdraw(who, amount_pallet, WithdrawReason::Transfer.into(), new_balance_pallet)
	}

	fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
		let amount_pallet = BalanceConvert::from(amount).into();
		Currency::transfer(from, to, amount_pallet, ExistenceRequirement::AllowDeath)
	}

	fn deposit(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		let _ = Currency::deposit_creating(who, BalanceConvert::from(amount).into());
		Ok(())
	}

	fn withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		Currency::withdraw(
			who,
			BalanceConvert::from(amount).into(),
			WithdrawReason::Transfer.into(),
			ExistenceRequirement::AllowDeath,
		)
		.map(|_| ())
	}

	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
		let (_, gap) = Currency::slash(who, BalanceConvert::from(amount).into());
		BalanceConvert::from(gap).into()
	}
}

// Adapt `frame_support::traits::Currency`
impl<AccountId, T, Currency, BalanceConvert> BasicCurrencyExtended<AccountId>
	for BasicCurrencyAdapter<T, Currency, BalanceConvert>
where
	T: Trait,
	Currency: PalletCurrency<AccountId>,
	BalanceConvert: From<PalletBalanceOf<AccountId, Currency>>
		+ Into<PalletBalanceOf<AccountId, Currency>>
		+ From<BalanceOf<T>>
		+ Into<BalanceOf<T>>,
{
	type Amount = AmountOf<T>;

	fn update_balance(who: &AccountId, by_amount: Self::Amount) -> DispatchResult {
		let by_balance = by_amount
			.abs()
			.try_into()
			.map_err(|_| Error::<T>::AmountIntoBalanceFailed)?;
		if by_amount.is_positive() {
			Self::deposit(who, by_balance)
		} else {
			Self::withdraw(who, by_balance)
		}
	}
}
