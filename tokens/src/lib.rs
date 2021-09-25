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
#![allow(clippy::unused_unit)]
#![allow(clippy::comparison_chain)]

pub use crate::imbalances::{NegativeImbalance, PositiveImbalance};

use codec::MaxEncodedLen;
use frame_support::{
	ensure, log,
	pallet_prelude::*,
	traits::{
		tokens::{fungible, fungibles, DepositConsequence, WithdrawConsequence},
		BalanceStatus as Status, Contains, Currency as PalletCurrency, ExistenceRequirement, Get, Imbalance,
		LockableCurrency as PalletLockableCurrency, ReservableCurrency as PalletReservableCurrency, SignedImbalance,
		WithdrawReasons,
	},
	transactional, BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, Saturating,
		StaticLookup, Zero,
	},
	ArithmeticError, DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::{
	convert::{Infallible, TryFrom, TryInto},
	marker,
	prelude::*,
	vec::Vec,
};

use orml_traits::{
	arithmetic::{self, Signed},
	currency::TransferAll,
	BalanceStatus, GetByKey, LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
	MultiReservableCurrency, OnDust,
};

mod imbalances;
mod impls;
mod mock;
mod tests;
mod weights;

pub use impls::*;
pub use weights::WeightInfo;

pub struct TransferDust<T, GetAccountId>(marker::PhantomData<(T, GetAccountId)>);
impl<T, GetAccountId> OnDust<T::AccountId, T::CurrencyId, T::Balance> for TransferDust<T, GetAccountId>
where
	T: Config,
	GetAccountId: Get<T::AccountId>,
{
	fn on_dust(who: &T::AccountId, currency_id: T::CurrencyId, amount: T::Balance) {
		// transfer the dust to treasury account, ignore the result,
		// if failed will leave some dust which still could be recycled.
		let _ = Pallet::<T>::do_transfer(
			currency_id,
			who,
			&GetAccountId::get(),
			amount,
			ExistenceRequirement::AllowDeath,
		);
	}
}

pub struct BurnDust<T>(marker::PhantomData<T>);
impl<T: Config> OnDust<T::AccountId, T::CurrencyId, T::Balance> for BurnDust<T> {
	fn on_dust(who: &T::AccountId, currency_id: T::CurrencyId, amount: T::Balance) {
		// burn the dust, ignore the result,
		// if failed will leave some dust which still could be recycled.
		let _ = Pallet::<T>::do_withdraw(currency_id, who, amount, ExistenceRequirement::AllowDeath, true);
	}
}

/// A single lock on a balance. There can be many of these on an account and
/// they "overlap", so the same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug)]
pub struct BalanceLock<Balance> {
	/// An identifier for this lock. Only one lock may be in existence for
	/// each identifier.
	pub id: LockIdentifier,
	/// The amount which the free balance may not drop below when this lock
	/// is in effect.
	pub amount: Balance,
}

/// balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, MaxEncodedLen, RuntimeDebug)]
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
	/// This balance is a 'reserve' balance that other subsystems use in
	/// order to set aside tokens that are still 'owned' by the account
	/// holder, but which are suspendable.
	pub reserved: Balance,
	/// The amount that `free` may not drop below when withdrawing.
	pub frozen: Balance,
}

