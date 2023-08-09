//! Implementation of `fungibles` traits for Tokens Module.
use super::*;

impl<T: Config> fungibles::Inspect<T::AccountId> for Pallet<T> {
	type AssetId = T::CurrencyId;
	type Balance = T::Balance;

	fn total_issuance(asset_id: Self::AssetId) -> Self::Balance {
		TotalIssuance::<T>::get(asset_id)
	}

	fn minimum_balance(asset_id: Self::AssetId) -> Self::Balance {
		Self::ed(asset_id)
	}

	fn total_balance(asset_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, asset_id).total()
	}

	fn balance(asset_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, asset_id).free
	}

	/// Get the maximum amount that `who` can withdraw/transfer successfully
	/// based on whether the account should be kept alive (`preservation`) or
	/// whether we are willing to force the transfer and potentially go below
	/// user-level restrictions on the minimum amount of the account.
	///
	/// Always less than `free_balance()`.
	fn reducible_balance(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		preservation: Preservation,
		force: Fortitude,
	) -> Self::Balance {
		let a = Self::accounts(who, asset_id);
		let mut untouchable = Zero::zero();
		if force == Fortitude::Polite {
			// Frozen balance applies to total. Anything on hold therefore gets discounted
			// from the limit given by the freezes.
			untouchable = a.frozen.saturating_sub(a.reserved);
		}
		// If we want to keep our provider ref..
		if preservation == Preservation::Preserve
			// ..or we don't want the account to die and our provider ref is needed for it to live..
			|| preservation == Preservation::Protect && !a.free.is_zero() &&
				frame_system::Pallet::<T>::providers(who) == 1
			// ..or we don't care about the account dying but our provider ref is required..
			|| preservation == Preservation::Expendable && !a.free.is_zero() &&
				!frame_system::Pallet::<T>::can_dec_provider(who)
		{
			// ..then the ED needed except for the account in dust removal whitelist.
			if !Self::in_dust_removal_whitelist(who) {
				untouchable = untouchable.max(Self::ed(asset_id));
			}
		}
		// Liquid balance is what is neither on hold nor frozen/required for provider.
		a.free.saturating_sub(untouchable)
	}

	fn can_deposit(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		if amount.is_zero() {
			return DepositConsequence::Success;
		}

		if provenance == Provenance::Minted && TotalIssuance::<T>::get(asset_id).checked_add(&amount).is_none() {
			return DepositConsequence::Overflow;
		}

		let account = Self::accounts(who, asset_id);
		let new_free_balance = match account.free.checked_add(&amount) {
			Some(x) if x < Self::ed(asset_id) => return DepositConsequence::BelowMinimum,
			Some(x) => x,
			None => return DepositConsequence::Overflow,
		};

		match account.reserved.checked_add(&new_free_balance) {
			Some(_) => {}
			None => return DepositConsequence::Overflow,
		};

		// NOTE: We assume that we are a provider, so don't need to do any checks in the
		// case of account creation.

		DepositConsequence::Success
	}

	fn can_withdraw(
		asset_id: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		if amount.is_zero() {
			return WithdrawConsequence::Success;
		}

		if TotalIssuance::<T>::get(asset_id).checked_sub(&amount).is_none() {
			return WithdrawConsequence::Underflow;
		}

		let account = Self::accounts(who, asset_id);
		let new_free_balance = match account.free.checked_sub(&amount) {
			Some(x) => x,
			None => return WithdrawConsequence::BalanceLow,
		};

		let liquid = Self::reducible_balance(asset_id, who, Preservation::Expendable, Fortitude::Polite);
		if amount > liquid {
			return WithdrawConsequence::Frozen;
		}

		// Provider restriction - total account balance cannot be reduced to zero if it
		// cannot sustain the loss of a provider reference.
		// NOTE: This assumes that the pallet is a provider (which is true). Is this
		// ever changes, then this will need to adapt accordingly.
		let ed = Self::ed(asset_id);
		let success = if new_free_balance < ed && !Self::in_dust_removal_whitelist(who) {
			if frame_system::Pallet::<T>::can_dec_provider(who) {
				WithdrawConsequence::ReducedToZero(new_free_balance)
			} else {
				return WithdrawConsequence::WouldDie;
			}
		} else {
			WithdrawConsequence::Success
		};

		let new_total_balance = new_free_balance.saturating_add(account.reserved);

		// Eventual total funds must be no less than the frozen balance.
		if new_total_balance < account.frozen {
			return WithdrawConsequence::Frozen;
		}

		success
	}

	fn asset_exists(asset: Self::AssetId) -> bool {
		TotalIssuance::<T>::contains_key(asset)
	}
}

