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

use codec::MaxEncodedLen;
use frame_support::{
	ensure, log,
	pallet_prelude::*,
	traits::{
		tokens::{
			fungible, fungibles, DepositConsequence, Fortitude, Precision, Preservation, Provenance,
			WithdrawConsequence,
		},
		BalanceStatus as Status, Contains, DefensiveSaturating, ExistenceRequirement, Get, Imbalance, OnUnbalanced,
	},
	BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, Saturating,
		StaticLookup, Zero,
	},
	ArithmeticError, DispatchError, DispatchResult, FixedPointOperand, RuntimeDebug, TokenError,
};
use sp_std::{cmp, convert::Infallible, marker, prelude::*, vec::Vec};

use orml_traits::{
	arithmetic::{self, Signed},
	currency::{MutationHooks, OnDeposit, OnDust, OnSlash, OnTransfer, TransferAll},
	BalanceStatus, GetByKey, Happened, LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
	MultiReservableCurrency, NamedMultiReservableCurrency,
};

mod impl_currency;
mod impl_fungibles;
mod impls;
mod mock;
mod tests;
mod tests_currency_adapter;
mod tests_events;
mod tests_fungibles;
mod tests_multicurrency;

mod weights;

pub use impl_currency::{NegativeImbalance, PositiveImbalance};
pub use impls::*;
pub use weights::WeightInfo;

/// A single lock on a balance. There can be many of these on an account and
/// they "overlap", so the same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub struct BalanceLock<Balance> {
	/// An identifier for this lock. Only one lock may be in existence for
	/// each identifier.
	pub id: LockIdentifier,
	/// The amount which the free balance may not drop below when this lock
	/// is in effect.
	pub amount: Balance,
}

/// Store named reserved balance.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ReserveData<ReserveIdentifier, Balance> {
	/// The identifier for the named reserve.
	pub id: ReserveIdentifier,
	/// The amount of the named reserve.
	pub amount: Balance,
}

