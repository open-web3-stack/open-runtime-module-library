#![cfg_attr(not(feature = "std"), no_std)]

use rstd::{result, fmt::Debug};
use codec::FullCodec;
use sr_primitives::traits::{MaybeSerializeDeserialize, SimpleArithmetic};

/// Abstraction over a fungible multi-currency system.
pub trait MultiCurrency<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec + Copy + MaybeSerializeDeserialize + Debug;

	/// The balance of an account.
	type Balance: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	// Public immutables

	/// The total amount of issuance of `currency_id`.
	fn total_inssuance(currency_id: &Self::CurrencyId) -> Self::Balance;

	/// The combined balance of `who` under `currency_id`.
	fn balance(currency_id: &Self::CurrencyId, who: &AccountId) -> Self::Balance;

	// Public mutables

	/// Transfer some amount from one account to another.
	fn transfer(
		currency_id: &Self::CurrencyId,
		from: &AccountId,
		to: &AccountId,
		amount: Self::Balance,
	) -> result::Result<(), &'static str>;

	/// Mint and increase the total inssuance of `currency_id` by `amount`.
	fn mint(currency_id: &Self::CurrencyId, amount: Self::Balance);

	/// Burn and reduce the total inssuance of `currency_id` by `amount`.
	fn burn(currency_id: &Self::CurrencyId, amount: Self::Balance);

	/// Mint `amount` to the balance of `who`.
	fn deposit(currency_id: &Self::CurrencyId, who: &AccountId, amount: Self::Balance);

	/// Burn `amount` from the balance of `who`.
	fn withdraw(currency_id: &Self::CurrencyId, who: &AccountId, amount: Self::Balance);
}
