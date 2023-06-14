// wrapping these imbalances in a private module is necessary to ensure absolute
// privacy of the inner member.
use super::*;
use frame_support::traits::{SameOrOther, TryDrop};
use sp_std::{marker, mem, result};

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been created without any equal and opposite
/// accounting.
#[must_use]
pub struct PositiveImbalance<T: Config, GetCurrencyId: Get<T::CurrencyId>>(
	T::Balance,
	marker::PhantomData<GetCurrencyId>,
);

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> PositiveImbalance<T, GetCurrencyId> {
	/// Create a new positive imbalance from a balance.
	pub fn new(amount: T::Balance) -> Self {
		PositiveImbalance(amount, marker::PhantomData::<GetCurrencyId>)
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Default for PositiveImbalance<T, GetCurrencyId> {
	fn default() -> Self {
		Self::zero()
	}
}

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been destroyed without any equal and opposite
/// accounting.
#[must_use]
pub struct NegativeImbalance<T: Config, GetCurrencyId: Get<T::CurrencyId>>(
	T::Balance,
	marker::PhantomData<GetCurrencyId>,
);

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> NegativeImbalance<T, GetCurrencyId> {
	/// Create a new negative imbalance from a balance.
	pub fn new(amount: T::Balance) -> Self {
		NegativeImbalance(amount, marker::PhantomData::<GetCurrencyId>)
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Default for NegativeImbalance<T, GetCurrencyId> {
	fn default() -> Self {
		Self::zero()
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> TryDrop for PositiveImbalance<T, GetCurrencyId> {
	fn try_drop(self) -> result::Result<(), Self> {
		self.drop_zero()
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Imbalance<T::Balance> for PositiveImbalance<T, GetCurrencyId> {
	type Opposite = NegativeImbalance<T, GetCurrencyId>;

	fn zero() -> Self {
		Self::new(Zero::zero())
	}
	fn drop_zero(self) -> result::Result<(), Self> {
		if self.0.is_zero() {
			Ok(())
		} else {
			Err(self)
		}
	}
	fn split(self, amount: T::Balance) -> (Self, Self) {
		let first = self.0.min(amount);
		let second = self.0.saturating_sub(first);

		mem::forget(self);
		(Self::new(first), Self::new(second))
	}
	fn merge(mut self, other: Self) -> Self {
		self.0 = self.0.saturating_add(other.0);
		mem::forget(other);

		self
	}
	fn subsume(&mut self, other: Self) {
		self.0 = self.0.saturating_add(other.0);
		mem::forget(other);
	}
	// allow to make the impl same with `pallet-balances`
	#[allow(clippy::comparison_chain)]
	fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
		let (a, b) = (self.0, other.0);
		mem::forget((self, other));

		if a > b {
			SameOrOther::Same(Self::new(a.saturating_sub(b)))
		} else if b > a {
			SameOrOther::Other(NegativeImbalance::new(b.saturating_sub(a)))
		} else {
			SameOrOther::None
		}
	}
	fn peek(&self) -> T::Balance {
		self.0
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> TryDrop for NegativeImbalance<T, GetCurrencyId> {
	fn try_drop(self) -> result::Result<(), Self> {
		self.drop_zero()
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Imbalance<T::Balance> for NegativeImbalance<T, GetCurrencyId> {
	type Opposite = PositiveImbalance<T, GetCurrencyId>;

	fn zero() -> Self {
		Self::new(Zero::zero())
	}
	fn drop_zero(self) -> result::Result<(), Self> {
		if self.0.is_zero() {
			Ok(())
		} else {
			Err(self)
		}
	}
	fn split(self, amount: T::Balance) -> (Self, Self) {
		let first = self.0.min(amount);
		let second = self.0.saturating_sub(first);

		mem::forget(self);
		(Self::new(first), Self::new(second))
	}
	fn merge(mut self, other: Self) -> Self {
		self.0 = self.0.saturating_add(other.0);
		mem::forget(other);

		self
	}
	fn subsume(&mut self, other: Self) {
		self.0 = self.0.saturating_add(other.0);
		mem::forget(other);
	}
	// allow to make the impl same with `pallet-balances`
	#[allow(clippy::comparison_chain)]
	fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
		let (a, b) = (self.0, other.0);
		mem::forget((self, other));

		if a > b {
			SameOrOther::Same(Self::new(a.saturating_sub(b)))
		} else if b > a {
			SameOrOther::Other(PositiveImbalance::new(b.saturating_sub(a)))
		} else {
			SameOrOther::None
		}
	}
	fn peek(&self) -> T::Balance {
		self.0
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Drop for PositiveImbalance<T, GetCurrencyId> {
	/// Basic drop handler will just square up the total issuance.
	fn drop(&mut self) {
		TotalIssuance::<T>::mutate(GetCurrencyId::get(), |v| *v = v.saturating_add(self.0));
	}
}

impl<T: Config, GetCurrencyId: Get<T::CurrencyId>> Drop for NegativeImbalance<T, GetCurrencyId> {
	/// Basic drop handler will just square up the total issuance.
	fn drop(&mut self) {
		TotalIssuance::<T>::mutate(GetCurrencyId::get(), |v| *v = v.saturating_sub(self.0));
	}
}

/// Implementation of Currency traits for Tokens Module.
impl<T: Config> MultiCurrency<T::AccountId> for Pallet<T> {
	type CurrencyId = T::CurrencyId;
	type Balance = T::Balance;

	fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
		Self::ed(currency_id)
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
		// need change total issuance
		Self::do_deposit(currency_id, who, amount, false, true)?;
		Ok(())
	}

	fn withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		// allow death
		// need change total issuance
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
			// Cannot underflow becuase free_slashed_amount can never be greater than
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
		Self::mutate_account_handling_dust(currency_id, who, |account| {
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

		Self::mutate_account_handling_dust(currency_id, who, |account| {
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

		let remaining = Self::mutate_account_handling_dust(currency_id, who, |account| {
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