impl<Balance: Saturating + Copy + Ord> AccountData<Balance> {
	/// The amount that this account's free balance may not be reduced
	/// beyond.
	pub(crate) fn frozen(&self) -> Balance {
		self.frozen
	}
	/// The total balance in this account including any that is reserved and
	/// ignoring any frozen.
	fn total(&self) -> Balance {
		self.free.saturating_add(self.reserved)
	}
}

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The balance type
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// The amount type, should be signed version of `Balance`
		type Amount: Signed
			+ TryInto<Self::Balance>
			+ TryFrom<Self::Balance>
			+ Parameter
			+ Member
			+ arithmetic::SimpleArithmetic
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize;

		/// The currency ID type
		type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The minimum amount required to keep an account.
		/// It's deprecated to config 0 as ED for any currency_id,
		/// zero ED will retain account even if its total is zero.
		/// Since accounts of orml_tokens are also used as providers of
		/// System::AccountInfo, zero ED may cause some problems.
		type ExistentialDeposits: GetByKey<Self::CurrencyId, Self::Balance>;

		/// Handler to burn or transfer account's dust
		type OnDust: OnDust<Self::AccountId, Self::CurrencyId, Self::Balance>;

		#[pallet::constant]
		type MaxLocks: Get<u32>;

		// The whitelist of accounts that will not be reaped even if its total
		// is zero or below ED.
		type DustRemovalWhitelist: Contains<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The balance is too low
		BalanceTooLow,
		/// Cannot convert Amount into Balance type
		AmountIntoBalanceFailed,
		/// Failed because liquidity restrictions due to locking
		LiquidityRestrictions,
		/// Failed because the maximum locks was exceeded
		MaxLocksExceeded,
		/// Transfer/payment would kill account
		KeepAlive,
		/// Value too low to create account due to existential deposit
		ExistentialDeposit,
		/// Beneficiary account must pre-exist
		DeadAccount,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	#[pallet::metadata(T::CurrencyId = "CurrencyId", T::AccountId = "AccountId", T::Balance = "Balance")]
	pub enum Event<T: Config> {
		/// An account was created with some free balance. \[currency_id,
		/// account, free_balance\]
		Endowed(T::CurrencyId, T::AccountId, T::Balance),
		/// An account was removed whose balance was non-zero but below
		/// ExistentialDeposit, resulting in an outright loss. \[currency_id,
		/// account, balance\]
		DustLost(T::CurrencyId, T::AccountId, T::Balance),
		/// Transfer succeeded. \[currency_id, from, to, value\]
		Transfer(T::CurrencyId, T::AccountId, T::AccountId, T::Balance),
		/// Some balance was reserved (moved from free to reserved).
		/// \[currency_id, who, value\]
		Reserved(T::CurrencyId, T::AccountId, T::Balance),
		/// Some balance was unreserved (moved from reserved to free).
		/// \[currency_id, who, value\]
		Unreserved(T::CurrencyId, T::AccountId, T::Balance),
		/// A balance was set by root. \[who, free, reserved\]
		BalanceSet(T::CurrencyId, T::AccountId, T::Balance, T::Balance),
	}

	/// The total issuance of a token type.
	#[pallet::storage]
	#[pallet::getter(fn total_issuance)]
	pub type TotalIssuance<T: Config> = StorageMap<_, Twox64Concat, T::CurrencyId, T::Balance, ValueQuery>;

	/// Any liquidity locks of a token type under an account.
	/// NOTE: Should only be accessed when setting, changing and freeing a lock.
	#[pallet::storage]
	#[pallet::getter(fn locks)]
	pub type Locks<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		T::CurrencyId,
		BoundedVec<BalanceLock<T::Balance>, T::MaxLocks>,
		ValueQuery,
	>;

	/// The balance of a token type under an account.
	///
	/// NOTE: If the total is ever zero, decrease account ref account.
	///
	/// NOTE: This is only used in the case that this module is used to store
	/// balances.
	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub type Accounts<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		T::CurrencyId,
		AccountData<T::Balance>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub balances: Vec<(T::AccountId, T::CurrencyId, T::Balance)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { balances: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// ensure no duplicates exist.
			let unique_endowed_accounts = self
				.balances
				.iter()
				.map(|(account_id, currency_id, _)| (account_id, currency_id))
				.collect::<std::collections::BTreeSet<_>>();
			assert!(
				unique_endowed_accounts.len() == self.balances.len(),
				"duplicate endowed accounts in genesis."
			);

			self.balances
				.iter()
				.for_each(|(account_id, currency_id, initial_balance)| {
					assert!(
						*initial_balance >= T::ExistentialDeposits::get(currency_id),
						"the balance of any account should always be more than existential deposit.",
					);
					Pallet::<T>::mutate_account(account_id, *currency_id, |account_data, _| {
						account_data.free = *initial_balance
					});
					TotalIssuance::<T>::mutate(*currency_id, |total_issuance| {
						*total_issuance = total_issuance
							.checked_add(initial_balance)
							.expect("total issuance cannot overflow when building genesis")
					});
				});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer some liquid free balance to another account.
		///
		/// `transfer` will set the `FreeBalance` of the sender and receiver.
		/// It will decrease the total issuance of the system by the
		/// `TransferFee`. If the sender's account is below the existential
		/// deposit as a result of the transfer, the account will be reaped.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		///
		/// - `dest`: The recipient of the transfer.
		/// - `currency_id`: currency type.
		/// - `amount`: free balance amount to tranfer.
		#[pallet::weight(T::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			#[pallet::compact] amount: T::Balance,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			Self::do_transfer(currency_id, &from, &to, amount, ExistenceRequirement::AllowDeath)?;

			Self::deposit_event(Event::Transfer(currency_id, from, to, amount));
			Ok(())
		}

		/// Transfer all remaining balance to the given account.
		///
		/// NOTE: This function only attempts to transfer _transferable_
		/// balances. This means that any locked, reserved, or existential
		/// deposits (when `keep_alive` is `true`), will not be transferred by
		/// this function. To ensure that this function results in a killed
		/// account, you might need to prepare the account by removing any
		/// reference counters, storage deposits, etc...
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		///
		/// - `dest`: The recipient of the transfer.
		/// - `currency_id`: currency type.
		/// - `keep_alive`: A boolean to determine if the `transfer_all`
		///   operation should send all of the funds the account has, causing
		///   the sender account to be killed (false), or transfer everything
		///   except at least the existential deposit, which will guarantee to
		///   keep the sender account alive (true).
		#[pallet::weight(T::WeightInfo::transfer_all())]
		pub fn transfer_all(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			keep_alive: bool,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			let reducible_balance =
				<Self as fungibles::Inspect<T::AccountId>>::reducible_balance(currency_id, &from, keep_alive);
			<Self as fungibles::Transfer<_>>::transfer(currency_id, &from, &to, reducible_balance, keep_alive)?;

			Self::deposit_event(Event::Transfer(currency_id, from, to, reducible_balance));
			Ok(())
		}

		/// Same as the [`transfer`] call, but with a check that the transfer
		/// will not kill the origin account.
		///
		/// 99% of the time you want [`transfer`] instead.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		///
		/// - `dest`: The recipient of the transfer.
		/// - `currency_id`: currency type.
		/// - `amount`: free balance amount to tranfer.
		#[pallet::weight(T::WeightInfo::transfer_keep_alive())]
		pub fn transfer_keep_alive(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			#[pallet::compact] amount: T::Balance,
		) -> DispatchResultWithPostInfo {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			Self::do_transfer(currency_id, &from, &to, amount, ExistenceRequirement::KeepAlive)?;

			Self::deposit_event(Event::Transfer(currency_id, from, to, amount));
			Ok(().into())
		}

		/// Exactly as `transfer`, except the origin must be root and the source
		/// account may be specified.
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// - `source`: The sender of the transfer.
		/// - `dest`: The recipient of the transfer.
		/// - `currency_id`: currency type.
		/// - `amount`: free balance amount to tranfer.
		#[pallet::weight(T::WeightInfo::force_transfer())]
		pub fn force_transfer(
			origin: OriginFor<T>,
			source: <T::Lookup as StaticLookup>::Source,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			#[pallet::compact] amount: T::Balance,
		) -> DispatchResult {
			ensure_root(origin)?;
			let from = T::Lookup::lookup(source)?;
			let to = T::Lookup::lookup(dest)?;
			Self::do_transfer(currency_id, &from, &to, amount, ExistenceRequirement::AllowDeath)?;

			Self::deposit_event(Event::Transfer(currency_id, from, to, amount));
			Ok(())
		}

		/// Set the balances of a given account.
		///
		/// This will alter `FreeBalance` and `ReservedBalance` in storage. it
		/// will also decrease the total issuance of the system
		/// (`TotalIssuance`). If the new free or reserved balance is below the
		/// existential deposit, it will reap the `AccountInfo`.
		///
		/// The dispatch origin for this call is `root`.
		#[pallet::weight(T::WeightInfo::set_balance())]
		pub fn set_balance(
			origin: OriginFor<T>,
			who: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			#[pallet::compact] new_free: T::Balance,
			#[pallet::compact] new_reserved: T::Balance,
		) -> DispatchResult {
			ensure_root(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::try_mutate_account(&who, currency_id, |account, _| -> DispatchResult {
				let mut new_total = new_free.checked_add(&new_reserved).ok_or(ArithmeticError::Overflow)?;
				let (new_free, new_reserved) = if new_free + new_reserved < T::ExistentialDeposits::get(&currency_id) {
					new_total = Zero::zero();
					(Zero::zero(), Zero::zero())
				} else {
					(new_free, new_reserved)
				};
				let old_total = account.total();

				account.free = new_free;
				account.reserved = new_reserved;

				if new_total > old_total {
					TotalIssuance::<T>::try_mutate(currency_id, |t| -> DispatchResult {
						*t = t
							.checked_add(&(new_total - old_total))
							.ok_or(ArithmeticError::Overflow)?;
						Ok(())
					})?;
				} else if new_total < old_total {
					TotalIssuance::<T>::try_mutate(currency_id, |t| -> DispatchResult {
						*t = t
							.checked_sub(&(old_total - new_total))
							.ok_or(ArithmeticError::Underflow)?;
						Ok(())
					})?;
				}

				Self::deposit_event(Event::BalanceSet(currency_id, who.clone(), new_free, new_reserved));
				Ok(())
			})
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn deposit_consequence(
		_who: &T::AccountId,
		currency_id: T::CurrencyId,
		amount: T::Balance,
		account: &AccountData<T::Balance>,
	) -> DepositConsequence {
		if amount.is_zero() {
			return DepositConsequence::Success;
		}

		if TotalIssuance::<T>::get(currency_id).checked_add(&amount).is_none() {
			return DepositConsequence::Overflow;
		}

		let new_total_balance = match account.total().checked_add(&amount) {
			Some(x) => x,
			None => return DepositConsequence::Overflow,
		};

		if new_total_balance < T::ExistentialDeposits::get(&currency_id) {
			return DepositConsequence::BelowMinimum;
		}

		// NOTE: We assume that we are a provider, so don't need to do any checks in the
		// case of account creation.

		DepositConsequence::Success
	}

	pub(crate) fn withdraw_consequence(
		who: &T::AccountId,
		currency_id: T::CurrencyId,
		amount: T::Balance,
		account: &AccountData<T::Balance>,
	) -> WithdrawConsequence<T::Balance> {
		if amount.is_zero() {
			return WithdrawConsequence::Success;
		}

		if TotalIssuance::<T>::get(currency_id).checked_sub(&amount).is_none() {
			return WithdrawConsequence::Underflow;
		}

		let new_total_balance = match account.total().checked_sub(&amount) {
			Some(x) => x,
			None => return WithdrawConsequence::NoFunds,
		};

		// Provider restriction - total account balance cannot be reduced to zero if it
		// cannot sustain the loss of a provider reference.
		// NOTE: This assumes that the pallet is a provider (which is true). Is this
		// ever changes, then this will need to adapt accordingly.
		let ed = T::ExistentialDeposits::get(&currency_id);
		let success = if new_total_balance < ed {
			if frame_system::Pallet::<T>::can_dec_provider(who) {
				WithdrawConsequence::ReducedToZero(new_total_balance)
			} else {
				return WithdrawConsequence::WouldDie;
			}
		} else {
			WithdrawConsequence::Success
		};

		// Enough free funds to have them be reduced.
		let new_free_balance = match account.free.checked_sub(&amount) {
			Some(b) => b,
			None => return WithdrawConsequence::NoFunds,
		};

		// Eventual free funds must be no less than the frozen balance.
		if new_free_balance < account.frozen() {
			return WithdrawConsequence::Frozen;
		}

		success
	}

	// Ensure that an account can withdraw from their free balance given any
	// existing withdrawal restrictions like locks and vesting balance.
	// Is a no-op if amount to be withdrawn is zero.
	pub(crate) fn ensure_can_withdraw(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
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

	pub(crate) fn try_mutate_account<R, E>(
		who: &T::AccountId,
		currency_id: T::CurrencyId,
		f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> sp_std::result::Result<R, E>,
	) -> sp_std::result::Result<R, E> {
		Accounts::<T>::try_mutate_exists(who, currency_id, |maybe_account| {
			let existed = maybe_account.is_some();
			let mut account = maybe_account.take().unwrap_or_default();
			f(&mut account, existed).map(move |result| {
				let maybe_endowed = if !existed { Some(account.free) } else { None };
				let mut maybe_dust: Option<T::Balance> = None;
				let total = account.total();
				*maybe_account = if total < T::ExistentialDeposits::get(&currency_id) {
					// if ED is not zero, but account total is zero, account will be reaped
					if total.is_zero() {
						None
					} else {
						if !T::DustRemovalWhitelist::contains(who) {
							maybe_dust = Some(total);
						}
						Some(account)
					}
				} else {
					// Note: if ED is zero, account will never be reaped
					Some(account)
				};

				(maybe_endowed, existed, maybe_account.is_some(), maybe_dust, result)
			})
		})
		.map(|(maybe_endowed, existed, exists, maybe_dust, result)| {
			if existed && !exists {
				// If existed before, decrease account provider.
				// Ignore the result, because if it failed then there are remaining consumers,
				// and the account storage in frame_system shouldn't be reaped.
				let _ = frame_system::Pallet::<T>::dec_providers(who);
			} else if !existed && exists {
				// if new, increase account provider
				frame_system::Pallet::<T>::inc_providers(who);
			}

			if let Some(endowed) = maybe_endowed {
				Self::deposit_event(Event::Endowed(currency_id, who.clone(), endowed));
			}

			if let Some(dust_amount) = maybe_dust {
				// `OnDust` maybe get/set storage `Accounts` of `who`, trigger handler here
				// to avoid some unexpected errors.
				T::OnDust::on_dust(who, currency_id, dust_amount);
				Self::deposit_event(Event::DustLost(currency_id, who.clone(), dust_amount));
			}

			result
		})
	}

	pub(crate) fn mutate_account<R>(
		who: &T::AccountId,
		currency_id: T::CurrencyId,
		f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> R,
	) -> R {
		Self::try_mutate_account(who, currency_id, |account, existed| -> Result<R, Infallible> {
			Ok(f(account, existed))
		})
		.expect("Error is infallible; qed")
	}

	/// Set free balance of `who` to a new value.
	///
	/// Note: this will not maintain total issuance, and the caller is expected
	/// to do it. If it will cause the account to be removed dust, shouldn't use
	/// it, because maybe the account that should be reaped to remain due to
	/// failed transfer/withdraw dust.
	pub(crate) fn set_free_balance(currency_id: T::CurrencyId, who: &T::AccountId, amount: T::Balance) {
		Self::mutate_account(who, currency_id, |account, _| {
			account.free = amount;
		});
	}

	/// Set reserved balance of `who` to a new value.
	///
	/// Note: this will not maintain total issuance, and the caller is expected
	/// to do it. If it will cause the account to be removed dust, shouldn't use
	/// it, because maybe the account that should be reaped to remain due to
	/// failed transfer/withdraw dust.
	pub(crate) fn set_reserved_balance(currency_id: T::CurrencyId, who: &T::AccountId, amount: T::Balance) {
		Self::mutate_account(who, currency_id, |account, _| {
			account.reserved = amount;
		});
	}

	/// Update the account entry for `who` under `currency_id`, given the
	/// locks.
	pub(crate) fn update_locks(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		locks: &[BalanceLock<T::Balance>],
	) -> DispatchResult {
		// update account data
		Self::mutate_account(who, currency_id, |account, _| {
			account.frozen = Zero::zero();
			for lock in locks.iter() {
				account.frozen = account.frozen.max(lock.amount);
			}
		});

		// update locks
		let existed = Locks::<T>::contains_key(who, currency_id);
		if locks.is_empty() {
			Locks::<T>::remove(who, currency_id);
			if existed {
				// decrease account ref count when destruct lock
				frame_system::Pallet::<T>::dec_consumers(who);
			}
		} else {
			let bounded_locks: BoundedVec<BalanceLock<T::Balance>, T::MaxLocks> =
				locks.to_vec().try_into().map_err(|_| Error::<T>::MaxLocksExceeded)?;
			Locks::<T>::insert(who, currency_id, bounded_locks);
			if !existed {
				// increase account ref count when initialize lock
				if frame_system::Pallet::<T>::inc_consumers(who).is_err() {
					// No providers for the locks. This is impossible under normal circumstances
					// since the funds that are under the lock will themselves be stored in the
					// account and therefore will need a reference.
					log::warn!(
						"Warning: Attempt to introduce lock consumer reference, yet no providers. \
						This is unexpected but should be safe."
					);
				}
			}
		}

		Ok(())
	}

	/// Transfer some free balance from `from` to `to`. Ensure from_account
	/// allow death or new balance will not be reaped, and ensure
	/// to_account will not be removed dust.
	///
	/// Is a no-op if value to be transferred is zero or the `from` is the same
	/// as `to`.
	pub(crate) fn do_transfer(
		currency_id: T::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: T::Balance,
		existence_requirement: ExistenceRequirement,
	) -> DispatchResult {
		if amount.is_zero() || from == to {
			return Ok(());
		}

		Self::try_mutate_account(to, currency_id, |to_account, _existed| -> DispatchResult {
			Self::try_mutate_account(from, currency_id, |from_account, _existed| -> DispatchResult {
				from_account.free = from_account
					.free
					.checked_sub(&amount)
					.ok_or(Error::<T>::BalanceTooLow)?;
				to_account.free = to_account.free.checked_add(&amount).ok_or(ArithmeticError::Overflow)?;

				let ed = T::ExistentialDeposits::get(&currency_id);
				// if the total of `to_account` is below existential deposit, would return an
				// error.
				// Note: if `to_account` is in `T::DustRemovalWhitelist`, can bypass this check.
				ensure!(
					to_account.total() >= ed || T::DustRemovalWhitelist::contains(to),
					Error::<T>::ExistentialDeposit
				);

				Self::ensure_can_withdraw(currency_id, from, amount)?;

				let allow_death = existence_requirement == ExistenceRequirement::AllowDeath;
				let allow_death = allow_death && frame_system::Pallet::<T>::can_dec_provider(from);
				let would_be_dead = if from_account.total() < ed {
					if from_account.total().is_zero() {
						true
					} else {
						// Note: if account is not in `T::DustRemovalWhitelist`, account will eventually
						// be reaped due to the dust removal.
						!T::DustRemovalWhitelist::contains(from)
					}
				} else {
					false
				};

				ensure!(allow_death || !would_be_dead, Error::<T>::KeepAlive);

				Ok(())
			})?;
			Ok(())
		})
	}

	/// Withdraw some free balance from an account, respecting existence
	/// requirements.
	///
	/// `change_total_issuance`:
	/// - true, decrease the total issuance by burned amount.
	/// - false, do not update the total issuance.
	///
	/// Is a no-op if value to be withdrawn is zero.
	pub(crate) fn do_withdraw(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		amount: T::Balance,
		existence_requirement: ExistenceRequirement,
		change_total_issuance: bool,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}

		Self::try_mutate_account(who, currency_id, |account, _existed| -> DispatchResult {
			Self::ensure_can_withdraw(currency_id, who, amount)?;
			let previous_total = account.total();
			account.free -= amount;

			let ed = T::ExistentialDeposits::get(&currency_id);
			let would_be_dead = if account.total() < ed {
				if account.total().is_zero() {
					true
				} else {
					// Note: if account is not in `T::DustRemovalWhitelist`, account will eventually
					// be reaped due to the dust removal.
					!T::DustRemovalWhitelist::contains(who)
				}
			} else {
				false
			};
			let would_kill = would_be_dead && (previous_total >= ed || !previous_total.is_zero());
			ensure!(
				existence_requirement == ExistenceRequirement::AllowDeath || !would_kill,
				Error::<T>::KeepAlive
			);

			if change_total_issuance {
				TotalIssuance::<T>::mutate(currency_id, |v| *v -= amount);
			}

			Ok(())
		})
	}

	/// Deposit some `value` into the free balance of `who`.
	///
	/// `require_existed`:
	/// - true, the account must already exist, do not require ED.
	/// - false, possibly creating a new account, require ED if the account does
	///   not yet exist, but except this account is in the dust removal
	///   whitelist.
	///
	/// `change_total_issuance`:
	/// - true, increase the issued amount to total issuance.
	/// - false, do not update the total issuance.
	pub(crate) fn do_deposit(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		amount: T::Balance,
		require_existed: bool,
		change_total_issuance: bool,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}

		Self::try_mutate_account(who, currency_id, |account, existed| -> DispatchResult {
			if require_existed {
				ensure!(existed, Error::<T>::DeadAccount);
			} else {
				let ed = T::ExistentialDeposits::get(&currency_id);
				// Note: if who is in dust removal whitelist, allow to deposit the amount that
				// below ED to it.
				ensure!(
					amount >= ed || existed || T::DustRemovalWhitelist::contains(who),
					Error::<T>::ExistentialDeposit
				);
			}

			let new_total_issuance = Self::total_issuance(currency_id)
				.checked_add(&amount)
				.ok_or(ArithmeticError::Overflow)?;
			if change_total_issuance {
				TotalIssuance::<T>::mutate(currency_id, |v| *v = new_total_issuance);
			}
			account.free += amount;

			Ok(())
		})
	}
}

