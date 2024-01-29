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

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{
		tokens::{
			fungible, fungibles, DepositConsequence, Fortitude, Precision, Preservation, Provenance, Restriction,
			WithdrawConsequence,
		},
		BalanceStatus as Status, Contains, Currency as PalletCurrency, DefensiveSaturating, ExistenceRequirement, Get,
		Imbalance, LockableCurrency as PalletLockableCurrency,
		NamedReservableCurrency as PalletNamedReservableCurrency, ReservableCurrency as PalletReservableCurrency,
		SignedImbalance, WithdrawReasons,
	},
	transactional, BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use parity_scale_codec::MaxEncodedLen;
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

mod imbalances;
mod impls;
mod mock;
mod tests;
mod tests_currency_adapter;
mod tests_events;
mod tests_fungibles;
mod tests_multicurrency;

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
		Issued {
			currency_id: T::CurrencyId,
			amount: T::Balance,
		},
		Rescinded {
			currency_id: T::CurrencyId,
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

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				balances: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// ensure no duplicates exist.
			let unique_endowed_accounts = self
				.balances
				.iter()
				.map(|(account_id, currency_id, _)| (account_id, currency_id))
				.collect::<sp_std::collections::btree_set::BTreeSet<_>>();
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
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

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

			Self::try_mutate_account(&who, currency_id, |account, _| -> DispatchResult {
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
			None => return WithdrawConsequence::BalanceLow,
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
			None => return WithdrawConsequence::BalanceLow,
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
	) -> sp_std::result::Result<(R, Option<T::Balance>), E> {
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
				<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnKilledTokenAccount::happened(&(who.clone(), currency_id));
			} else if !existed && exists {
				// if new, increase account provider
				frame_system::Pallet::<T>::inc_providers(who);
				<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnNewTokenAccount::happened(&(who.clone(), currency_id));
			}

			if let Some(endowed) = maybe_endowed {
				Self::deposit_event(Event::Endowed {
					currency_id,
					who: who.clone(),
					amount: endowed,
				});
			}

			if let Some(dust_amount) = maybe_dust {
				// `OnDust` maybe get/set storage `Accounts` of `who`, trigger handler here
				// to avoid some unexpected errors.
				<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnDust::on_dust(who, currency_id, dust_amount);

				Self::deposit_event(Event::DustLost {
					currency_id,
					who: who.clone(),
					amount: dust_amount,
				});
			}

			(result, maybe_dust)
		})
	}

	pub(crate) fn mutate_account<R>(
		who: &T::AccountId,
		currency_id: T::CurrencyId,
		f: impl FnOnce(&mut AccountData<T::Balance>, bool) -> R,
	) -> (R, Option<T::Balance>) {
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
		Self::mutate_account(who, currency_id, |account, _| {
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
		Self::mutate_account(who, currency_id, |account, _| {
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

		Self::try_mutate_account(who, currency_id, |account, _existed| -> DispatchResult {
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
		Self::do_deposit(currency_id, who, amount, false, true)?;
		Ok(())
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

		<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnSlash::on_slash(
			currency_id,
			who,
			amount,
		);
		let account = Self::accounts(who, currency_id);
		let free_slashed_amount = account.free.min(amount);
		// Cannot underflow because free_slashed_amount can never be greater than amount
		// but just to be defensive here.
		let mut remaining_slash = amount.defensive_saturating_sub(free_slashed_amount);

		// slash free balance
		if !free_slashed_amount.is_zero() {
			// Cannot underflow because free_slashed_amount can never be greater than
			// account.free but just to be defensive here.
			Self::set_free_balance(
				currency_id,
				who,
				account.free.defensive_saturating_sub(free_slashed_amount),
			);
		}

		// slash reserved balance
		let reserved_slashed_amount = account.reserved.min(remaining_slash);

		if !reserved_slashed_amount.is_zero() {
			// Cannot underflow due to above line but just to be defensive here.
			remaining_slash = remaining_slash.defensive_saturating_sub(reserved_slashed_amount);
			Self::set_reserved_balance(
				currency_id,
				who,
				account.reserved.defensive_saturating_sub(reserved_slashed_amount),
			);
		}

		// Cannot underflow because the slashed value cannot be greater than total
		// issuance but just to be defensive here.
		TotalIssuance::<T>::mutate(currency_id, |v| {
			*v = v.defensive_saturating_sub(amount.defensive_saturating_sub(remaining_slash))
		});

		Self::deposit_event(Event::Slashed {
			currency_id,
			who: who.clone(),
			free_amount: free_slashed_amount,
			reserved_amount: reserved_slashed_amount,
		});
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
	type Moment = BlockNumberFor<T>;

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
		Self::update_locks(currency_id, who, &locks[..])?;

		Self::deposit_event(Event::LockSet {
			lock_id,
			currency_id,
			who: who.clone(),
			amount,
		});
		Ok(())
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
					new_lock.take().map(|nl| {
						let new_amount = lock.amount.max(nl.amount);
						Self::deposit_event(Event::LockSet {
							lock_id,
							currency_id,
							who: who.clone(),
							amount: new_amount,
						});
						BalanceLock {
							id: lock.id,
							amount: new_amount,
						}
					})
				} else {
					Some(lock)
				}
			})
			.collect::<Vec<_>>();
		if let Some(lock) = new_lock {
			Self::deposit_event(Event::LockSet {
				lock_id,
				currency_id,
				who: who.clone(),
				amount: lock.amount,
			});
			locks.push(lock)
		}
		Self::update_locks(currency_id, who, &locks[..])
	}

	fn remove_lock(lock_id: LockIdentifier, currency_id: Self::CurrencyId, who: &T::AccountId) -> DispatchResult {
		let mut locks = Self::locks(who, currency_id);
		locks.retain(|lock| lock.id != lock_id);
		let locks_vec = locks.to_vec();
		Self::update_locks(currency_id, who, &locks_vec[..])?;

		Self::deposit_event(Event::LockRemoved {
			lock_id,
			currency_id,
			who: who.clone(),
		});
		Ok(())
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

		<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnSlash::on_slash(
			currency_id,
			who,
			value,
		);
		let reserved_balance = Self::reserved_balance(currency_id, who);
		let actual = reserved_balance.min(value);
		Self::mutate_account(who, currency_id, |account, _| {
			// ensured reserved_balance >= actual but just to be defensive here.
			account.reserved = reserved_balance.defensive_saturating_sub(actual);
		});
		TotalIssuance::<T>::mutate(currency_id, |v| *v = v.defensive_saturating_sub(actual));

		Self::deposit_event(Event::Slashed {
			currency_id,
			who: who.clone(),
			free_amount: Zero::zero(),
			reserved_amount: actual,
		});
		value.defensive_saturating_sub(actual)
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

		Self::mutate_account(who, currency_id, |account, _| {
			account.free = account.free.defensive_saturating_sub(value);
			account.reserved = account.reserved.defensive_saturating_add(value);

			Self::deposit_event(Event::Reserved {
				currency_id,
				who: who.clone(),
				amount: value,
			});
		});

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

		let (remaining, _) = Self::mutate_account(who, currency_id, |account, _| {
			let actual = account.reserved.min(value);
			account.reserved = account.reserved.defensive_saturating_sub(actual);
			account.free = account.free.defensive_saturating_add(actual);

			Self::deposit_event(Event::Unreserved {
				currency_id,
				who: who.clone(),
				amount: actual,
			});
			value.defensive_saturating_sub(actual)
		});

		remaining
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
				Self::set_free_balance(
					currency_id,
					beneficiary,
					to_account.free.defensive_saturating_add(actual),
				);
			}
			BalanceStatus::Reserved => {
				Self::set_reserved_balance(
					currency_id,
					beneficiary,
					to_account.reserved.defensive_saturating_add(actual),
				);
			}
		}
		Self::set_reserved_balance(
			currency_id,
			slashed,
			from_account.reserved.defensive_saturating_sub(actual),
		);

		Self::deposit_event(Event::<T>::ReserveRepatriated {
			currency_id,
			from: slashed.clone(),
			to: beneficiary.clone(),
			amount: actual,
			status,
		});
		Ok(value.defensive_saturating_sub(actual))
	}
}

