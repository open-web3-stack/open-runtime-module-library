//! # Tokens Module
//!
//! ## Overview
//!
//! The tokens module provides fungible multi-currency functionality that
//! implements `MultiCurrency` trait.
//!
//! The tokens module provides functions for:
//!
//! - Querying and setting the balance of a given account.
//! - Getting and managing total issuance.
//! - Balance transfer between accounts.
//! - Depositing and withdrawing balance.
//! - Slashing an account balance.
//!
//! ### Implementations
//!
//! The tokens module provides implementations for following traits.
//!
//! - `MultiCurrency` - Abstraction over a fungible multi-currency system.
//! - `MultiCurrencyExtended` - Extended `MultiCurrency` with additional helper
//!   types and methods, like updating balance
//! by a given signed integer amount.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `transfer` - Transfer some balance to another account.
//! - `transfer_all` - Transfer all balance to another account.
//!
//! ### Genesis Config
//!
//! The tokens module depends on the `GenesisConfig`. Endowed accounts could be
//! configured in genesis configs.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::dispatch::result::Result;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::Get,
	traits::{BalanceStatus, ExistenceRequirement, Imbalance, LockIdentifier, SignedImbalance, WithdrawReasons},
	weights::Weight,
	Parameter, StorageMap,
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::traits::One;
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, Saturating,
		StaticLookup, Zero,
	},
	DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::{
	convert::{TryFrom, TryInto},
	marker,
	prelude::*,
	result,
};

#[cfg(feature = "std")]
use sp_std::collections::btree_map::BTreeMap;

pub use crate::imbalances::{NegativeImbalance, PositiveImbalance};
use mangata_primitives::{Amount, Balance, TokenId};

mod default_weight;
mod imbalances;
mod mock;
mod multi_token_currency;
mod multi_token_imbalances;
mod tests;

use frame_support::traits::{
	Currency as PalletCurrency, LockableCurrency as PalletLockableCurrency,
	ReservableCurrency as PalletReservableCurrency,
};
pub use orml_traits::MultiCurrency;
use orml_traits::{
	arithmetic::{self, Signed},
	MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency, OnReceived,
};

use codec::FullCodec;
pub use multi_token_currency::{
	MultiTokenCurrency, MultiTokenCurrencyExtended, MultiTokenLockableCurrency, MultiTokenReservableCurrency,
};

pub use multi_token_imbalances::{
	NegativeImbalance as MultiTokenNegativeImbalance, PositiveImbalance as MultiTokenPositiveImbalance,
};

pub use multi_token_imbalances::MultiTokenImbalanceWithZeroTrait;

pub trait WeightInfo {
	fn transfer() -> Weight;
	fn transfer_all() -> Weight;
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The balance type
	type Balance: Parameter
		+ Member
		+ AtLeast32BitUnsigned
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ From<Balance>
		+ Into<Balance>;

	/// The amount type, should be signed version of `Balance`
	type Amount: Signed
		+ TryInto<Self::Balance>
		+ TryFrom<Self::Balance>
		+ Parameter
		+ Member
		+ arithmetic::SimpleArithmetic
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ From<Amount>
		+ Into<Amount>;

	/// The currency ID type
	type CurrencyId: Parameter
		+ Member
		+ Copy
		+ MaybeSerializeDeserialize
		+ Ord
		+ Default
		+ AtLeast32BitUnsigned
		+ FullCodec
		+ From<TokenId>
		+ Into<TokenId>;

	/// Hook when some fund is deposited into an account
	type OnReceived: OnReceived<Self::AccountId, Self::CurrencyId, Self::Balance>;

	/// Weight information for extrinsics in this module.
	type WeightInfo: WeightInfo;
}

/// A single lock on a balance. There can be many of these on an account and
/// they "overlap", so the same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct BalanceLock<Balance> {
	/// An identifier for this lock. Only one lock may be in existence for each
	/// identifier.
	pub id: LockIdentifier,
	/// The amount which the free balance may not drop below when this lock is
	/// in effect.
	pub amount: Balance,
}

/// balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug)]
pub struct AccountData<Balance> {
	/// Non-reserved part of the balance. There may still be restrictions on
	/// this, but it is the total pool what may in principle be transferred,
	/// reserved.
	///
	/// This is the only balance that matters in terms of most operations on
	/// tokens.
	pub free: Balance,
	/// Balance which is reserved and may not be used at all.
	///
	/// This can still get slashed, but gets slashed last of all.
	///
	/// This balance is a 'reserve' balance that other subsystems use in order
	/// to set aside tokens that are still 'owned' by the account holder, but
	/// which are suspendable.
	pub reserved: Balance,
	/// The amount that `free` may not drop below when withdrawing.
	pub frozen: Balance,
}