impl<T: Config> MultiCurrency<T::AccountId> for Pallet<T> {
	type CurrencyId = T::CurrencyId;
	type Balance = T::Balance;

	fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
		T::ExistentialDeposits::get(&currency_id)
	}

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		Self::total_issuance(currency_id)
	}

	fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, currency_id).total()
	}

	fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, currency_id).free
	}

	fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		Self::ensure_can_withdraw(currency_id, who, amount)
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		// allow death
		Self::do_transfer(currency_id, from, to, amount, ExistenceRequirement::AllowDeath)
	}

	fn deposit(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		// do not require existing
		Self::do_deposit(currency_id, who, amount, false, true)
	}

	fn withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		// allow death
		Self::do_withdraw(currency_id, who, amount, ExistenceRequirement::AllowDeath, true)
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
	/// NOTE: `slash()` prefers free balance, but assumes that reserve
	/// balance can be drawn from in extreme circumstances. `can_slash()`
	/// should be used prior to `slash()` to avoid having to draw from
	/// reserved funds, however we err on the side of punishment if things
	/// are inconsistent or `can_slash` wasn't used appropriately.
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
		TotalIssuance::<T>::mutate(currency_id, |v| *v -= amount - remaining_slash);
		remaining_slash
	}
}

impl<T: Config> MultiCurrencyExtended<T::AccountId> for Pallet<T> {
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

impl<T: Config> MultiLockableCurrency<T::AccountId> for Pallet<T> {
	type Moment = T::BlockNumber;