impl<T: Config> NamedMultiReservableCurrency<T::AccountId> for Pallet<T> {
	type ReserveIdentifier = T::ReserveIdentifier;

	fn reserved_balance_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
	) -> Self::Balance {
		let reserves = Self::reserves(who, currency_id);
		reserves
			.binary_search_by_key(id, |data| data.id)
			.map(|index| reserves[index].amount)
			.unwrap_or_default()
	}

	/// Move `value` from the free balance from `who` to a named reserve
	/// balance.
	///
	/// Is a no-op if value to be reserved is zero.
	fn reserve_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> DispatchResult {
		if value.is_zero() {
			return Ok(());
		}

		Reserves::<T>::try_mutate(who, currency_id, |reserves| -> DispatchResult {
			match reserves.binary_search_by_key(id, |data| data.id) {
				Ok(index) => {
					// this add can't overflow but just to be defensive.
					reserves[index].amount = reserves[index].amount.defensive_saturating_add(value);
				}
				Err(index) => {
					reserves
						.try_insert(index, ReserveData { id: *id, amount: value })
						.map_err(|_| Error::<T>::TooManyReserves)?;
				}
			};
			<Self as MultiReservableCurrency<_>>::reserve(currency_id, who, value)
		})
	}

	/// Unreserve some funds, returning any amount that was unable to be
	/// unreserved.
	///
	/// Is a no-op if the value to be unreserved is zero.
	fn unreserve_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		if value.is_zero() {
			return Zero::zero();
		}

		Reserves::<T>::mutate_exists(who, currency_id, |maybe_reserves| -> Self::Balance {
			if let Some(reserves) = maybe_reserves.as_mut() {
				match reserves.binary_search_by_key(id, |data| data.id) {
					Ok(index) => {
						let to_change = cmp::min(reserves[index].amount, value);

						let remain = <Self as MultiReservableCurrency<_>>::unreserve(currency_id, who, to_change);

						// remain should always be zero but just to be defensive here.
						let actual = to_change.defensive_saturating_sub(remain);

						// `actual <= to_change` and `to_change <= amount`, but just to be defensive
						// here.
						reserves[index].amount = reserves[index].amount.defensive_saturating_sub(actual);

						if reserves[index].amount.is_zero() {
							if reserves.len() == 1 {
								// no more named reserves
								*maybe_reserves = None;
							} else {
								// remove this named reserve
								reserves.remove(index);
							}
						}
						value.defensive_saturating_sub(actual)
					}
					Err(_) => value,
				}
			} else {
				value
			}
		})
	}

	/// Slash from reserved balance, returning the amount that was unable to be
	/// slashed.
	///
	/// Is a no-op if the value to be slashed is zero.
	fn slash_reserved_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		who: &T::AccountId,
		value: Self::Balance,
	) -> Self::Balance {
		if value.is_zero() {
			return Zero::zero();
		}

		Reserves::<T>::mutate(who, currency_id, |reserves| -> Self::Balance {
			match reserves.binary_search_by_key(id, |data| data.id) {
				Ok(index) => {
					let to_change = cmp::min(reserves[index].amount, value);

					let remain = <Self as MultiReservableCurrency<_>>::slash_reserved(currency_id, who, to_change);

					// remain should always be zero but just to be defensive here.
					let actual = to_change.defensive_saturating_sub(remain);

					// `actual <= to_change` and `to_change <= amount` but just to be defensive
					// here.
					reserves[index].amount = reserves[index].amount.defensive_saturating_sub(actual);

					Self::deposit_event(Event::Slashed {
						who: who.clone(),
						currency_id,
						free_amount: Zero::zero(),
						reserved_amount: actual,
					});
					value.defensive_saturating_sub(actual)
				}
				Err(_) => value,
			}
		})
	}

	/// Move the reserved balance of one account into the balance of another,
	/// according to `status`. If `status` is `Reserved`, the balance will be
	/// reserved with given `id`.
	///
	/// Is a no-op if:
	/// - the value to be moved is zero; or
	/// - the `slashed` id equal to `beneficiary` and the `status` is
	///   `Reserved`.
	fn repatriate_reserved_named(
		id: &Self::ReserveIdentifier,
		currency_id: Self::CurrencyId,
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: Status,
	) -> Result<Self::Balance, DispatchError> {
		if value.is_zero() {
			return Ok(Zero::zero());
		}

		if slashed == beneficiary {
			return match status {
				Status::Free => Ok(Self::unreserve_named(id, currency_id, slashed, value)),
				Status::Reserved => Ok(value.saturating_sub(Self::reserved_balance_named(id, currency_id, slashed))),
			};
		}

		Reserves::<T>::try_mutate(
			slashed,
			currency_id,
			|reserves| -> Result<Self::Balance, DispatchError> {
				match reserves.binary_search_by_key(id, |data| data.id) {
					Ok(index) => {
						let to_change = cmp::min(reserves[index].amount, value);

						let actual = if status == Status::Reserved {
							// make it the reserved under same identifier
							Reserves::<T>::try_mutate(
								beneficiary,
								currency_id,
								|reserves| -> Result<T::Balance, DispatchError> {
									match reserves.binary_search_by_key(id, |data| data.id) {
										Ok(index) => {
											let remain = <Self as MultiReservableCurrency<_>>::repatriate_reserved(
												currency_id,
												slashed,
												beneficiary,
												to_change,
												status,
											)?;

											// remain should always be zero but just to be defensive
											// here.
											let actual = to_change.defensive_saturating_sub(remain);

											// this add can't overflow but just to be defensive.
											reserves[index].amount =
												reserves[index].amount.defensive_saturating_add(actual);

											Ok(actual)
										}
										Err(index) => {
											let remain = <Self as MultiReservableCurrency<_>>::repatriate_reserved(
												currency_id,
												slashed,
												beneficiary,
												to_change,
												status,
											)?;

											// remain should always be zero but just to be defensive
											// here
											let actual = to_change.defensive_saturating_sub(remain);

											reserves
												.try_insert(
													index,
													ReserveData {
														id: *id,
														amount: actual,
													},
												)
												.map_err(|_| Error::<T>::TooManyReserves)?;

											Ok(actual)
										}
									}
								},
							)?
						} else {
							let remain = <Self as MultiReservableCurrency<_>>::repatriate_reserved(
								currency_id,
								slashed,
								beneficiary,
								to_change,
								status,
							)?;

							// remain should always be zero but just to be defensive here
							to_change.defensive_saturating_sub(remain)
						};

						// `actual <= to_change` and `to_change <= amount` but just to be defensive
						// here.
						reserves[index].amount = reserves[index].amount.defensive_saturating_sub(actual);
						Ok(value.defensive_saturating_sub(actual))
					}
					Err(_) => Ok(value),
				}
			},
		)
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
		Self::accounts(who, asset_id).free
	}

	fn total_balance(asset_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, asset_id).total()
	}

	fn reducible_balance(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		preservation: Preservation,
		_force: Fortitude,
	) -> Self::Balance {
		let a = Self::accounts(who, asset_id);
		// Liquid balance is what is neither reserved nor locked/frozen.
		let liquid = a.free.saturating_sub(a.frozen);
		if frame_system::Pallet::<T>::can_dec_provider(who) && !matches!(preservation, Preservation::Protect) {
			liquid
		} else {
			// `must_remain_to_exist` is the part of liquid balance which must remain to
			// keep total over ED.
			let must_remain_to_exist =
				T::ExistentialDeposits::get(&asset_id).saturating_sub(a.total().saturating_sub(liquid));
			liquid.saturating_sub(must_remain_to_exist)
		}
	}

	fn can_deposit(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
		_provenance: Provenance,
	) -> DepositConsequence {
		Self::deposit_consequence(who, asset_id, amount, &Self::accounts(who, asset_id))
	}

	fn can_withdraw(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		Self::withdraw_consequence(who, asset_id, amount, &Self::accounts(who, asset_id))
	}

	fn asset_exists(asset: Self::AssetId) -> bool {
		TotalIssuance::<T>::contains_key(asset)
	}
}