impl<Balance: Saturating + Copy + Ord> AccountData<Balance> {
	/// The amount that this account's free balance may not be reduced beyond.
	fn frozen(&self) -> Balance {
		self.frozen
	}
	/// The total balance in this account including any that is reserved and
	/// ignoring any frozen.
	fn total(&self) -> Balance {
		self.free.saturating_add(self.reserved)
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Tokens {
		/// The total issuance of a token type.
		pub TotalIssuance get(fn total_issuance): map hasher(twox_64_concat) T::CurrencyId => T::Balance; 

		/// Any liquidity locks of a token type under an account.
		/// NOTE: Should only be accessed when setting, changing and freeing a lock.
		pub Locks get(fn locks): double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) T::CurrencyId => Vec<BalanceLock<T::Balance>>;

		/// The balance of a token type under an account.
		///
		/// NOTE: If the total is ever zero, decrease account ref account.
		///
		/// NOTE: This is only used in the case that this module is used to store balances.
		pub Accounts get(fn accounts): double_map hasher(blake2_128_concat) T::AccountId, hasher(twox_64_concat) T::CurrencyId => AccountData<T::Balance>;

		pub NextCurrencyId get(fn next_asset_id): T::CurrencyId;
	}
	add_extra_genesis {
		config(tokens_endowment): Vec<(T::AccountId, T::CurrencyId, T::Balance)>;
		config(created_tokens_for_staking): Vec<(T::AccountId, T::CurrencyId, T::Balance)>;

		build(|config: &GenesisConfig<T>| {
			config.tokens_endowment.iter().for_each(|(account_id, token_id, initial_balance)| {
				if MultiTokenCurrencyAdapter::<T>::exists(*token_id){
					assert!(MultiTokenCurrencyAdapter::<T>::mint(*token_id, account_id, *initial_balance).is_ok(), "Tokens mint failed");
				}else{
					let created_token_id = MultiTokenCurrencyAdapter::<T>::create(account_id, *initial_balance);
					assert!(created_token_id == *token_id, "Assets not initialized in the expected sequence");
				}
			});
			config.created_tokens_for_staking.iter().for_each(|(account_id, token_id, initial_balance)| {
				if MultiTokenCurrencyAdapter::<T>::exists(*token_id){
					assert!(MultiTokenCurrencyAdapter::<T>::mint(*token_id, account_id, *initial_balance).is_ok(), "Tokens mint failed");
				}else{
					let created_token_id = MultiTokenCurrencyAdapter::<T>::create(account_id, *initial_balance);
					assert!(created_token_id == *token_id, "Assets not initialized in the expected sequence");
				}
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
		/// Token transfer success. [currency_id, from, to, amount]
		Transferred(CurrencyId, AccountId, AccountId, Balance),
		Issued(CurrencyId, AccountId, Balance),
		Minted(CurrencyId, AccountId, Balance),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Transfer some balance to another account.
		///
		/// The dispatch origin for this call must be `Signed` by the transactor.
		///
		/// # <weight>
		/// - Complexity: `O(1)`
		/// - Db reads: 4
		/// - Db writes: 2
		/// -------------------
		/// Base Weight: 84.08 µs
		/// # </weight>
		#[weight = T::WeightInfo::transfer()]
		pub fn transfer(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			token_id: TokenId,
			#[compact] value: Balance,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			let currency_id: T::CurrencyId = token_id.into();
			let amount: T::Balance = value.into();
			<Self as MultiCurrency<_>>::transfer(currency_id, &from, &to, amount)?;

			Self::deposit_event(RawEvent::Transferred(currency_id, from, to, amount));
		}

		/// Transfer all remaining balance to the given account.
		///
		/// The dispatch origin for this call must be `Signed` by the transactor.
		///
		/// # <weight>
		/// - Complexity: `O(1)`
		/// - Db reads: 4
		/// - Db writes: 2
		/// -------------------
		/// Base Weight: 87.71 µs
		/// # </weight>
		#[weight = T::WeightInfo::transfer_all()]
		pub fn transfer_all(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			token_id: TokenId,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			let currency_id: T::CurrencyId = token_id.into();
			let balance = <Self as MultiCurrency<T::AccountId>>::free_balance(currency_id, &from);
			<Self as MultiCurrency<T::AccountId>>::transfer(currency_id, &from, &to, balance)?;

			Self::deposit_event(RawEvent::Transferred(currency_id, from, to, balance));
		}

		#[weight = 10_000]
		pub fn create(
			origin,
			account_id: T::AccountId,
			value: Balance,
		) {
			ensure_root(origin)?;
			let amount: T::Balance = value.into();
			let currency_id = MultiTokenCurrencyAdapter::<T>::create(&account_id, amount);
			Self::deposit_event(RawEvent::Issued(currency_id, account_id, amount));
		}

		#[weight = 10_000]
		pub fn mint(
			origin,
			token_id: TokenId,
			account_id: T::AccountId,
			value: Balance,
		) {
			ensure_root(origin)?;
			let currency_id: T::CurrencyId = token_id.into();
			let amount: T::Balance = value.into();
			MultiTokenCurrencyAdapter::<T>::mint(currency_id, &account_id, amount)?;
			Self::deposit_event(RawEvent::Minted(currency_id, account_id, amount));
		}
	}
}

decl_error! {
	/// Error for token module.
	pub enum Error for Module<T: Trait> {
		/// The balance is too low
		BalanceTooLow,
		/// This operation will cause balance to overflow
		BalanceOverflow,
		/// This operation will cause total issuance to overflow
		TotalIssuanceOverflow,
		/// Cannot convert Amount into Balance type
		AmountIntoBalanceFailed,
		/// Failed because liquidity restrictions due to locking
		LiquidityRestrictions,
		/// Failed because token with given id does not exits
		TokenIdNotExists,
	}
}

impl<T: Trait> Module<T> {
	/// Set free balance of `who` to a new value.
	///
	/// Note this will not maintain total issuance.
	fn set_free_balance(currency_id: T::CurrencyId, who: &T::AccountId, balance: T::Balance) {
		<Accounts<T>>::mutate(who, currency_id, |account_data| account_data.free = balance);
	}

	/// Set reserved balance of `who` to a new value, meanwhile enforce
	/// existential rule.
	///
	/// Note this will not maintain total issuance, and the caller is expected
	/// to do it.
	fn set_reserved_balance(currency_id: T::CurrencyId, who: &T::AccountId, balance: T::Balance) {
		<Accounts<T>>::mutate(who, currency_id, |account_data| account_data.reserved = balance);
	}

	/// Update the account entry for `who` under `currency_id`, given the locks.
	fn update_locks(currency_id: T::CurrencyId, who: &T::AccountId, locks: &[BalanceLock<T::Balance>]) {
		// update account data
		<Accounts<T>>::mutate(who, currency_id, |account_data| {
			account_data.frozen = Zero::zero();
			for lock in locks.iter() {
				account_data.frozen = account_data.frozen.max(lock.amount);
			}
		});

		// update locks
		let existed = <Locks<T>>::contains_key(who, currency_id);
		if locks.is_empty() {
			<Locks<T>>::remove(who, currency_id);
			if existed {
				// decrease account ref count when destruct lock
				frame_system::Module::<T>::dec_ref(who);
			}
		} else {
			<Locks<T>>::insert(who, currency_id, locks);
			if !existed {
				// increase account ref count when initialize lock
				frame_system::Module::<T>::inc_ref(who);
			}
		}
	}
}

impl<T: Trait> MultiCurrency<T::AccountId> for Module<T> {
	type CurrencyId = T::CurrencyId;
	type Balance = T::Balance;

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		<TotalIssuance<T>>::get(currency_id)
	}

	fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, currency_id).total()
	}

	fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, currency_id).free
	}

	// Ensure that an account can withdraw from their free balance given any
	// existing withdrawal restrictions like locks and vesting balance.
	// Is a no-op if amount to be withdrawn is zero.
	fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}

		let new_balance = Self::free_balance(currency_id, who)
			.checked_sub(&amount)
			.ok_or(Error::<T>::BalanceTooLow)?;
		ensure!(
			new_balance >= Self::accounts(who, currency_id).frozen(),
			Error::<T>::LiquidityRestrictions
		);
		Ok(())
	}

	/// Transfer some free balance from `from` to `to`.
	/// Is a no-op if value to be transferred is zero or the `from` is the same
	/// as `to`.
	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() || from == to {
			return Ok(());
		}
		Self::ensure_can_withdraw(currency_id, from, amount)?;

		let from_balance = Self::free_balance(currency_id, from);
		let to_balance = Self::free_balance(currency_id, to)
			.checked_add(&amount)
			.ok_or(Error::<T>::BalanceOverflow)?;
		// Cannot underflow because ensure_can_withdraw check
		Self::set_free_balance(currency_id, from, from_balance - amount);
		Self::set_free_balance(currency_id, to, to_balance);
		T::OnReceived::on_received(to, currency_id, amount);

		Ok(())
	}

	/// Deposit some `amount` into the free balance of account `who`.
	///
	/// Is a no-op if the `amount` to be deposited is zero.
	fn deposit(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}

		let new_total = Self::total_issuance(currency_id)
			.checked_add(&amount)
			.ok_or(Error::<T>::TotalIssuanceOverflow)?;
		<TotalIssuance<T>>::insert(currency_id, new_total);
		Self::set_free_balance(currency_id, who, Self::free_balance(currency_id, who) + amount);
		T::OnReceived::on_received(who, currency_id, amount);

		Ok(())
	}

	fn withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		Self::ensure_can_withdraw(currency_id, who, amount)?;

		// Cannot underflow because ensure_can_withdraw check
		<TotalIssuance<T>>::mutate(currency_id, |v| *v -= amount);
		Self::set_free_balance(currency_id, who, Self::free_balance(currency_id, who) - amount);

		Ok(())
	}

	// Check if `value` amount of free balance can be slashed from `who`.
	fn can_slash(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
		if value.is_zero() {
			return true;
		}
		Self::free_balance(currency_id, who) >= value
	}

	/// Is a no-op if `value` to be slashed is zero.
	///
	/// NOTE: `slash()` prefers free balance, but assumes that reserve balance
	/// can be drawn from in extreme circumstances. `can_slash()` should be used
	/// prior to `slash()` to avoid having to draw from reserved funds, however
	/// we err on the side of punishment if things are inconsistent
	/// or `can_slash` wasn't used appropriately.
	fn slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		if amount.is_zero() {
			return amount;
		}

		let account = Self::accounts(who, currency_id);
		let free_slashed_amount = account.free.min(amount);
		// Cannot underflow becuase free_slashed_amount can never be greater than amount
		let mut remaining_slash = amount - free_slashed_amount;

		// slash free balance
		if !free_slashed_amount.is_zero() {
			// Cannot underflow becuase free_slashed_amount can never be greater than
			// account.free
			Self::set_free_balance(currency_id, who, account.free - free_slashed_amount);
		}

		// slash reserved balance
		if !remaining_slash.is_zero() {
			let reserved_slashed_amount = account.reserved.min(remaining_slash);
			// Cannot underflow due to above line
			remaining_slash -= reserved_slashed_amount;
			Self::set_reserved_balance(currency_id, who, account.reserved - reserved_slashed_amount);
		}

		// Cannot underflow because the slashed value cannot be greater than total
		// issuance
		<TotalIssuance<T>>::mutate(currency_id, |v| *v -= amount - remaining_slash);
		remaining_slash
	}
}

