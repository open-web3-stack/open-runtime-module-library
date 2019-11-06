#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use rstd::{fmt::Debug, result};
use sr_primitives::traits::{MaybeSerializeDeserialize, SimpleArithmetic};

/// Abstraction over a fungible multi-currency system.
pub trait MultiCurrency<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec + Copy + MaybeSerializeDeserialize + Debug;

	/// The balance of an account.
	type Balance: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The error type.
	type Error: Into<&'static str>;

	// Public immutables

	/// The total amount of issuance of `currency_id`.
	fn total_inssuance(currency_id: Self::CurrencyId) -> Self::Balance;

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

	/// Remove `amount` from the balance of `who` under `currency_id` and recude total issuance.
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
