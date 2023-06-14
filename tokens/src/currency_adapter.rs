//! The adapter for specific token, which implements the
//! frame_support::traits::Currency traits, orml Currency traits and fungible
//! traits.

use super::*;

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
			currency_id,
			who,
			|account, is_new| -> Result<SignedImbalance<Self::Balance, Self::PositiveImbalance>, ()> {
				// If we're attempting to set an existing account to less than ED, then
				// bypass the entire operation. It's a no-op if you follow it through, but
				// since this is an instance where we might account for a negative imbalance
				// (in the dust cleaner of set_account) before we account for its actual
				// equal and opposite cause (returned as an Imbalance), then in the
				// instance that there's no other accounts on the system at all, we might
				// underflow the issuance and our arithmetic will be off.
				let ed = T::ExistentialDeposits::get(&currency_id);
				ensure!(value.saturating_add(account.reserved) >= ed || !is_new, ());

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