impl<T: Config> fungibles::Mutate<T::AccountId> for Pallet<T> {
	fn mint_into(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		Self::deposit_consequence(who, asset_id, amount, &Self::accounts(who, asset_id)).into_result()?;
		// do not require existing
		Self::do_deposit(asset_id, who, amount, false, true)
	}

	fn burn_from(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
		// TODO: Respect precision
		_precision: Precision,
		// TODO: Respect fortitude
		_fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		let extra =
			Self::withdraw_consequence(who, asset_id, amount, &Self::accounts(who, asset_id)).into_result(false)?;
		let actual = amount.defensive_saturating_add(extra);
		// allow death
		Self::do_withdraw(asset_id, who, actual, ExistenceRequirement::AllowDeath, true).map(|_| actual)
	}

	fn transfer(
		asset_id: Self::AssetId,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: T::Balance,
		preservation: Preservation,
	) -> Result<T::Balance, DispatchError> {
		let existence_requirement = match preservation {
			Preservation::Expendable => ExistenceRequirement::AllowDeath,
			Preservation::Protect | Preservation::Preserve => ExistenceRequirement::KeepAlive,
		};
		Self::do_transfer(asset_id, source, dest, amount, existence_requirement).map(|_| amount)
	}
}

impl<T: Config> fungibles::Unbalanced<T::AccountId> for Pallet<T> {
	fn handle_dust(_dust: fungibles::Dust<T::AccountId, Self>) {
		// Dust is handled in account mutate method
	}