impl<T: Trait> MultiCurrencyExtended<T::AccountId> for Module<T> {
	type Amount = T::Amount;

	fn update_balance(currency_id: Self::CurrencyId, who: &T::AccountId, by_amount: Self::Amount) -> DispatchResult {
		if by_amount.is_zero() {
			return Ok(());
		}

		// Ensure this doesn't overflow. There isn't any traits that exposes
		// `saturating_abs` so we need to do it manually.
		let by_amount_abs = if by_amount == Self::Amount::min_value() {
			Self::Amount::max_value()
		} else {
			by_amount.abs()
		};

		let by_balance =
			TryInto::<Self::Balance>::try_into(by_amount_abs).map_err(|_| Error::<T>::AmountIntoBalanceFailed)?;
		if by_amount.is_positive() {
			Self::deposit(currency_id, who, by_balance)
		} else {
			Self::withdraw(currency_id, who, by_balance).map(|_| ())
		}
	}
}

impl<T: Trait> MultiLockableCurrency<T::AccountId> for Module<T> {
	type Moment = T::BlockNumber;

	// Set a lock on the balance of `who` under `currency_id`.
	// Is a no-op if lock amount is zero.
	fn set_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) {
		if amount.is_zero() {
			return;
		}
		let mut new_lock = Some(BalanceLock { id: lock_id, amount });
		let mut locks = Self::locks(who, currency_id)
			.into_iter()
			.filter_map(|lock| {
				if lock.id == lock_id {
					new_lock.take()
				} else {
					Some(lock)
				}
			})
			.collect::<Vec<_>>();
		if let Some(lock) = new_lock {
			locks.push(lock)
		}
		Self::update_locks(currency_id, who, &locks[..]);
	}

	// Extend a lock on the balance of `who` under `currency_id`.
	// Is a no-op if lock amount is zero
	fn extend_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) {
		if amount.is_zero() {
			return;
		}
		let mut new_lock = Some(BalanceLock { id: lock_id, amount });
		let mut locks = Self::locks(who, currency_id)
			.into_iter()
			.filter_map(|lock| {
				if lock.id == lock_id {
					new_lock.take().map(|nl| BalanceLock {
						id: lock.id,
						amount: lock.amount.max(nl.amount),
					})
				} else {
					Some(lock)
				}
			})
			.collect::<Vec<_>>();
		if let Some(lock) = new_lock {
			locks.push(lock)
		}
		Self::update_locks(currency_id, who, &locks[..]);
	}

	fn remove_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId) {
		let mut locks = Self::locks(who, currency_id);
		locks.retain(|lock| lock.id != lock_id);
		Self::update_locks(currency_id, who, &locks[..]);
	}
}

