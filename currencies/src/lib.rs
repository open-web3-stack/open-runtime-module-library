#![cfg_attr(not(feature = "std"), no_std)]

use paint_support::{
	decl_event, decl_module, decl_storage,
	traits::{Currency as PaintCurrency, ExistenceRequirement, Get, WithdrawReason},
};
use rstd::{marker, result};
use sr_primitives::traits::StaticLookup;
// FIXME: `paint-` prefix should be used for all paint modules, but currently `paint_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use paint_system::{self as system, ensure_signed};

use traits::{BasicCurrency, BasicCurrencyExtended, MultiCurrency, MultiCurrencyExtended};

mod mock;
mod tests;

type BalanceOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as paint_system::Trait>::AccountId>>::Balance;
type CurrencyIdOf<T> =
	<<T as Trait>::MultiCurrency as MultiCurrency<<T as paint_system::Trait>::AccountId>>::CurrencyId;
type ErrorOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as paint_system::Trait>::AccountId>>::Error;

type AmountOf<T> =
	<<T as Trait>::MultiCurrency as MultiCurrencyExtended<<T as paint_system::Trait>::AccountId>>::Amount;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrencyExtended<Self::AccountId>;
	type NativeCurrency: BasicCurrencyExtended<
		Self::AccountId,
		Balance = BalanceOf<Self>,
		Error = ErrorOf<Self>,
		Amount = AmountOf<Self>,
	>;
	type GetNativeCurrencyId: Get<CurrencyIdOf<Self>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Currencies {

	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		Balance = BalanceOf<T>,
		CurrencyId = CurrencyIdOf<T>
	{
		/// Currency transfer success (currency_id, from, to, amount)
		Transferred(CurrencyId, AccountId, AccountId, Balance),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		/// Transfer some balance to another account.
		pub fn transfer(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			#[compact] currency_id: CurrencyIdOf<T>,
			#[compact] amount: BalanceOf<T>,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			if currency_id == T::GetNativeCurrencyId::get() {
				T::NativeCurrency::transfer(&from, &to, amount).map_err(Into::into)?;
			} else {
				T::MultiCurrency::transfer(currency_id, &from, &to, amount).map_err(Into::into)?;
			}

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
			let currency_id = T::GetNativeCurrencyId::get();
			<Self as MultiCurrency<_>>::transfer(currency_id, &from, &to, amount).map_err(Into::into)?;

			Self::deposit_event(RawEvent::Transferred(currency_id, from, to, amount));
		}
	}
}

impl<T: Trait> Module<T> {}

impl<T: Trait> MultiCurrency<T::AccountId> for Module<T> {
	type Balance = BalanceOf<T>;
	type CurrencyId = CurrencyIdOf<T>;
	type Error = ErrorOf<T>;

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

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error> {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::transfer(from, to, amount)
		} else {
			T::MultiCurrency::transfer(currency_id, from, to, amount)
		}
	}

	fn deposit(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error> {
		if currency_id == T::GetNativeCurrencyId::get() {
			T::NativeCurrency::deposit(who, amount)
		} else {
			T::MultiCurrency::deposit(currency_id, who, amount)
		}
	}

	fn withdraw(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error> {
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

	fn update_balance(
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		by_amount: Self::Amount,
	) -> result::Result<(), Self::Error> {
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
	type Error = ErrorOf<T>;

	fn total_issuance() -> Self::Balance {
		T::MultiCurrency::total_issuance(GetCurrencyId::get())
	}

	fn balance(who: &T::AccountId) -> Self::Balance {
		T::MultiCurrency::balance(GetCurrencyId::get(), who)
	}

	fn transfer(from: &T::AccountId, to: &T::AccountId, amount: Self::Balance) -> result::Result<(), Self::Error> {
		T::MultiCurrency::transfer(GetCurrencyId::get(), from, to, amount)
	}

	fn deposit(who: &T::AccountId, amount: Self::Balance) -> result::Result<(), Self::Error> {
		T::MultiCurrency::deposit(GetCurrencyId::get(), who, amount)
	}

	fn withdraw(who: &T::AccountId, amount: Self::Balance) -> result::Result<(), Self::Error> {
		T::MultiCurrency::withdraw(GetCurrencyId::get(), who, amount)
	}

	fn slash(who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		T::MultiCurrency::slash(GetCurrencyId::get(), who, amount)
	}
}

impl<T, GetCurrencyId> BasicCurrencyExtended<T::AccountId> for Currency<T, GetCurrencyId>
where
	T: Trait,
	GetCurrencyId: Get<CurrencyIdOf<T>>,
{
	type Amount = AmountOf<T>;

	fn update_balance(who: &T::AccountId, by_amount: Self::Amount) -> result::Result<(), Self::Error> {
		T::MultiCurrency::update_balance(GetCurrencyId::get(), who, by_amount)
	}
}

pub type NativeCurrencyOf<T> = Currency<T, <T as Trait>::GetNativeCurrencyId>;

/// Adapt other currency traits implementation to `BasicCurrency`.
pub struct BasicCurrencyAdapter<T>(marker::PhantomData<T>);

// Adapat `paint_support::traits::Currency`
impl<AccountId, T> BasicCurrency<AccountId> for BasicCurrencyAdapter<T>
where
	T: PaintCurrency<AccountId>,
{
	type Balance = <T as PaintCurrency<AccountId>>::Balance;
	type Error = &'static str;

	fn total_issuance() -> Self::Balance {
		T::total_issuance()
	}

	fn balance(who: &AccountId) -> Self::Balance {
		T::total_balance(who)
	}

	fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> result::Result<(), Self::Error> {
		T::transfer(from, to, amount, ExistenceRequirement::AllowDeath)
	}

	fn deposit(who: &AccountId, amount: Self::Balance) -> result::Result<(), Self::Error> {
		let _ = T::deposit_creating(who, amount);
		Ok(())
	}

	fn withdraw(who: &AccountId, amount: Self::Balance) -> result::Result<(), Self::Error> {
		T::withdraw(
			who,
			amount,
			WithdrawReason::Transfer.into(),
			ExistenceRequirement::AllowDeath,
		)
		.map(|_| ())
	}

	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
		let (_, gap) = T::slash(who, amount);
		gap
	}
}