	fn write_balance(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		let max_reduction = <Self as fungibles::Inspect<_>>::reducible_balance(
			asset_id,
			who,
			Preservation::Expendable,
			Fortitude::Force,
		);

		// Balance is the same type and will not overflow
		let (_, dust_amount) = Self::try_mutate_account(who, asset_id, |account, _| -> Result<(), DispatchError> {
			// Make sure the reduction (if there is one) is no more than the maximum
			// allowed.
			let reduction = account.free.saturating_sub(amount);
			ensure!(reduction <= max_reduction, Error::<T>::BalanceTooLow);

			account.free = amount;
			Self::deposit_event(Event::BalanceSet {
				currency_id: asset_id,
				who: who.clone(),
				free: account.free,
				reserved: account.reserved,
			});

			Ok(())
		})?;

		Ok(dust_amount)
	}

	fn set_total_issuance(asset_id: Self::AssetId, amount: Self::Balance) {
		// Balance is the same type and will not overflow
		TotalIssuance::<T>::mutate(asset_id, |t| *t = amount);

		Self::deposit_event(Event::TotalIssuanceSet {
			currency_id: asset_id,
			amount,
		});
	}

	fn decrease_balance(
		asset: Self::AssetId,
		who: &T::AccountId,
		mut amount: Self::Balance,
		precision: Precision,
		preservation: Preservation,
		force: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		let old_balance = <Pallet<T> as fungibles::Inspect<T::AccountId>>::balance(asset, who);
		let free = <Pallet<T> as fungibles::Inspect<T::AccountId>>::reducible_balance(asset, who, preservation, force);
		if let Precision::BestEffort = precision {
			amount = amount.min(free);
		}
		let new_balance = old_balance.checked_sub(&amount).ok_or(TokenError::FundsUnavailable)?;
		let _dust_amount = Self::write_balance(asset, who, new_balance)?.unwrap_or_default();

		// here just return decrease amount, shouldn't count the dust_amount
		Ok(old_balance.saturating_sub(new_balance))
	}
}

