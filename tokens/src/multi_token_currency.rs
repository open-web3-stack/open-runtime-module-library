use frame_support::traits::{tokens::currency::MultiTokenCurrency, BalanceStatus};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::result;

/// A currency where funds can be reserved from the user.
pub trait MultiTokenReservableCurrency<AccountId>: MultiTokenCurrency<AccountId> {
	/// Same result as `reserve(who, value)` (but without the side-effects)
	/// assuming there are no balance changes in the meantime.
	fn can_reserve(currency_id: Self::CurrencyId, who: &AccountId, value: Self::Balance) -> bool;

	/// Deducts up to `value` from reserved balance of `who`. This function
	/// cannot fail.
	///
	/// As much funds up to `value` will be deducted as possible. If the reserve
	/// balance of `who` is less than `value`, then a non-zero second item will
	/// be returned.
	fn slash_reserved(
		currency_id: Self::CurrencyId,
		who: &AccountId,
		value: Self::Balance,
	) -> (Self::NegativeImbalance, Self::Balance);

	/// The amount of the balance of a given account that is externally
	/// reserved; this can still get slashed, but gets slashed last of all.
	///
	/// This balance is a 'reserve' balance that other subsystems use in order
	/// to set aside tokens that are still 'owned' by the account holder, but
	/// which are suspendable.
	///
	/// When this balance falls below the value of `ExistentialDeposit`, then
	/// this 'reserve account' is deleted: specifically, `ReservedBalance`.
	///
	/// `system::AccountNonce` is also deleted if `FreeBalance` is also zero (it
	/// also gets collapsed to zero if it ever becomes less than
	/// `ExistentialDeposit`.
	fn reserved_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance;

	/// Moves `value` from balance to reserved balance.
	///
	/// If the free balance is lower than `value`, then no funds will be moved
	/// and an `Err` will be returned to notify of this. This is different
	/// behavior than `unreserve`.
	fn reserve(currency_id: Self::CurrencyId, who: &AccountId, value: Self::Balance) -> DispatchResult;

	/// Moves up to `value` from reserved balance to free balance. This function
	/// cannot fail.
	///
	/// As much funds up to `value` will be moved as possible. If the reserve
	/// balance of `who` is less than `value`, then the remaining amount will be
	/// returned.
	///
	/// # NOTES
	///
	/// - This is different from `reserve`.
	/// - If the remaining reserved balance is less than `ExistentialDeposit`,
	///   it will
	/// invoke `on_reserved_too_low` and could reap the account.
	fn unreserve(currency_id: Self::CurrencyId, who: &AccountId, value: Self::Balance) -> Self::Balance;

	/// Moves up to `value` from reserved balance of account `slashed` to
	/// balance of account `beneficiary`. `beneficiary` must exist for this to
	/// succeed. If it does not, `Err` will be returned. Funds will be placed in
	/// either the `free` balance or the `reserved` balance, depending on the
	/// `status`.
	///
	/// As much funds up to `value` will be deducted as possible. If this is
	/// less than `value`, then `Ok(non_zero)` will be returned.
	fn repatriate_reserved(
		currency_id: Self::CurrencyId,
		slashed: &AccountId,
		beneficiary: &AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError>;
}

pub trait MultiTokenCurrencyExtended<AccountId>: MultiTokenCurrency<AccountId> {
	fn create(address: &AccountId, amount: Self::Balance) -> sp_std::result::Result<Self::CurrencyId, DispatchError>;
	fn mint(currency_id: Self::CurrencyId, address: &AccountId, amount: Self::Balance) -> DispatchResult;
	fn get_next_currency_id() -> Self::CurrencyId;
	fn exists(currency_id: Self::CurrencyId) -> bool;
	fn burn_and_settle(currency_id: Self::CurrencyId, who: &AccountId, amount: Self::Balance) -> DispatchResult;
	fn locked_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance;
	fn available_balance(currency_id: Self::CurrencyId, who: &AccountId) -> Self::Balance;
}