impl<T: Config> fungibles::Unbalanced<T::AccountId> for Pallet<T> {
	fn handle_dust(dust: fungibles::Dust<T::AccountId, Self>) {
		T::DustRemoval::on_unbalanced(dust.into_credit());
	}

	/// Forcefully set the balance of `who` to `amount`.
	///
	/// If this call executes successfully, you can `assert_eq!(Self::balance(),
	/// amount);`.
	///
	/// For implementations which include one or more balances on hold, then
	/// these are *not* included in the `amount`.
	///
	/// This function does its best to force the balance change through, but
	/// will not break system invariants such as any Existential Deposits needed
	/// or overflows/underflows. If this cannot be done for some reason (e.g.
	/// because the account cannot be created, deleted or would overflow) then
	/// an `Err` is returned.
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
		let (result, maybe_dust) = Self::mutate_account(asset_id, who, |account| -> DispatchResult {
			// Make sure the reduction (if there is one) is no more than the maximum
			// allowed.
			let reduction = account.free.saturating_sub(amount);
			ensure!(reduction <= max_reduction, Error::<T>::BalanceTooLow);

			account.free = amount;
			Ok(())
		});
		result?;
		Ok(maybe_dust)
	}

	/// Increase the balance of `who` by `amount`.
	///
	/// If it cannot be increased by that amount for some reason, return `Err`
	/// and don't increase it at all. If Ok, return the imbalance.
	/// Minimum balance will be respected and an error will be returned if
	/// `amount < Self::minimum_balance()` when the account of `who` is zero.
	/// NOTE: this impl overrides the default implementation of
	/// fungibles::Unbalanced, allow `amount < Self::minimum_balance() && who is
	/// in DustRemovalWhitelist` when the account of `who` is zero
	fn increase_balance(
		asset: Self::AssetId,
		who: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
	) -> Result<Self::Balance, DispatchError> {
		let old_balance = <Self as fungibles::Inspect<_>>::balance(asset, who);
		let new_balance = if let Precision::BestEffort = precision {
			old_balance.saturating_add(amount)
		} else {
			old_balance.checked_add(&amount).ok_or(ArithmeticError::Overflow)?
		};
		if new_balance < <Self as fungibles::Inspect<_>>::minimum_balance(asset)
			&& !Self::in_dust_removal_whitelist(who)
		{
			// Attempt to increase from 0 to below minimum -> stays at zero.
			if let Precision::BestEffort = precision {
				Ok(Self::Balance::default())
			} else {
				Err(TokenError::BelowMinimum.into())
			}
		} else {
			if new_balance == old_balance {
				Ok(Self::Balance::default())
			} else {
				if let Some(dust) = Self::write_balance(asset, who, new_balance)? {
					Self::handle_dust(fungibles::Dust(asset, dust));
				}
				Ok(new_balance.saturating_sub(old_balance))
			}
		}
	}

	fn set_total_issuance(asset_id: Self::AssetId, amount: Self::Balance) {
		// Balance is the same type and will not overflow
		TotalIssuance::<T>::mutate(asset_id, |t| *t = amount);
	}
}

impl<T: Config> fungibles::Balanced<T::AccountId> for Pallet<T> {
	type OnDropCredit = fungibles::DecreaseIssuance<T::AccountId, Self>;
	type OnDropDebt = fungibles::IncreaseIssuance<T::AccountId, Self>;