impl<T: Trait> MultiReservableCurrency<T::AccountId> for Module<T> {
	/// Check if `who` can reserve `value` from their free balance.
	///
	/// Always `true` if value to be reserved is zero.
	fn can_reserve(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
		if value.is_zero() {
			return true;
		}
		Self::ensure_can_withdraw(currency_id, who, value).is_ok()
	}

	/// Slash from reserved balance, returning any amount that was unable to be
	/// slashed.
	///
	/// Is a no-op if the value to be slashed is zero.
	fn slash_reserved(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		if value.is_zero() {
			return value;
		}

		let reserved_balance = Self::reserved_balance(currency_id, who);
		let actual = reserved_balance.min(value);
		Self::set_reserved_balance(currency_id, who, reserved_balance - actual);
		<TotalIssuance<T>>::mutate(currency_id, |v| *v -= actual);
		value - actual
	}

	fn reserved_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, currency_id).reserved
	}

	/// Move `value` from the free balance from `who` to their reserved balance.
	///
	/// Is a no-op if value to be reserved is zero.
	fn reserve(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> DispatchResult {
		if value.is_zero() {
			return Ok(());
		}
		Self::ensure_can_withdraw(currency_id, who, value)?;

		let account = Self::accounts(who, currency_id);
		Self::set_free_balance(currency_id, who, account.free - value);
		// Cannot overflow becuase total issuance is using the same balance type and
		// this doesn't increase total issuance
		Self::set_reserved_balance(currency_id, who, account.reserved + value);
		Ok(())
	}

	/// Unreserve some funds, returning any amount that was unable to be
	/// unreserved.
	///
	/// Is a no-op if the value to be unreserved is zero.
	fn unreserve(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		if value.is_zero() {
			return value;
		}

		let account = Self::accounts(who, currency_id);
		let actual = account.reserved.min(value);
		Self::set_reserved_balance(currency_id, who, account.reserved - actual);
		Self::set_free_balance(currency_id, who, account.free + actual);
		T::OnReceived::on_received(who, currency_id, actual);
		value - actual
	}

	/// Move the reserved balance of one account into the balance of another,
	/// according to `status`.
	///
	/// Is a no-op if:
	/// - the value to be moved is zero; or
	/// - the `slashed` id equal to `beneficiary` and the `status` is
	///   `Reserved`.
	fn repatriate_reserved(
		currency_id: Self::CurrencyId,
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		if value.is_zero() {
			return Ok(value);
		}

		if slashed == beneficiary {
			return match status {
				BalanceStatus::Free => Ok(Self::unreserve(currency_id, slashed, value)),
				BalanceStatus::Reserved => Ok(value.saturating_sub(Self::reserved_balance(currency_id, slashed))),
			};
		}

		let from_account = Self::accounts(slashed, currency_id);
		let to_account = Self::accounts(beneficiary, currency_id);
		let actual = from_account.reserved.min(value);
		match status {
			BalanceStatus::Free => {
				Self::set_free_balance(currency_id, beneficiary, to_account.free + actual);
				T::OnReceived::on_received(beneficiary, currency_id, actual);
			}
			BalanceStatus::Reserved => {
				Self::set_reserved_balance(currency_id, beneficiary, to_account.reserved + actual);
			}
		}
		Self::set_reserved_balance(currency_id, slashed, from_account.reserved - actual);
		Ok(value - actual)
	}
}