	// Set a lock on the balance of `who` under `currency_id`.
	// Is a no-op if lock amount is zero.
	fn set_lock(
		lock_id: LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
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
		Self::update_locks(currency_id, who, &locks[..])
	}

	// Extend a lock on the balance of `who` under `currency_id`.
	// Is a no-op if lock amount is zero
	fn extend_lock(
		lock_id: LockIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
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
		Self::update_locks(currency_id, who, &locks[..])
	}

	fn remove_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId) -> DispatchResult {
		let mut locks = Self::locks(who, currency_id);
		locks.retain(|lock| lock.id != lock_id);
		let locks_vec = locks.to_vec();
		Self::update_locks(currency_id, who, &locks_vec[..])
	}
}

impl<T: Config> MultiReservableCurrency<T::AccountId> for Pallet<T> {
	/// Check if `who` can reserve `value` from their free balance.
	///
	/// Always `true` if value to be reserved is zero.
	fn can_reserve(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> bool {
		if value.is_zero() {
			return true;
		}
		Self::ensure_can_withdraw(currency_id, who, value).is_ok()
	}

	/// Slash from reserved balance, returning any amount that was unable to
	/// be slashed.
	///
	/// Is a no-op if the value to be slashed is zero.
	fn slash_reserved(currency_id: Self::CurrencyId, who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		if value.is_zero() {
			return value;
		}

		let reserved_balance = Self::reserved_balance(currency_id, who);
		let actual = reserved_balance.min(value);
		Self::set_reserved_balance(currency_id, who, reserved_balance - actual);
		TotalIssuance::<T>::mutate(currency_id, |v| *v -= actual);
		value - actual
	}

	fn reserved_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, currency_id).reserved
	}

	/// Move `value` from the free balance from `who` to their reserved
	/// balance.
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

		Self::deposit_event(Event::Reserved(currency_id, who.clone(), value));
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

		Self::deposit_event(Event::Unreserved(currency_id, who.clone(), actual));
		value - actual
	}

	/// Move the reserved balance of one account into the balance of
	/// another, according to `status`.
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
	) -> sp_std::result::Result<Self::Balance, DispatchError> {
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
			}
			BalanceStatus::Reserved => {
				Self::set_reserved_balance(currency_id, beneficiary, to_account.reserved + actual);
			}
		}
		Self::set_reserved_balance(currency_id, slashed, from_account.reserved - actual);
		Ok(value - actual)
	}
}

