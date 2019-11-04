#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use rstd::{fmt::Debug, result};
use sr_primitives::traits::{MaybeSerializeDeserialize, SimpleArithmetic};

pub trait Rebalance<CurrencyId, Balance> {
	/// This is infallible, but doesn't guarantee to rebalance the entire `amount`, for example in the case of
	/// total issuance overflow or underflow.
	fn rebalance(currency_id: CurrencyId, amount: Balance);
}

pub trait Imbalance {
	type Balance;
	type CurrencyId;
	type Opposite: Imbalance;
	type Rebalance: Rebalance<Self::CurrencyId, Self::Balance>;

	fn currency_id(&self) -> Self::CurrencyId;
	fn amount(&self) -> Self::Balance;

	fn rebalance(&self) {
		Self::Rebalance::rebalance(self.currency_id(), self.amount());
	}

	// TODO: add imbalance merge/subsume/split etc.
}

/// Abstraction over a fungible multi-currency system.
pub trait MultiCurrency<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec + Copy + MaybeSerializeDeserialize + Debug;

	/// The balance of an account.
	type Balance: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The opaque token type for an imbalance. This is returned by unbalanced operations
	/// and must be dealt with. It may be dropped but cannot be cloned.
	type PositiveImbalance: Imbalance<
		Balance = Self::Balance,
		CurrencyId = Self::CurrencyId,
		Opposite = Self::NegativeImbalance,
		Rebalance = Self::RebalancePositive,
	>;

	/// The opaque token type for an imbalance. This is returned by unbalanced operations
	/// and must be dealt with. It may be dropped but cannot be cloned.
	type NegativeImbalance: Imbalance<
		Balance = Self::Balance,
		CurrencyId = Self::CurrencyId,
		Opposite = Self::PositiveImbalance,
		Rebalance = Self::RebalanceNegative,
	>;

	type RebalancePositive: Rebalance<Self::CurrencyId, Self::Balance>;
	type RebalanceNegative: Rebalance<Self::CurrencyId, Self::Balance>;

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
	) -> result::Result<(), &'static str>;

	/// Add `amount` to the balance of `who` under `currency_id`. Returns a `PositiveImbalance`.
	fn deposit(
		currency_id: Self::CurrencyId,
		who: &AccountId,
		amount: Self::Balance,
	) -> result::Result<Self::PositiveImbalance, &'static str>;

	/// Remove `amount` from the balance of `who` under `currency_id`. Returns a `NegativeImbalance`.
	fn withdraw(
		currency_id: Self::CurrencyId,
		who: &AccountId,
		amount: Self::Balance,
	) -> result::Result<Self::NegativeImbalance, &'static str>;

	/// Deduct the balance of `who` by up to `amount`.
	///
	/// As much funds up to `amount` will be deducted as possible. Returns a `NegativeImbalance` with the actual
	/// slashed amount.
	fn slash(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> Self::NegativeImbalance;
}