pub struct CurrencyAdapter<T, GetCurrencyId>(marker::PhantomData<(T, GetCurrencyId)>);

impl<T, GetCurrencyId> PalletCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Trait,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type Balance = T::Balance;
	type PositiveImbalance = PositiveImbalance<T, GetCurrencyId>;
	type NegativeImbalance = NegativeImbalance<T, GetCurrencyId>;

	fn total_balance(who: &T::AccountId) -> Self::Balance {
		Module::<T>::total_balance(GetCurrencyId::get(), who)
	}

	fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
		Module::<T>::can_slash(GetCurrencyId::get(), who, value)
	}

	fn total_issuance() -> Self::Balance {
		Module::<T>::total_issuance(GetCurrencyId::get())
	}

	fn minimum_balance() -> Self::Balance {
		Zero::zero()
	}

	fn burn(mut amount: Self::Balance) -> Self::PositiveImbalance {
		if amount.is_zero() {
			return PositiveImbalance::zero();
		}
		<TotalIssuance<T>>::mutate(GetCurrencyId::get(), |issued| {
			*issued = issued.checked_sub(&amount).unwrap_or_else(|| {
				amount = *issued;
				Zero::zero()
			});
		});
		PositiveImbalance::new(amount)
	}

	fn issue(mut amount: Self::Balance) -> Self::NegativeImbalance {
		if amount.is_zero() {
			return NegativeImbalance::zero();
		}
		<TotalIssuance<T>>::mutate(GetCurrencyId::get(), |issued| {
			*issued = issued.checked_add(&amount).unwrap_or_else(|| {
				amount = Self::Balance::max_value() - *issued;
				Self::Balance::max_value()
			})
		});
		NegativeImbalance::new(amount)
	}

	fn free_balance(who: &T::AccountId) -> Self::Balance {
		Module::<T>::free_balance(GetCurrencyId::get(), who)
	}

	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
		_new_balance: Self::Balance,
	) -> DispatchResult {
		Module::<T>::ensure_can_withdraw(GetCurrencyId::get(), who, amount)
	}

	fn transfer(
		source: &T::AccountId,
		dest: &T::AccountId,
		value: Self::Balance,
		_existence_requirement: ExistenceRequirement,
	) -> DispatchResult {
		<Module<T> as MultiCurrency<T::AccountId>>::transfer(GetCurrencyId::get(), &source, &dest, value)
	}

	fn slash(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		if value.is_zero() {
			return (Self::NegativeImbalance::zero(), value);
		}

		let currency_id = GetCurrencyId::get();
		let account = Module::<T>::accounts(who, currency_id);
		let free_slashed_amount = account.free.min(value);
		let mut remaining_slash = value - free_slashed_amount;

		// slash free balance
		if !free_slashed_amount.is_zero() {
			Module::<T>::set_free_balance(currency_id, who, account.free - free_slashed_amount);
		}

		// slash reserved balance
		if !remaining_slash.is_zero() {
			let reserved_slashed_amount = account.reserved.min(remaining_slash);
			remaining_slash -= reserved_slashed_amount;
			Module::<T>::set_reserved_balance(currency_id, who, account.reserved - reserved_slashed_amount);
			(
				Self::NegativeImbalance::new(free_slashed_amount + reserved_slashed_amount),
				remaining_slash,
			)
		} else {
			(Self::NegativeImbalance::new(value), remaining_slash)
		}
	}

	fn deposit_into_existing(
		who: &T::AccountId,
		value: Self::Balance,
	) -> result::Result<Self::PositiveImbalance, DispatchError> {
		if value.is_zero() {
			return Ok(Self::PositiveImbalance::zero());
		}
		let currency_id = GetCurrencyId::get();
		let new_total = Module::<T>::free_balance(currency_id, who)
			.checked_add(&value)
			.ok_or(Error::<T>::TotalIssuanceOverflow)?;
		Module::<T>::set_free_balance(currency_id, who, new_total);

		Ok(Self::PositiveImbalance::new(value))
	}

	fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
		Self::deposit_into_existing(who, value).unwrap_or_else(|_| Self::PositiveImbalance::zero())
	}

	fn withdraw(
		who: &T::AccountId,
		value: Self::Balance,
		_reasons: WithdrawReasons,
		_liveness: ExistenceRequirement,
	) -> result::Result<Self::NegativeImbalance, DispatchError> {
		if value.is_zero() {
			return Ok(Self::NegativeImbalance::zero());
		}
		let currency_id = GetCurrencyId::get();
		Module::<T>::ensure_can_withdraw(currency_id, who, value)?;
		Module::<T>::set_free_balance(currency_id, who, Module::<T>::free_balance(currency_id, who) - value);

		Ok(Self::NegativeImbalance::new(value))
	}

	fn make_free_balance_be(
		who: &T::AccountId,
		value: Self::Balance,
	) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
		<Accounts<T>>::mutate(
			who,
			GetCurrencyId::get(),
			|account| -> Result<SignedImbalance<Self::Balance, Self::PositiveImbalance>, ()> {
				let imbalance = if account.free <= value {
					SignedImbalance::Positive(PositiveImbalance::new(value - account.free))
				} else {
					SignedImbalance::Negative(NegativeImbalance::new(account.free - value))
				};
				account.free = value;
				Ok(imbalance)
			},
		)
		.unwrap_or_else(|_| SignedImbalance::Positive(Self::PositiveImbalance::zero()))
	}
}