/// balance information for an account.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, MaxEncodedLen, RuntimeDebug, TypeInfo)]
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
	pub(crate) fn usable(&self) -> Balance {
		self.free.saturating_sub(self.frozen)
	}

	/// The total balance in this account including any that is reserved and
	/// ignoring any frozen.
	pub(crate) fn total(&self) -> Balance {
		self.free.saturating_add(self.reserved)
	}
}

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use orml_traits::currency::MutationHooks;

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The balance type
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ FixedPointOperand;

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
			+ MaxEncodedLen;

		/// The currency ID type
		type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + TypeInfo + MaxEncodedLen;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The minimum amount required to keep an account.
		/// It's deprecated to config 0 as ED for any currency_id,
		/// zero ED will retain account even if its total is zero.
		/// Since accounts of orml_tokens are also used as providers of
		/// System::AccountInfo, zero ED may cause some problems.
		type ExistentialDeposits: GetByKey<Self::CurrencyId, Self::Balance>;

		/// Hooks are actions that are executed on certain events.
		/// For example: OnDust, OnNewTokenAccount
		type CurrencyHooks: MutationHooks<Self::AccountId, Self::CurrencyId, Self::Balance>;

		#[pallet::constant]
		type MaxLocks: Get<u32>;

		/// The maximum number of named reserves that can exist on an account.
		#[pallet::constant]
		type MaxReserves: Get<u32>;

		/// The id type for named reserves.
		type ReserveIdentifier: Parameter + Member + MaxEncodedLen + Ord + Copy;

		// The whitelist of accounts that will not be reaped even if its total
		// is zero or below ED.
		type DustRemovalWhitelist: Contains<Self::AccountId>;

		/// Handler for the unbalanced reduction when removing a dust account.
		type DustRemoval: OnUnbalanced<CreditOf<Self>>;
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
		// Number of named reserves exceed `T::MaxReserves`
		TooManyReserves,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An account was created with some free balance.
		Endowed {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// An account was removed whose balance was non-zero but below
		/// ExistentialDeposit, resulting in an outright loss.
		DustLost {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// Transfer succeeded.
		Transfer {
			currency_id: T::CurrencyId,
			from: T::AccountId,
			to: T::AccountId,
			amount: T::Balance,
		},
		/// Some balance was reserved (moved from free to reserved).
		Reserved {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// Some balance was unreserved (moved from reserved to free).
		Unreserved {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// Some reserved balance was repatriated (moved from reserved to
		/// another account).
		ReserveRepatriated {
			currency_id: T::CurrencyId,
			from: T::AccountId,
			to: T::AccountId,
			amount: T::Balance,
			status: BalanceStatus,
		},
		/// A balance was set by root.
		BalanceSet {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			free: T::Balance,
			reserved: T::Balance,
		},
		/// The total issuance of an currency has been set
		TotalIssuanceSet {
			currency_id: T::CurrencyId,
			amount: T::Balance,
		},
		/// Some balances were withdrawn (e.g. pay for transaction fee)
		Withdrawn {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// Some balances were slashed (e.g. due to mis-behavior)
		Slashed {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			free_amount: T::Balance,
			reserved_amount: T::Balance,
		},
		/// Deposited some balance into an account
		Deposited {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// Some funds are locked
		LockSet {
			lock_id: LockIdentifier,
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// Some locked funds were unlocked
		LockRemoved {
			lock_id: LockIdentifier,
			currency_id: T::CurrencyId,
			who: T::AccountId,
		},
		/// Some free balance was locked.
		Locked {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
		/// Some locked balance was freed.
		Unlocked {
			currency_id: T::CurrencyId,
			who: T::AccountId,
			amount: T::Balance,
		},
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

	/// Named reserves on some account balances.
	#[pallet::storage]
	#[pallet::getter(fn reserves)]
	pub type Reserves<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Twox64Concat,
		T::CurrencyId,
		BoundedVec<ReserveData<T::ReserveIdentifier, T::Balance>, T::MaxReserves>,
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
					Pallet::<T>::mutate_account(*currency_id, account_id, |account_data| {
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
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			#[pallet::compact] amount: T::Balance,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			Self::do_transfer(currency_id, &from, &to, amount, ExistenceRequirement::AllowDeath)
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
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::transfer_all())]
		pub fn transfer_all(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: T::CurrencyId,
			keep_alive: bool,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			let preservation = if keep_alive {
				Preservation::Protect
			} else {
				Preservation::Expendable
			};
			let reducible_balance = <Self as fungibles::Inspect<T::AccountId>>::reducible_balance(
				currency_id,
				&from,
				preservation,
				Fortitude::Polite,
			);
			<Self as fungibles::Mutate<_>>::transfer(currency_id, &from, &to, reducible_balance, preservation)
				.map(|_| ())
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
		#[pallet::call_index(2)]
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
		#[pallet::call_index(3)]
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
			Self::do_transfer(currency_id, &from, &to, amount, ExistenceRequirement::AllowDeath)
		}

		/// Set the balances of a given account.
		///
		/// This will alter `FreeBalance` and `ReservedBalance` in storage. it
		/// will also decrease the total issuance of the system
		/// (`TotalIssuance`). If the new free or reserved balance is below the
		/// existential deposit, it will reap the `AccountInfo`.
		///
		/// The dispatch origin for this call is `root`.
		#[pallet::call_index(4)]
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

			Self::try_mutate_account(currency_id, &who, |account, _| -> DispatchResult {
				let mut new_total = new_free.checked_add(&new_reserved).ok_or(ArithmeticError::Overflow)?;
				let (new_free, new_reserved) = if new_total < T::ExistentialDeposits::get(&currency_id) {
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
							.checked_add(&(new_total.defensive_saturating_sub(old_total)))
							.ok_or(ArithmeticError::Overflow)?;
						Ok(())
					})?;
				} else if new_total < old_total {
					TotalIssuance::<T>::try_mutate(currency_id, |t| -> DispatchResult {
						*t = t
							.checked_sub(&(old_total.defensive_saturating_sub(new_total)))
							.ok_or(ArithmeticError::Underflow)?;
						Ok(())
					})?;
				}

				Self::deposit_event(Event::BalanceSet {
					currency_id,
					who: who.clone(),
					free: new_free,
					reserved: new_reserved,
				});
				Ok(())
			})?;

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn ed(currency_id: T::CurrencyId) -> T::Balance {
		T::ExistentialDeposits::get(&currency_id)
	}

	fn in_dust_removal_whitelist(who: &T::AccountId) -> bool {
		T::DustRemovalWhitelist::contains(who)
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
			new_balance >= Self::accounts(who, currency_id).frozen,
			Error::<T>::LiquidityRestrictions
		);
		Ok(())
	}

	/// Mutate an account to some new value, or delete it entirely with `None`.
	/// Will enforce `ExistentialDeposit` law, annulling the account as needed.
	/// This will do nothing if the result of `f` is an `Err`.
	///
	/// It returns both the result from the closure, and an optional amount of
	/// dust which should be handled once it is known that all nested mutates
	/// that could affect storage items what the dust handler touches have
	/// completed.
	///
	/// NOTE: Doesn't do any preparatory work for creating a new account, so
	/// should only be used when it is known that the account already exists.
	///
	/// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is
	/// expected that the caller will do this.
	pub(crate) fn try_mutate_account<R, E>(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> Result<R, E>,
	) -> Result<(R, Option<T::Balance>), E> {
		Accounts::<T>::try_mutate_exists(who, currency_id, |maybe_account| {
			let is_new = maybe_account.is_none();
			let mut account = maybe_account.take().unwrap_or_default();

			let result = f(&mut account, is_new)?;

			let maybe_endowed = if is_new { Some(account.free) } else { None };

			// Handle any steps needed after mutating an account.
			//
			// This includes DustRemoval unbalancing, in the case than the `new` account's total
			// balance is non-zero but below ED.
			//
			// Updates `maybe_account` to `Some` iff the account has sufficient balance.
			// Evaluates `maybe_dust`, which is `Some` containing the dust to be dropped, iff
			// some dust should be dropped.
			//
			// We should never be dropping if reserved is non-zero. Reserved being non-zero
			// should imply that we have a consumer ref, so this is economically safe.
			let ed = Self::ed(currency_id);
			let maybe_dust = if account.free < ed && account.reserved.is_zero() {
				if account.free.is_zero() {
					None
				} else if Self::in_dust_removal_whitelist(who) {
					// NOTE: if the account is in the dust removal whitelist, don't drop!
					*maybe_account = Some(account);

					None
				} else {
					Some(account.free)
				}
			} else {
				assert!(
					account.free.is_zero() || account.free >= ed || !account.reserved.is_zero()
				);
				*maybe_account = Some(account);
				None
			};

			let exists = maybe_account.is_some();

			if !is_new && !exists {
				// If existed before, decrease account provider.
				// Ignore the result, because if it failed then there are remaining consumers,
				// and the account storage in frame_system shouldn't be reaped.
				let _ = frame_system::Pallet::<T>::dec_providers(who);
				<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnKilledTokenAccount::happened(&(who.clone(), currency_id));
			} else if is_new && exists {
				// if new, increase account provider
				frame_system::Pallet::<T>::inc_providers(who);
				<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnNewTokenAccount::happened(&(who.clone(), currency_id));
			}

			Ok((maybe_endowed, maybe_dust, result))
		}).map(|(maybe_endowed, maybe_dust, result)| {
			if let Some(endowed) = maybe_endowed {
				Self::deposit_event(Event::Endowed {
					currency_id,
					who: who.clone(),
					amount: endowed,
				});
			}

			if let Some(dust_amount) = maybe_dust {
				<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnDust::on_dust(
					currency_id,
					who,
					dust_amount,
				);
				Self::deposit_event(Event::DustLost {
					currency_id,
					who: who.clone(),
					amount: dust_amount,
				});
			}

			(result, maybe_dust)
		})
	}

	/// Mutate an account to some new value, or delete it entirely with `None`.
	/// Will enforce `ExistentialDeposit` law, annulling the account as needed.
	///
	/// It returns both the result from the closure, and an optional amount of
	/// dust which should be handled once it is known that all nested mutates
	/// that could affect storage items what the dust handler touches have
	/// completed.
	///
	/// NOTE: Doesn't do any preparatory work for creating a new account, so
	/// should only be used when it is known that the account already exists.
	///
	/// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is
	/// expected that the caller will do this.
	pub(crate) fn mutate_account<R>(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		f: impl FnOnce(&mut AccountData<T::Balance>) -> R,
	) -> (R, Option<T::Balance>) {
		Self::try_mutate_account(currency_id, who, |account, _| -> Result<R, Infallible> {
			Ok(f(account))
		})
		.expect("Error is infallible; qed")
	}

	/// Mutate an account to some new value, or delete it entirely with `None`.
	/// Will enforce `ExistentialDeposit` law, annulling the account as needed.
	///
	/// It returns the result from the closure. Any dust is handled through the
	/// low-level `fungible::Unbalanced` trap-door for legacy dust management.
	///
	/// NOTE: Doesn't do any preparatory work for creating a new account, so
	/// should only be used when it is known that the account already exists.
	///
	/// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is
	/// expected that the caller will do this.
	pub(crate) fn try_mutate_account_handling_dust<R, E>(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> Result<R, E>,
	) -> Result<R, E> {
		let (r, maybe_dust) = Self::try_mutate_account(currency_id, who, f)?;
		if let Some(dust) = maybe_dust {
			<Self as fungibles::Unbalanced<_>>::handle_raw_dust(currency_id, dust);
		}
		Ok(r)
	}

	/// Mutate an account to some new value, or delete it entirely with `None`.
	/// Will enforce `ExistentialDeposit` law, annulling the account as needed.
	///
	/// It returns the result from the closure. Any dust is handled through the
	/// low-level `fungible::Unbalanced` trap-door for legacy dust management.
	///
	/// NOTE: Doesn't do any preparatory work for creating a new account, so
	/// should only be used when it is known that the account already exists.
	///
	/// NOTE: LOW-LEVEL: This will not attempt to maintain total issuance. It is
	/// expected that the caller will do this.
	pub(crate) fn mutate_account_handling_dust<R>(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		f: impl FnOnce(&mut AccountData<T::Balance>) -> R,
	) -> R {
		let (r, maybe_dust) = Self::mutate_account(currency_id, who, f);
		if let Some(dust) = maybe_dust {
			<Self as fungibles::Unbalanced<_>>::handle_raw_dust(currency_id, dust);
		}
		r
	}

	/// Set free balance of `who` to a new value.
	///
	/// Note: this will not maintain total issuance, and the caller is expected
	/// to do it. If it will cause the account to be removed dust, shouldn't use
	/// it, because maybe the account that should be reaped to remain due to
	/// failed transfer/withdraw dust.
	pub(crate) fn set_free_balance(currency_id: T::CurrencyId, who: &T::AccountId, amount: T::Balance) {
		Self::mutate_account(currency_id, who, |account| {
			account.free = amount;

			Self::deposit_event(Event::BalanceSet {
				currency_id,
				who: who.clone(),
				free: account.free,
				reserved: account.reserved,
			});
		});
	}

	/// Set reserved balance of `who` to a new value.
	///
	/// Note: this will not maintain total issuance, and the caller is expected
	/// to do it. If it will cause the account to be removed dust, shouldn't use
	/// it, because maybe the account that should be reaped to remain due to
	/// failed transfer/withdraw dust.
	pub(crate) fn set_reserved_balance(currency_id: T::CurrencyId, who: &T::AccountId, amount: T::Balance) {
		Self::mutate_account(currency_id, who, |account| {
			account.reserved = amount;

			Self::deposit_event(Event::BalanceSet {
				currency_id,
				who: who.clone(),
				free: account.free,
				reserved: account.reserved,
			});
		});
	}

	/// Update the account entry for `who` under `currency_id`, given the
	/// locks.
	pub(crate) fn update_locks(
		currency_id: T::CurrencyId,
		who: &T::AccountId,
		locks: &[BalanceLock<T::Balance>],
	) -> DispatchResult {
		// track lock delta
		let mut total_frozen_prev = Zero::zero();
		let mut total_frozen_after = Zero::zero();

		// update account data
		Self::mutate_account(currency_id, who, |account| {
			total_frozen_prev = account.frozen;
			account.frozen = Zero::zero();
			for lock in locks.iter() {
				account.frozen = account.frozen.max(lock.amount);
			}
			total_frozen_after = account.frozen;
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

		if total_frozen_prev < total_frozen_after {
			let amount = total_frozen_after.saturating_sub(total_frozen_prev);
			Self::deposit_event(Event::Locked {
				currency_id,
				who: who.clone(),
				amount,
			});
		} else if total_frozen_prev > total_frozen_after {
			let amount = total_frozen_prev.saturating_sub(total_frozen_after);
			Self::deposit_event(Event::Unlocked {
				currency_id,
				who: who.clone(),
				amount,
			});
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

		<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::PreTransfer::on_transfer(
			currency_id,
			from,
			to,
			amount,
		)?;
		Self::try_mutate_account(currency_id, to, |to_account, _| -> DispatchResult {
			Self::try_mutate_account(currency_id, from, |from_account, _| -> DispatchResult {
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
		})?;

		<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::PostTransfer::on_transfer(
			currency_id,
			from,
			to,
			amount,
		)?;
		Self::deposit_event(Event::Transfer {
			currency_id,
			from: from.clone(),
			to: to.clone(),
			amount,
		});
		Ok(())
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

		Self::try_mutate_account(currency_id, who, |account, _| -> DispatchResult {
			Self::ensure_can_withdraw(currency_id, who, amount)?;
			let previous_total = account.total();
			account.free = account.free.defensive_saturating_sub(amount);

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
				TotalIssuance::<T>::mutate(currency_id, |v| *v = v.defensive_saturating_sub(amount));
			}

			Self::deposit_event(Event::Withdrawn {
				currency_id,
				who: who.clone(),
				amount,
			});
			Ok(())
		})?;

		Ok(())
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
	) -> Result<T::Balance, DispatchError> {
		if amount.is_zero() {
			return Ok(amount);
		}

		<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::PreDeposit::on_deposit(
			currency_id,
			who,
			amount,
		)?;
		Self::try_mutate_account(currency_id, who, |account, is_new| -> DispatchResult {
			if require_existed {
				ensure!(!is_new, Error::<T>::DeadAccount);
			} else {
				let ed = T::ExistentialDeposits::get(&currency_id);
				// Note: if who is in dust removal whitelist, allow to deposit the amount that
				// below ED to it.
				ensure!(
					amount >= ed || !is_new || T::DustRemovalWhitelist::contains(who),
					Error::<T>::ExistentialDeposit
				);
			}

			let new_total_issuance = Self::total_issuance(currency_id)
				.checked_add(&amount)
				.ok_or(ArithmeticError::Overflow)?;
			if change_total_issuance {
				TotalIssuance::<T>::mutate(currency_id, |v| *v = new_total_issuance);
			}
			account.free = account.free.defensive_saturating_add(amount);
			Ok(())
		})?;
		<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::PostDeposit::on_deposit(
			currency_id,
			who,
			amount,
		)?;
		Self::deposit_event(Event::Deposited {
			currency_id,
			who: who.clone(),
			amount,
		});
		Ok(amount)
	}
}