impl<T: Config> fungibles::Balanced<T::AccountId> for Pallet<T> {
	type OnDropDebt = fungibles::IncreaseIssuance<T::AccountId, Self>;
	type OnDropCredit = fungibles::DecreaseIssuance<T::AccountId, Self>;

	fn done_deposit(currency_id: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::Deposited {
			currency_id,
			who: who.clone(),
			amount,
		});
	}
	fn done_withdraw(currency_id: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::Withdrawn {
			currency_id,
			who: who.clone(),
			amount,
		});
	}
	fn done_issue(currency_id: Self::AssetId, amount: Self::Balance) {
		Self::deposit_event(Event::Issued { currency_id, amount });
	}
	fn done_rescind(currency_id: Self::AssetId, amount: Self::Balance) {
		Self::deposit_event(Event::Rescinded { currency_id, amount });
	}
}

type ReasonOf<P, T> = <P as fungibles::InspectHold<<T as frame_system::Config>::AccountId>>::Reason;
impl<T: Config> fungibles::InspectHold<T::AccountId> for Pallet<T> {
	type Reason = ();

	fn balance_on_hold(asset_id: Self::AssetId, _reason: &Self::Reason, who: &T::AccountId) -> T::Balance {
		Self::accounts(who, asset_id).reserved
	}

