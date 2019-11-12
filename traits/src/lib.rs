#![cfg_attr(not(feature = "std"), no_std)]

pub mod arithmetic;
pub mod auction;

pub use auction::{Auction, AuctionHandler, AuctionInfo, OnNewBidResult};
use codec::{Codec, FullCodec, HasCompact};
use rstd::{
	cmp::{Eq, PartialEq},
	convert::{TryFrom, TryInto},
	fmt::Debug,
	result,
};
use sr_primitives::traits::{MaybeSerializeDeserialize, SimpleArithmetic};
use srml_support::{
	dispatch,
	traits::{Currency as SrmlCurrency, ExistenceRequirement, WithdrawReason},
};

/// Abstraction over a fungible multi-currency system.
pub trait MultiCurrency<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec + HasCompact + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;

	/// The balance of an account.
	type Balance: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The error type.
	type Error: Into<&'static str>;

	// Public immutables

	/// The total amount of issuance of `currency_id`.
	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance;

	/// The combined balance of `who` under `currency_id`.
	fn balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance;

	// Public mutables

	/// Transfer some amount from one account to another.
	fn transfer(
		currency_id: Self::CurrencyId,
		from: &AccountId,
		to: &AccountId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error>;

	/// Add `amount` to the balance of `who` under `currency_id` and increase total issuance.
	fn deposit(
		currency_id: Self::CurrencyId,
		who: &AccountId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error>;

	/// Remove `amount` from the balance of `who` under `currency_id` and reduce total issuance.
	fn withdraw(
		currency_id: Self::CurrencyId,
		who: &AccountId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error>;

	/// Deduct the balance of `who` by up to `amount`.
	///
	/// As much funds up to `amount` will be deducted as possible, the actual slashed amount will be returned.
	fn slash(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> Self::Balance;
}

/// Extended `MultiCurrency` with additional helper types and methods.
pub trait MultiCurrencyExtended<AccountId>: MultiCurrency<AccountId> {
	/// The type for balance related operations, typically signed int.
	type Amount: arithmetic::Signed
		+ TryInto<Self::Balance>
		+ TryFrom<Self::Balance>
		+ arithmetic::SimpleArithmetic
		+ Codec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default;

	/// Add or remove abs(`by_amount`) from the balance of `who` under `currency_id`. If positive `by_amount`, do add, else do remove.
	fn update_balance(
		currency_id: Self::CurrencyId,
		who: &AccountId,
		by_amount: Self::Amount,
	) -> result::Result<(), Self::Error>;
}

/// Abstraction over a fungible (single) currency system.
pub trait BasicCurrency<AccountId> {
	/// The balance of an account.
	type Balance: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The error type.
	type Error: Into<&'static str>;

	// Public immutables

	/// The total amount of issuance.
	fn total_issuance() -> Self::Balance;

	/// The balance of `who`.
	fn balance(who: &AccountId) -> Self::Balance;

	// Public mutables

	/// Transfer some amount from one account to another.
	fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> result::Result<(), Self::Error>;

	/// Add `amount` to the balance of `who` and increase total issuance.
	fn deposit(who: &AccountId, amount: Self::Balance) -> result::Result<(), Self::Error>;

	/// Remove `amount` from the balance of `who` and reduce total issuance.
	fn withdraw(who: &AccountId, amount: Self::Balance) -> result::Result<(), Self::Error>;

	/// Deduct the balance of `who` by up to `amount`.
	///
	/// As much funds up to `amount` will be deducted as possible, the actual slashed amount will be returned.
	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance;
}

/// Extended `BasicCurrency` with additional helper types and methods.
pub trait BasicCurrencyExtended<AccountId>: BasicCurrency<AccountId> {
	/// The signed type for balance related operations, typically signed int.
	type Amount: arithmetic::Signed
		+ TryInto<Self::Balance>
		+ TryFrom<Self::Balance>
		+ arithmetic::SimpleArithmetic
		+ Codec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default;

	/// Add or remove abs(`by_amount`) from the balance of `who`. If positive `by_amount`, do add, else do remove.
	fn update_balance(who: &AccountId, by_amount: Self::Amount) -> result::Result<(), Self::Error>;
}

// Development notes: The `T: ModuleErrorMetadata` bound is to resolve the potential E0119 error
// (https://doc.rust-lang.org/error-index.html#E0119), which occurs if another type (which is not a runtime `Module`)
// implemented `BasicCurrency`.
impl<AccountId, T: SrmlCurrency<AccountId> + dispatch::ModuleErrorMetadata> BasicCurrency<AccountId> for T {
	type Balance = <T as SrmlCurrency<AccountId>>::Balance;
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
		let imbalance = T::deposit_creating(who, amount);
		drop(imbalance);

		Ok(())
	}

	fn withdraw(who: &AccountId, amount: Self::Balance) -> result::Result<(), Self::Error> {
		T::withdraw(
			who,
			amount,
			WithdrawReason::Transfer.into(),
			ExistenceRequirement::AllowDeath,
		)
		.map(|imbalance| {
			drop(imbalance);
		})
	}

	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
		let (imbalance, slashed_amount) = T::slash(who, amount);
		drop(imbalance);
		slashed_amount
	}
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
pub trait OnNewData<Key, Value> {
	fn on_new_data(key: &Key, value: &Value);
}

pub trait DataProvider<Key, Value> {
	fn get(key: &Key) -> Option<Value>;
}

pub trait PriceProvider<CurrencyId, Price> {
	fn get_price(base: CurrencyId, quote: CurrencyId) -> Option<Price>;
}