impl<T: Config> fungibles::Inspect<T::AccountId> for Pallet<T> {
	type AssetId = T::CurrencyId;
	type Balance = T::Balance;

	fn total_issuance(asset_id: Self::AssetId) -> Self::Balance {
		Self::total_issuance(asset_id)
	}

	fn minimum_balance(asset_id: Self::AssetId) -> Self::Balance {
		T::ExistentialDeposits::get(&asset_id)
	}

	fn balance(asset_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, asset_id).total()
	}

	fn reducible_balance(asset_id: Self::AssetId, who: &T::AccountId, keep_alive: bool) -> Self::Balance {
		let a = Self::accounts(who, asset_id);
		// Liquid balance is what is neither reserved nor locked/frozen.
		let liquid = a.free.saturating_sub(a.frozen);
		if frame_system::Pallet::<T>::can_dec_provider(who) && !keep_alive {
			liquid
		} else {
			// `must_remain_to_exist` is the part of liquid balance which must remain to
			// keep total over ED.
			let must_remain_to_exist = T::ExistentialDeposits::get(&asset_id).saturating_sub(a.total() - liquid);
			liquid.saturating_sub(must_remain_to_exist)
		}
	}

	fn can_deposit(asset_id: Self::AssetId, who: &T::AccountId, amount: Self::Balance) -> DepositConsequence {
		Self::deposit_consequence(who, asset_id, amount, &Self::accounts(who, asset_id))
	}

	fn can_withdraw(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		Self::withdraw_consequence(who, asset_id, amount, &Self::accounts(who, asset_id))
	}
}