impl<T, GetCurrencyId> PalletReservableCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Trait,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
		Module::<T>::can_reserve(GetCurrencyId::get(), who, value)
	}

	fn slash_reserved(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		let actual = Module::<T>::slash_reserved(GetCurrencyId::get(), who, value);
		(Self::NegativeImbalance::zero(), actual)
	}

	fn reserved_balance(who: &T::AccountId) -> Self::Balance {
		Module::<T>::reserved_balance(GetCurrencyId::get(), who)
	}

	fn reserve(who: &T::AccountId, value: Self::Balance) -> DispatchResult {
		Module::<T>::reserve(GetCurrencyId::get(), who, value)
	}

	fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		Module::<T>::unreserve(GetCurrencyId::get(), who, value)
	}

	fn repatriate_reserved(
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		Module::<T>::repatriate_reserved(GetCurrencyId::get(), slashed, beneficiary, value, status)
	}
}

impl<T, GetCurrencyId> PalletLockableCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Trait,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type Moment = T::BlockNumber;
	type MaxLocks = ();

	fn set_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, _reasons: WithdrawReasons) {
		Module::<T>::set_lock(id, GetCurrencyId::get(), who, amount)
	}

	fn extend_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, _reasons: WithdrawReasons) {
		Module::<T>::extend_lock(id, GetCurrencyId::get(), who, amount)
	}

	fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
		Module::<T>::remove_lock(id, GetCurrencyId::get(), who)
	}
}