	fn total_balance_on_hold(asset: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, asset).reserved
	}

	fn reducible_total_balance_on_hold(_asset: Self::AssetId, _who: &T::AccountId, _force: Fortitude) -> Self::Balance {
		0u32.into()
	}

	fn hold_available(_asset: Self::AssetId, _reason: &Self::Reason, _who: &T::AccountId) -> bool {
		true
	}

	fn can_hold(asset_id: Self::AssetId, _reason: &Self::Reason, who: &T::AccountId, amount: T::Balance) -> bool {
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
	fn hold(
		asset_id: Self::AssetId,
		_reason: &ReasonOf<Self, T>,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		<Pallet<T> as MultiReservableCurrency<_>>::reserve(asset_id, who, amount)
	}

	fn release(
		asset_id: Self::AssetId,
		_reason: &ReasonOf<Self, T>,
		who: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
	) -> Result<T::Balance, DispatchError> {
		if amount.is_zero() {
			return Ok(amount);
		}

		// Done on a best-effort basis.
		let (released, _) =
			Self::try_mutate_account(who, asset_id, |a, _existed| -> Result<T::Balance, DispatchError> {
				let new_free = a.free.saturating_add(amount.min(a.reserved));
				let actual = new_free.defensive_saturating_sub(a.free);
				// Guaranteed to be <= amount and <= a.reserved
				ensure!(
					matches!(precision, Precision::BestEffort) || actual == amount,
					Error::<T>::BalanceTooLow
				);
				a.free = new_free;
				a.reserved = a.reserved.saturating_sub(actual);

				Self::deposit_event(Event::Unreserved {
					currency_id: asset_id,
					who: who.clone(),
					amount,
				});
				Ok(actual)
			})?;

		Ok(released)
	}

	fn transfer_on_hold(
		asset_id: Self::AssetId,
		reason: &ReasonOf<Self, T>,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
		restriction: Restriction,
		_fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		let status = if restriction == Restriction::OnHold {
			Status::Reserved
		} else {
			Status::Free
		};
		ensure!(
			amount <= <Self as fungibles::InspectHold<T::AccountId>>::balance_on_hold(asset_id, reason, source)
				|| precision == Precision::BestEffort,
			Error::<T>::BalanceTooLow
		);
		let gap = Self::repatriate_reserved(asset_id, source, dest, amount, status)?;
		// return actual transferred amount
		Ok(amount.saturating_sub(gap))
	}
}

impl<T: Config> fungibles::UnbalancedHold<T::AccountId> for Pallet<T> {
	fn set_balance_on_hold(
		asset: Self::AssetId,
		_reason: &Self::Reason,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		// Balance is the same type and will not overflow
		Self::try_mutate_account(who, asset, |account, _| -> Result<(), DispatchError> {
			let old_reserved = account.reserved;
			account.reserved = amount;
			// free = free + old - new
			account.free = account
				.free
				.checked_add(&old_reserved)
				.ok_or(ArithmeticError::Overflow)?
				.checked_sub(&account.reserved)
				.ok_or(TokenError::BelowMinimum)?;

			Self::deposit_event(Event::BalanceSet {
				currency_id: asset,
				who: who.clone(),
				free: account.free,
				reserved: account.reserved,
			});

			Ok(())
		})
		.map(|_| ())
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
		let currency_id = GetCurrencyId::get();
		TotalIssuance::<T>::mutate(currency_id, |issued| {
			*issued = issued.checked_sub(&amount).unwrap_or_else(|| {
				amount = *issued;
				Zero::zero()
			})
		});

		Pallet::<T>::deposit_event(Event::TotalIssuanceSet {
			currency_id,
			amount: Self::total_issuance(),
		});
		PositiveImbalance::new(amount)
	}