impl<T: Config> fungibles::Mutate<T::AccountId> for Pallet<T> {
	fn mint_into(asset_id: Self::AssetId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		Self::deposit_consequence(who, asset_id, amount, &Self::accounts(who, asset_id)).into_result()?;
		// do not require existing
		Self::do_deposit(asset_id, who, amount, false, true)
	}

	fn burn_from(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		let extra = Self::withdraw_consequence(who, asset_id, amount, &Self::accounts(who, asset_id)).into_result()?;
		let actual = amount + extra;
		// allow death
		Self::do_withdraw(asset_id, who, actual, ExistenceRequirement::AllowDeath, true).map(|_| actual)
	}
}

impl<T: Config> fungibles::Transfer<T::AccountId> for Pallet<T> {
	fn transfer(
		asset_id: Self::AssetId,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: T::Balance,
		keep_alive: bool,
	) -> Result<T::Balance, DispatchError> {
		let existence_requirement = if keep_alive {
			ExistenceRequirement::KeepAlive
		} else {
			ExistenceRequirement::AllowDeath
		};
		Self::do_transfer(asset_id, source, dest, amount, existence_requirement).map(|_| amount)
	}
}

impl<T: Config> fungibles::Unbalanced<T::AccountId> for Pallet<T> {
	fn set_balance(asset_id: Self::AssetId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		// Balance is the same type and will not overflow
		Self::mutate_account(who, asset_id, |account, _| account.free = amount);
		Ok(())
	}

	fn set_total_issuance(asset_id: Self::AssetId, amount: Self::Balance) {
		// Balance is the same type and will not overflow
		TotalIssuance::<T>::mutate(asset_id, |t| *t = amount);
	}
}

impl<T: Config> fungibles::InspectHold<T::AccountId> for Pallet<T> {
	fn balance_on_hold(asset_id: Self::AssetId, who: &T::AccountId) -> T::Balance {
		Self::accounts(who, asset_id).reserved
	}

	fn can_hold(asset_id: Self::AssetId, who: &T::AccountId, amount: T::Balance) -> bool {
		let a = Self::accounts(who, asset_id);
		let min_balance = T::ExistentialDeposits::get(&asset_id).max(a.frozen);
		if a.reserved.checked_add(&amount).is_none() {
			return false;
		}
		// We require it to be min_balance + amount to ensure that the full reserved
		// funds may be slashed without compromising locked funds or destroying the
		// account.
		let required_free = match min_balance.checked_add(&amount) {
			Some(x) => x,
			None => return false,
		};
		a.free >= required_free
	}
}

impl<T: Config> fungibles::MutateHold<T::AccountId> for Pallet<T> {
	fn hold(asset_id: Self::AssetId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		ensure!(Self::can_reserve(asset_id, who, amount), Error::<T>::BalanceTooLow);
		Self::mutate_account(who, asset_id, |a, _| {
			// `can_reserve` has did underflow checking
			a.free -= amount;
			// Cannot overflow as `amount` is from `a.free`
			a.reserved += amount;
		});
		Ok(())
	}

	fn release(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
		best_effort: bool,
	) -> Result<T::Balance, DispatchError> {
		if amount.is_zero() {
			return Ok(amount);
		}
		// Done on a best-effort basis.
		Self::try_mutate_account(who, asset_id, |a, _existed| {
			let new_free = a.free.saturating_add(amount.min(a.reserved));
			let actual = new_free - a.free;
			// Guaranteed to be <= amount and <= a.reserved
			ensure!(best_effort || actual == amount, Error::<T>::BalanceTooLow);
			a.free = new_free;
			a.reserved = a.reserved.saturating_sub(actual);
			Ok(actual)
		})
	}