pub struct MultiTokenCurrencyAdapter<T>(marker::PhantomData<T>);

impl<T> MultiTokenCurrency<T::AccountId> for MultiTokenCurrencyAdapter<T>
where
	T: Trait,
{
	type Balance = T::Balance;
	type CurrencyId = T::CurrencyId;
	type PositiveImbalance = MultiTokenPositiveImbalance<T>;
	type NegativeImbalance = MultiTokenNegativeImbalance<T>;

	fn total_balance(currency_id: T::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Module::<T>::total_balance(currency_id, who)
	}

	fn can_slash(currency_id: T::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
		Module::<T>::can_slash(currency_id, who, value)
	}

	fn total_issuance(currency_id: T::CurrencyId) -> Self::Balance {
		Module::<T>::total_issuance(currency_id)
	}

	fn minimum_balance(_currency_id: T::CurrencyId) -> Self::Balance {
		Zero::zero()
	}

	fn burn(currency_id: T::CurrencyId, mut amount: Self::Balance) -> Self::PositiveImbalance {
		if amount.is_zero() {
			return MultiTokenPositiveImbalance::zero(currency_id);
		}

		<TotalIssuance<T>>::mutate(currency_id, |issued| {
			*issued = issued.checked_sub(&amount).unwrap_or_else(|| {
				amount = *issued;
				Zero::zero()
			});
		});
		MultiTokenPositiveImbalance::new(currency_id, amount)
	}

	// NOTE: should not be called directly - may invalidate NextCurrencyId ids
	fn issue(currency_id: T::CurrencyId, mut amount: Self::Balance) -> Self::NegativeImbalance {
		if amount.is_zero() {
			return MultiTokenNegativeImbalance::zero(currency_id);
		}
		<TotalIssuance<T>>::mutate(currency_id, |issued| {
			*issued = issued.checked_add(&amount).unwrap_or_else(|| {
				amount = Self::Balance::max_value() - *issued;
				Self::Balance::max_value()
			})
		});
		MultiTokenNegativeImbalance::new(currency_id, amount)
	}

	fn free_balance(currency_id: T::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Module::<T>::free_balance(currency_id, who)
	}

	fn ensure_can_withdraw(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
		_new_balance: Self::Balance,
	) -> DispatchResult {
		Module::<T>::ensure_can_withdraw(currency_id, who, amount)
	}

	fn transfer(
		currency_id: T::CurrencyId,
		source: &T::AccountId,
		dest: &T::AccountId,
		value: Self::Balance,
		_existence_requirement: ExistenceRequirement,
	) -> DispatchResult {
		<Module<T> as MultiCurrency<T::AccountId>>::transfer(currency_id, &source, &dest, value)
	}

	fn slash(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> (Self::NegativeImbalance, Self::Balance) {
		if value.is_zero() {
			return (MultiTokenNegativeImbalance::zero(currency_id), value);
		}

		let account = Module::<T>::accounts(who, currency_id);
		let free_slashed_amount = account.free.min(value);
		let mut remaining_slash = value - free_slashed_amount;

		// slash free balance
		if !free_slashed_amount.is_zero() {
			Module::<T>::set_free_balance(currency_id, who, account.free - free_slashed_amount);
		}

		// slash reserved balance
		if !remaining_slash.is_zero() {
			let reserved_slashed_amount = account.reserved.min(remaining_slash);
			remaining_slash -= reserved_slashed_amount;
			Module::<T>::set_reserved_balance(currency_id, who, account.reserved - reserved_slashed_amount);
			(
				Self::NegativeImbalance::new(currency_id, free_slashed_amount + reserved_slashed_amount),
				remaining_slash,
			)
		} else {
			(Self::NegativeImbalance::new(currency_id, value), remaining_slash)
		}
	}

	fn deposit_into_existing(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> result::Result<Self::PositiveImbalance, DispatchError> {
		if value.is_zero() {
			return Ok(MultiTokenPositiveImbalance::zero(currency_id));
		}
		let new_total = Module::<T>::free_balance(currency_id, who)
			.checked_add(&value)
			.ok_or(Error::<T>::TotalIssuanceOverflow)?;
		Module::<T>::set_free_balance(currency_id, who, new_total);

		Ok(Self::PositiveImbalance::new(currency_id, value))
	}

	fn deposit_creating(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::PositiveImbalance {
		Self::deposit_into_existing(currency_id, who, value)
			.unwrap_or_else(|_| MultiTokenPositiveImbalance::zero(currency_id))
	}

	fn withdraw(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
		_reasons: WithdrawReasons,
		_liveness: ExistenceRequirement,
	) -> result::Result<Self::NegativeImbalance, DispatchError> {
		if value.is_zero() {
			return Ok(MultiTokenNegativeImbalance::zero(currency_id));
		}
		Module::<T>::ensure_can_withdraw(currency_id, who, value)?;
		Module::<T>::set_free_balance(currency_id, who, Module::<T>::free_balance(currency_id, who) - value);

		Ok(Self::NegativeImbalance::new(currency_id, value))
	}

	fn make_free_balance_be(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
		<Accounts<T>>::mutate(
			who,
			currency_id,
			|account| -> Result<SignedImbalance<Self::Balance, Self::PositiveImbalance>, ()> {
				let imbalance = if account.free <= value {
					SignedImbalance::Positive(MultiTokenPositiveImbalance::new(currency_id, value - account.free))
				} else {
					SignedImbalance::Negative(MultiTokenNegativeImbalance::new(currency_id, account.free - value))
				};
				account.free = value;
				Ok(imbalance)
			},
		)
		.unwrap_or_else(|_| SignedImbalance::Positive(MultiTokenPositiveImbalance::zero(currency_id)))
	}
}

impl<T> MultiTokenReservableCurrency<T::AccountId> for MultiTokenCurrencyAdapter<T>
where
	T: Trait,
{
	fn can_reserve(currency_id: T::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
		Module::<T>::can_reserve(currency_id, who, value)
	}

	fn slash_reserved(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> (Self::NegativeImbalance, Self::Balance) {
		let actual = Module::<T>::slash_reserved(currency_id, who, value);
		(MultiTokenNegativeImbalance::zero(currency_id), actual)
	}

	fn reserved_balance(currency_id: T::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Module::<T>::reserved_balance(currency_id, who)
	}

	fn reserve(currency_id: T::CurrencyId, who: &T::AccountId, value: Self::Balance) -> DispatchResult {
		Module::<T>::reserve(currency_id, who, value)
	}

	fn unreserve(currency_id: T::CurrencyId, who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		Module::<T>::unreserve(currency_id, who, value)
	}

	fn repatriate_reserved(
		currency_id: T::CurrencyId,
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: BalanceStatus,
	) -> result::Result<Self::Balance, DispatchError> {
		Module::<T>::repatriate_reserved(currency_id, slashed, beneficiary, value, status)
	}
}

impl<T> MultiTokenLockableCurrency<T::AccountId> for MultiTokenCurrencyAdapter<T>
where
	T: Trait,
{
	type Moment = T::BlockNumber;
	type MaxLocks = ();

	fn set_lock(
		currency_id: T::CurrencyId,
		id: LockIdentifier,
		who: &T::AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
	) {
		Module::<T>::set_lock(id, currency_id, who, amount)
	}

	fn extend_lock(
		currency_id: T::CurrencyId,
		id: LockIdentifier,
		who: &T::AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
	) {
		Module::<T>::extend_lock(id, currency_id, who, amount)
	}

	fn remove_lock(currency_id: T::CurrencyId, id: LockIdentifier, who: &T::AccountId) {
		Module::<T>::remove_lock(id, currency_id, who)
	}
}

impl<T> MultiTokenCurrencyExtended<T::AccountId> for MultiTokenCurrencyAdapter<T>
where
	T: Trait,
{
	fn create(address: &T::AccountId, amount: T::Balance) -> T::CurrencyId {
		let token_id = <NextCurrencyId<T>>::get();
		NextCurrencyId::<T>::mutate(|id| *id += One::one());
		// we are creating new token so amount can not be overflowed as its always true
		// 0 + amount < T::Balance::max_value()
		let _ = <Self as MultiTokenCurrency<T::AccountId>>::deposit_creating(token_id, address, amount);
		token_id
	}

	fn mint(currency_id: T::CurrencyId, address: &T::AccountId, amount: T::Balance) -> DispatchResult {
		if !Self::exists(currency_id) {
			return Err(DispatchError::from(Error::<T>::TokenIdNotExists));
		}
		let current_balance = <Self as MultiTokenCurrency<T::AccountId>>::total_balance(currency_id, address);
		// check for overflow while minting
		current_balance
			.checked_add(&amount)
			.ok_or(Error::<T>::BalanceOverflow)?;

		let _ = <Self as MultiTokenCurrency<T::AccountId>>::deposit_creating(currency_id, address, amount);
		Ok(())
	}

	fn get_next_currency_id() -> Self::CurrencyId {
		<Module<T>>::next_asset_id()
	}

	fn exists(currency_id: Self::CurrencyId) -> bool {
		<TotalIssuance<T>>::contains_key(currency_id)
	}

	/// either succeeds or leaves state unchanged
	fn burn_and_settle(currency_id: T::CurrencyId, who: &T::AccountId, amount: T::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		Module::<T>::ensure_can_withdraw(currency_id, who, amount)?;
		<TotalIssuance<T>>::mutate(currency_id, |v| *v -= amount);
		Module::<T>::set_free_balance(currency_id, who, Self::free_balance(currency_id, who) - amount);
		Ok(())
	}
}