	fn issue(mut amount: Self::Balance) -> Self::NegativeImbalance {
		if amount.is_zero() {
			return NegativeImbalance::zero();
		}
		TotalIssuance::<T>::mutate(GetCurrencyId::get(), |issued| {
			*issued = issued.checked_add(&amount).unwrap_or_else(|| {
				amount = Self::Balance::max_value().defensive_saturating_sub(*issued);
				Self::Balance::max_value()
			})
		});

		Pallet::<T>::deposit_event(Event::TotalIssuanceSet {
			currency_id: GetCurrencyId::get(),
			amount: Self::total_issuance(),
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
		let mut remaining_slash = value.defensive_saturating_sub(free_slashed_amount);

		// slash free balance
		if !free_slashed_amount.is_zero() {
			Pallet::<T>::set_free_balance(
				currency_id,
				who,
				account.free.defensive_saturating_sub(free_slashed_amount),
			);
		}

		// slash reserved balance
		if !remaining_slash.is_zero() {
			let reserved_slashed_amount = account.reserved.min(remaining_slash);
			remaining_slash = remaining_slash.defensive_saturating_sub(reserved_slashed_amount);
			Pallet::<T>::set_reserved_balance(
				currency_id,
				who,
				account.reserved.defensive_saturating_sub(reserved_slashed_amount),
			);

			Pallet::<T>::deposit_event(Event::Slashed {
				currency_id,
				who: who.clone(),
				free_amount: free_slashed_amount,
				reserved_amount: reserved_slashed_amount,
			});
			(
				Self::NegativeImbalance::new(free_slashed_amount.saturating_add(reserved_slashed_amount)),
				remaining_slash,
			)
		} else {
			Pallet::<T>::deposit_event(Event::Slashed {
				currency_id,
				who: who.clone(),
				free_amount: value,
				reserved_amount: Zero::zero(),
			});
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
					SignedImbalance::Positive(PositiveImbalance::new(value.saturating_sub(account.free)))
				} else {
					SignedImbalance::Negative(NegativeImbalance::new(account.free.saturating_sub(value)))
				};
				account.free = value;

				Pallet::<T>::deposit_event(Event::BalanceSet {
					currency_id,
					who: who.clone(),
					free: value,
					reserved: account.reserved,
				});
				Ok(imbalance)
			},
		)
		.map(|(imbalance, _)| imbalance)
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

impl<T, GetCurrencyId> PalletNamedReservableCurrency<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type ReserveIdentifier = T::ReserveIdentifier;

	fn reserved_balance_named(id: &Self::ReserveIdentifier, who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as NamedMultiReservableCurrency<_>>::reserved_balance_named(id, GetCurrencyId::get(), who)
	}

	fn reserve_named(id: &Self::ReserveIdentifier, who: &T::AccountId, value: Self::Balance) -> DispatchResult {
		<Pallet<T> as NamedMultiReservableCurrency<_>>::reserve_named(id, GetCurrencyId::get(), who, value)
	}

	fn unreserve_named(id: &Self::ReserveIdentifier, who: &T::AccountId, value: Self::Balance) -> Self::Balance {
		<Pallet<T> as NamedMultiReservableCurrency<_>>::unreserve_named(id, GetCurrencyId::get(), who, value)
	}

	fn slash_reserved_named(
		id: &Self::ReserveIdentifier,
		who: &T::AccountId,
		value: Self::Balance,
	) -> (Self::NegativeImbalance, Self::Balance) {
		let actual =
			<Pallet<T> as NamedMultiReservableCurrency<_>>::slash_reserved_named(id, GetCurrencyId::get(), who, value);
		(Self::NegativeImbalance::zero(), actual)
	}

	fn repatriate_reserved_named(
		id: &Self::ReserveIdentifier,
		slashed: &T::AccountId,
		beneficiary: &T::AccountId,
		value: Self::Balance,
		status: Status,
	) -> sp_std::result::Result<Self::Balance, DispatchError> {
		<Pallet<T> as NamedMultiReservableCurrency<_>>::repatriate_reserved_named(
			id,
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
	type Moment = BlockNumberFor<T>;
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
	fn total_balance(who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<_>>::total_balance(GetCurrencyId::get(), who)
	}
	fn reducible_balance(who: &T::AccountId, preservation: Preservation, fortitude: Fortitude) -> Self::Balance {
		<Pallet<T> as fungibles::Inspect<_>>::reducible_balance(GetCurrencyId::get(), who, preservation, fortitude)
	}
	fn can_deposit(who: &T::AccountId, amount: Self::Balance, provenance: Provenance) -> DepositConsequence {
		<Pallet<T> as fungibles::Inspect<_>>::can_deposit(GetCurrencyId::get(), who, amount, provenance)
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
	fn mint_into(who: &T::AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError> {
		<Pallet<T> as fungibles::Mutate<_>>::mint_into(GetCurrencyId::get(), who, amount)
	}
	fn burn_from(
		who: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
		fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		<Pallet<T> as fungibles::Mutate<_>>::burn_from(GetCurrencyId::get(), who, amount, precision, fortitude)
	}

	fn transfer(
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: T::Balance,
		preservation: Preservation,
	) -> Result<T::Balance, DispatchError> {
		<Pallet<T> as fungibles::Mutate<_>>::transfer(GetCurrencyId::get(), source, dest, amount, preservation)
	}
}

impl<T, GetCurrencyId> fungible::Unbalanced<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn handle_dust(_dust: fungible::Dust<T::AccountId, Self>) {
		// Dust is handled in account mutate method
	}

	fn write_balance(who: &T::AccountId, amount: Self::Balance) -> Result<Option<Self::Balance>, DispatchError> {
		<Pallet<T> as fungibles::Unbalanced<_>>::write_balance(GetCurrencyId::get(), who, amount)
	}
	fn set_total_issuance(amount: Self::Balance) {
		<Pallet<T> as fungibles::Unbalanced<_>>::set_total_issuance(GetCurrencyId::get(), amount)
	}
}

type ReasonOfFungible<P, T> = <P as fungible::InspectHold<<T as frame_system::Config>::AccountId>>::Reason;
impl<T, GetCurrencyId> fungible::InspectHold<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type Reason = <Pallet<T> as fungibles::InspectHold<T::AccountId>>::Reason;

	fn balance_on_hold(reason: &Self::Reason, who: &T::AccountId) -> T::Balance {
		<Pallet<T> as fungibles::InspectHold<_>>::balance_on_hold(GetCurrencyId::get(), reason, who)
	}
	fn total_balance_on_hold(who: &T::AccountId) -> Self::Balance {
		<Pallet<T> as fungibles::InspectHold<_>>::total_balance_on_hold(GetCurrencyId::get(), who)
	}
	fn reducible_total_balance_on_hold(who: &T::AccountId, force: Fortitude) -> Self::Balance {
		<Pallet<T> as fungibles::InspectHold<_>>::reducible_total_balance_on_hold(GetCurrencyId::get(), who, force)
	}
	fn hold_available(reason: &Self::Reason, who: &T::AccountId) -> bool {
		<Pallet<T> as fungibles::InspectHold<_>>::hold_available(GetCurrencyId::get(), reason, who)
	}
	fn can_hold(reason: &Self::Reason, who: &T::AccountId, amount: T::Balance) -> bool {
		<Pallet<T> as fungibles::InspectHold<_>>::can_hold(GetCurrencyId::get(), reason, who, amount)
	}
}

impl<T, GetCurrencyId> fungible::MutateHold<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn hold(reason: &ReasonOfFungible<Self, T>, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T> as fungibles::MutateHold<_>>::hold(GetCurrencyId::get(), reason, who, amount)
	}
	fn release(
		reason: &ReasonOfFungible<Self, T>,
		who: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
	) -> Result<T::Balance, DispatchError> {
		<Pallet<T> as fungibles::MutateHold<_>>::release(GetCurrencyId::get(), reason, who, amount, precision)
	}
	fn transfer_on_hold(
		reason: &ReasonOfFungible<Self, T>,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
		restriction: Restriction,
		fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		<Pallet<T> as fungibles::MutateHold<_>>::transfer_on_hold(
			GetCurrencyId::get(),
			reason,
			source,
			dest,
			amount,
			precision,
			restriction,
			fortitude,
		)
	}
}

impl<T, GetCurrencyId> fungible::UnbalancedHold<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn set_balance_on_hold(reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		<Pallet<T> as fungibles::UnbalancedHold<_>>::set_balance_on_hold(GetCurrencyId::get(), reason, who, amount)
	}
}