	fn transfer_held(
		asset_id: Self::AssetId,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		_best_effort: bool,
		on_hold: bool,
	) -> Result<Self::Balance, DispatchError> {
		let status = if on_hold { Status::Reserved } else { Status::Free };
		Self::repatriate_reserved(asset_id, source, dest, amount, status)
	}
}

pub struct CurrencyAdapter<T, GetCurrencyId>(marker::PhantomData<(T, GetCurrencyId)>);

impl<T, GetCurrencyId> PalletCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type Balance = T::Balance;
	type PositiveImbalance = PositiveImbalance<T, GetCurrencyId>;
	type NegativeImbalance = NegativeImbalance<T, GetCurrencyId>;

	fn total_balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as MultiCurrency<_>>::total_balance(GetCurrencyId::get(), who)
	}

	fn can_slash(who: &T::AccountId, value: Self::Balance) -> bool {
		<Pallet<T> as MultiCurrency<_>>::can_slash(GetCurrencyId::get(), who, value)
	}

	fn total_issuance() -> Self::Balance {
		<Pallet<T> as MultiCurrency<_>>::total_issuance(GetCurrencyId::get())
	}

	fn minimum_balance() -> Self::Balance {
		<Pallet<T> as MultiCurrency<_>>::minimum_balance(GetCurrencyId::get())
	}

	fn burn(mut amount: Self::Balance) -> Self::PositiveImbalance {
		if amount.is_zero() {
			return PositiveImbalance::zero();
		}
		TotalIssuance::<T>::mutate(GetCurrencyId::get(), |issued| {
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
		TotalIssuance::<T>::mutate(GetCurrencyId::get(), |issued| {
			*issued = issued.checked_add(&amount).unwrap_or_else(|| {
				amount = Self::Balance::max_value() - *issued;
				Self::Balance::max_value()
			})
		});
		NegativeImbalance::new(amount)
	}

	fn free_balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as MultiCurrency<_>>::free_balance(GetCurrencyId::get(), who)
	}

	fn ensure_can_withdraw(
		who: &T::AccountId,
		amount: Self::Balance,
		_reasons: WithdrawReasons,
		_new_balance: Self::Balance,
	) -> DispatchResult {
		<Pallet<T> as MultiCurrency<_>>::ensure_can_withdraw(GetCurrencyId::get(), who, amount)
	}

	fn transfer(
		source: &T::AccountId,
		dest: &T::AccountId,
		value: Self::Balance,
		existence_requirement: ExistenceRequirement,
	) -> DispatchResult {
		Pallet::<T>::do_transfer(GetCurrencyId::get(), source, dest, value, existence_requirement)
	}

	fn slash(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		if value.is_zero() {
			return (Self::NegativeImbalance::zero(), value);
		}

		let currency_id = GetCurrencyId::get();
		let account = Pallet::<T>::accounts(who, currency_id);
		let free_slashed_amount = account.free.min(value);
		let mut remaining_slash = value - free_slashed_amount;

		// slash free balance
		if !free_slashed_amount.is_zero() {
			Pallet::<T>::set_free_balance(currency_id, who, account.free - free_slashed_amount);
		}

		// slash reserved balance
		if !remaining_slash.is_zero() {
			let reserved_slashed_amount = account.reserved.min(remaining_slash);
			remaining_slash -= reserved_slashed_amount;
			Pallet::<T>::set_reserved_balance(currency_id, who, account.reserved - reserved_slashed_amount);
			(
				Self::NegativeImbalance::new(free_slashed_amount + reserved_slashed_amount),
				remaining_slash,
			)
		} else {
			(Self::NegativeImbalance::new(value), remaining_slash)
		}
	}

	/// Deposit some `value` into the free balance of an existing target account
	/// `who`.
	fn deposit_into_existing(
		who: &T::AccountId,
		value: Self::Balance,
	) -> sp_std::result::Result<Self::PositiveImbalance, DispatchError> {
		// do not change total issuance
		Pallet::<T>::do_deposit(GetCurrencyId::get(), who, value, true, false).map(|_| PositiveImbalance::new(value))
	}

	/// Deposit some `value` into the free balance of `who`, possibly creating a
	/// new account.
	fn deposit_creating(who: &T::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
		// do not change total issuance
		Pallet::<T>::do_deposit(GetCurrencyId::get(), who, value, false, false)
			.map_or_else(|_| Self::PositiveImbalance::zero(), |_| PositiveImbalance::new(value))
	}

	fn withdraw(
		who: &T::AccountId,
		value: Self::Balance,
		_reasons: WithdrawReasons,
		liveness: ExistenceRequirement,
	) -> sp_std::result::Result<Self::NegativeImbalance, DispatchError> {
		// do not change total issuance
		Pallet::<T>::do_withdraw(GetCurrencyId::get(), who, value, liveness, false)
			.map(|_| Self::NegativeImbalance::new(value))
	}

	fn make_free_balance_be(
		who: &T::AccountId,
		value: Self::Balance,
	) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
		let currency_id = GetCurrencyId::get();
		Pallet::<T>::try_mutate_account(
			who,
			currency_id,
			|account, existed| -> Result<SignedImbalance<Self::Balance, Self::PositiveImbalance>, ()> {
				// If we're attempting to set an existing account to less than ED, then
				// bypass the entire operation. It's a no-op if you follow it through, but
				// since this is an instance where we might account for a negative imbalance
				// (in the dust cleaner of set_account) before we account for its actual
				// equal and opposite cause (returned as an Imbalance), then in the
				// instance that there's no other accounts on the system at all, we might
				// underflow the issuance and our arithmetic will be off.
				let ed = T::ExistentialDeposits::get(&currency_id);
				ensure!(value.saturating_add(account.reserved) >= ed || existed, ());

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
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn can_reserve(who: &T::AccountId, value: Self::Balance) -> bool {
		<Pallet<T> as MultiReservableCurrency<_>>::can_reserve(GetCurrencyId::get(), who, value)
	}

	fn slash_reserved(who: &T::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
		let actual = <Pallet<T> as MultiReservableCurrency<_>>::slash_reserved(GetCurrencyId::get(), who, value);
		(Self::NegativeImbalance::zero(), actual)
	}

	fn reserved_balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<_>>::reserved_balance(GetCurrencyId::get(), who)
	}

	fn reserve(who: &T::AccountId, value: Self::Balance) -> DispatchResult {
		<Pallet<T> as MultiReservableCurrency<_>>::reserve(GetCurrencyId::get(), who, value)
	}

	fn unreserve(who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		<Pallet<T> as MultiReservableCurrency<_>>::unreserve(GetCurrencyId::get(), who, value)
	}

	fn repatriate_reserved(
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: Status,
	) -> sp_std::result::Result<Self::Balance, DispatchError> {
		<Pallet<T> as MultiReservableCurrency<_>>::repatriate_reserved(
			GetCurrencyId::get(),
			slashed,
			beneficiary,
			value,
			status,
		)
	}
}

impl<T, GetCurrencyId> PalletLockableCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type Moment = T::BlockNumber;
	type MaxLocks = ();

	fn set_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, _reasons: WithdrawReasons) {
		let _ = <Pallet<T> as MultiLockableCurrency<_>>::set_lock(id, GetCurrencyId::get(), who, amount);
	}

	fn extend_lock(id: LockIdentifier, who: &T::AccountId, amount: Self::Balance, _reasons: WithdrawReasons) {
		let _ = <Pallet<T> as MultiLockableCurrency<_>>::extend_lock(id, GetCurrencyId::get(), who, amount);
	}

	fn remove_lock(id: LockIdentifier, who: &T::AccountId) {
		let _ = <Pallet<T> as MultiLockableCurrency<_>>::remove_lock(id, GetCurrencyId::get(), who);
	}
}