	fn done_deposit(asset: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::Deposited {
			currency_id: asset,
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_withdraw(asset: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::Withdrawn {
			currency_id: asset,
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_issue(asset: Self::AssetId, amount: Self::Balance) {
		Self::deposit_event(Event::Issued {
			currency_id: asset,
			amount,
		});
	}

	fn done_rescind(asset: Self::AssetId, amount: Self::Balance) {
		Self::deposit_event(Event::Rescinded {
			currency_id: asset,
			amount,
		});
	}
}

impl<T: Config> fungibles::Mutate<T::AccountId> for Pallet<T> {
	fn done_mint_into(asset: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::Deposited {
			currency_id: asset,
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_burn_from(asset: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::Withdrawn {
			currency_id: asset,
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_shelve(asset: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::Withdrawn {
			currency_id: asset,
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_restore(asset: Self::AssetId, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::Deposited {
			currency_id: asset,
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_transfer(asset: Self::AssetId, source: &T::AccountId, dest: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::Transfer {
			currency_id: asset,
			from: source.clone(),
			to: dest.clone(),
			amount: amount,
		});
	}
}

impl<T: Config> fungibles::InspectHold<T::AccountId> for Pallet<T> {
	type Reason = ();

	fn total_balance_on_hold(asset: Self::AssetId, who: &T::AccountId) -> Self::Balance {
		Self::accounts(who, asset).reserved
	}

	/// Get the maximum amount that the `total_balance_on_hold` of `who` can be
	/// reduced successfully based on whether we are willing to force the
	/// reduction and potentially go below user-level restrictions on the
	/// minimum amount of the account. Note: This cannot bring the account into
	/// an inconsistent state with regards any required existential deposit.
	///
	/// Always less than `total_balance_on_hold()`.
	fn reducible_total_balance_on_hold(asset: Self::AssetId, who: &T::AccountId, force: Fortitude) -> Self::Balance {
		// The total balance must never drop below the freeze requirements if we're not
		// forcing:
		let a = Self::accounts(who, asset);
		let unavailable = if force == Fortitude::Force {
			Self::Balance::zero()
		} else {
			// The freeze lock applies to the total balance, so we can discount the free
			// balance from the amount which the total reserved balance must provide to
			// satisfy it.
			a.frozen.saturating_sub(a.free)
		};
		a.reserved.saturating_sub(unavailable)
	}

	fn balance_on_hold(asset_id: Self::AssetId, _reason: &Self::Reason, who: &T::AccountId) -> T::Balance {
		Self::accounts(who, asset_id).reserved
	}

	fn hold_available(_asset: Self::AssetId, _reason: &Self::Reason, _who: &T::AccountId) -> bool {
		true
	}
}

impl<T: Config> fungibles::UnbalancedHold<T::AccountId> for Pallet<T> {
	/// Forcefully set the balance on hold of `who` to `amount`. This is
	/// independent of any other balances on hold or the main ("free") balance.
	///
	/// If this call executes successfully, you can
	/// `assert_eq!(Self::balance_on_hold(), amount);`.
	///
	/// This function does its best to force the balance change through, but
	/// will not break system invariants such as any Existential Deposits needed
	/// or overflows/underflows. If this cannot be done for some reason (e.g.
	/// because the account doesn't exist) then an `Err` is returned.
	// Implmentation note: This should increment the consumer refs if it moves total
	// on hold from zero to non-zero and decrement in the opposite direction.
	//
	// Since this was not done in the previous logic, this will need either a
	// migration or a state item which tracks whether the account is on the old
	// logic or new.
	fn set_balance_on_hold(
		asset: Self::AssetId,
		_reason: &Self::Reason,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		// Balance is the same type and will not overflow
		let (result, maybe_dust) = Self::try_mutate_account(asset, who, |account, _| -> Result<(), DispatchError> {
			let old_reserved = account.reserved;
			let delta = old_reserved.max(amount) - old_reserved.min(amount);

			account.reserved = if amount > old_reserved {
				account.reserved.checked_add(&delta).ok_or(ArithmeticError::Overflow)?
			} else {
				account.reserved.checked_sub(&delta).ok_or(ArithmeticError::Underflow)?
			};

			Ok(())
		})?;

		debug_assert!(
			maybe_dust.is_none(),
			"Does not alter main balance; dust only happens when it is altered; qed"
		);

		Ok(result)
	}
}

impl<T: Config> fungibles::MutateHold<T::AccountId> for Pallet<T> {}

// TODO: impl fungibles::InspectFreeze and fungibles::MutateFreeze