impl<T: Config> TransferAll<T::AccountId> for Pallet<T> {
	#[transactional]
	fn transfer_all(source: &T::AccountId, dest: &T::AccountId) -> DispatchResult {
		Accounts::<T>::iter_prefix(source).try_for_each(|(currency_id, account_data)| -> DispatchResult {
			// allow death
			Self::do_transfer(
				currency_id,
				source,
				dest,
				account_data.free,
				ExistenceRequirement::AllowDeath,
			)
		})
	}
}

impl<T, GetCurrencyId> fungible::Inspect<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type Balance = T::Balance;

	fn total_issuance() -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<_>>::total_issuance(GetCurrencyId::get())
	}
	fn minimum_balance() -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<_>>::minimum_balance(GetCurrencyId::get())
	}
	fn balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<_>>::balance(GetCurrencyId::get(), who)
	}
	fn reducible_balance(who: &T::AccountId, keep_alive: bool) -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<_>>::reducible_balance(GetCurrencyId::get(), who, keep_alive)
	}
	fn can_deposit(who: &T::AccountId, amount: Self::Balance) -> DepositConsequence {
		<Pallet<T> as fungibles::Inspect<_>>::can_deposit(GetCurrencyId::get(), who, amount)
	}
	fn can_withdraw(who: &T::AccountId, amount: Self::Balance) -> WithdrawConsequence<Self::Balance> {
		<Pallet<T> as fungibles::Inspect<_>>::can_withdraw(GetCurrencyId::get(), who, amount)
	}
}

impl<T, GetCurrencyId> fungible::Mutate<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn mint_into(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T> as fungibles::Mutate<_>>::mint_into(GetCurrencyId::get(), who, amount)
	}
	fn burn_from(who: &T::AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError> {
		<Pallet<T> as fungibles::Mutate<_>>::burn_from(GetCurrencyId::get(), who, amount)
	}
}

impl<T, GetCurrencyId> fungible::Transfer<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn transfer(
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: T::Balance,
		keep_alive: bool,
	) -> Result<T::Balance, DispatchError> {
		<Pallet<T> as fungibles::Transfer<_>>::transfer(GetCurrencyId::get(), source, dest, amount, keep_alive)
	}
}

impl<T, GetCurrencyId> fungible::Unbalanced<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn set_balance(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T> as fungibles::Unbalanced<_>>::set_balance(GetCurrencyId::get(), who, amount)
	}
	fn set_total_issuance(amount: Self::Balance) {
		<Pallet<T> as fungibles::Unbalanced<_>>::set_total_issuance(GetCurrencyId::get(), amount)
	}
}

impl<T, GetCurrencyId> fungible::InspectHold<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn balance_on_hold(who: &T::AccountId) -> T::Balance {
		<Pallet<T> as fungibles::InspectHold<_>>::balance_on_hold(GetCurrencyId::get(), who)
	}
	fn can_hold(who: &T::AccountId, amount: T::Balance) -> bool {
		<Pallet<T> as fungibles::InspectHold<_>>::can_hold(GetCurrencyId::get(), who, amount)
	}
}

impl<T, GetCurrencyId> fungible::MutateHold<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn hold(who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T> as fungibles::MutateHold<_>>::hold(GetCurrencyId::get(), who, amount)
	}
	fn release(who: &T::AccountId, amount: Self::Balance, best_effort: bool) -> Result<T::Balance, DispatchError> {
		<Pallet<T> as fungibles::MutateHold<_>>::release(GetCurrencyId::get(), who, amount, best_effort)
	}
	fn transfer_held(
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		best_effort: bool,
		on_hold: bool,
	) -> Result<Self::Balance, DispatchError> {
		<Pallet<T> as fungibles::MutateHold<_>>::transfer_held(
			GetCurrencyId::get(),
			source,
			dest,
			amount,
			best_effort,
			on_hold,
		)
	}
}
