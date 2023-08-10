use super::*;
use frame_support::{
	traits::{
		tokens::{Balance as BalanceT, Restriction},
		Currency as PalletCurrency, LockableCurrency as PalletLockableCurrency,
		NamedReservableCurrency as PalletNamedReservableCurrency, ReservableCurrency as PalletReservableCurrency,
		SignedImbalance, WithdrawReasons,
	},
	transactional,
};

pub type CreditOf<T> = fungibles::Credit<<T as frame_system::Config>::AccountId, Pallet<T>>;
pub struct DustReceiver<T, GetAccountId>(sp_std::marker::PhantomData<(T, GetAccountId)>);
impl<T, GetAccountId> OnUnbalanced<CreditOf<T>> for DustReceiver<T, GetAccountId>
where
	T: Config,
	GetAccountId: Get<Option<T::AccountId>>,
{
	fn on_nonzero_unbalanced(amount: CreditOf<T>) {
		match GetAccountId::get() {
			None => drop(amount),
			Some(receiver) => {
				let result = <Pallet<T> as fungibles::Balanced<_>>::resolve(&receiver, amount);
				debug_assert!(result.is_ok());
			}
		}
	}
}

pub struct Combiner<AccountId, TestKey, A, B>(sp_std::marker::PhantomData<(AccountId, TestKey, A, B)>);

impl<AccountId, TestKey, A, B> fungibles::Inspect<AccountId> for Combiner<AccountId, TestKey, A, B>
where
	TestKey: Contains<<B as fungibles::Inspect<AccountId>>::AssetId>,
	A: fungible::Inspect<AccountId, Balance = <B as fungibles::Inspect<AccountId>>::Balance>,
	B: fungibles::Inspect<AccountId>,
{
	type AssetId = <B as fungibles::Inspect<AccountId>>::AssetId;
	type Balance = <B as fungibles::Inspect<AccountId>>::Balance;

	fn total_issuance(asset: Self::AssetId) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::total_issuance()
		} else {
			B::total_issuance(asset)
		}
	}

	fn minimum_balance(asset: Self::AssetId) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::minimum_balance()
		} else {
			B::minimum_balance(asset)
		}
	}

	fn balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::balance(who)
		} else {
			B::balance(asset, who)
		}
	}

	fn total_balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::total_balance(who)
		} else {
			B::total_balance(asset, who)
		}
	}

	fn reducible_balance(
		asset: Self::AssetId,
		who: &AccountId,
		preservation: Preservation,
		fortitude: Fortitude,
	) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::reducible_balance(who, preservation, fortitude)
		} else {
			B::reducible_balance(asset, who, preservation, fortitude)
		}
	}

	fn can_deposit(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
		provenance: Provenance,
	) -> DepositConsequence {
		if TestKey::contains(&asset) {
			A::can_deposit(who, amount, provenance)
		} else {
			B::can_deposit(asset, who, amount, provenance)
		}
	}

	fn can_withdraw(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		if TestKey::contains(&asset) {
			A::can_withdraw(who, amount)
		} else {
			B::can_withdraw(asset, who, amount)
		}
	}

	fn asset_exists(asset: Self::AssetId) -> bool {
		if TestKey::contains(&asset) {
			true
		} else {
			B::asset_exists(asset)
		}
	}
}

impl<AccountId, TestKey, A, B> fungibles::Mutate<AccountId> for Combiner<AccountId, TestKey, A, B>
where
	TestKey: Contains<<B as fungibles::Inspect<AccountId>>::AssetId>,
	A: fungible::Mutate<AccountId, Balance = <B as fungibles::Inspect<AccountId>>::Balance>,
	B: fungibles::Mutate<AccountId>,
{
	fn mint_into(
		asset: Self::AssetId,
		dest: &AccountId,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError> {
		if TestKey::contains(&asset) {
			A::mint_into(dest, amount)
		} else {
			B::mint_into(asset, dest, amount)
		}
	}

	fn burn_from(
		asset: Self::AssetId,
		dest: &AccountId,
		amount: Self::Balance,
		precision: Precision,
		fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		if TestKey::contains(&asset) {
			A::burn_from(dest, amount, precision, fortitude)
		} else {
			B::burn_from(asset, dest, amount, precision, fortitude)
		}
	}

	fn transfer(
		asset: Self::AssetId,
		source: &AccountId,
		dest: &AccountId,
		amount: Self::Balance,
		preservation: Preservation,
	) -> Result<Self::Balance, DispatchError> {
		if TestKey::contains(&asset) {
			A::transfer(source, dest, amount, preservation)
		} else {
			B::transfer(asset, source, dest, amount, preservation)
		}
	}
}

impl<AccountId, TestKey, A, B> fungibles::Unbalanced<AccountId> for Combiner<AccountId, TestKey, A, B>
where
	TestKey: Contains<<B as fungibles::Inspect<AccountId>>::AssetId>,
	A: fungible::Mutate<AccountId, Balance = <B as fungibles::Inspect<AccountId>>::Balance>,
	B: fungibles::Mutate<AccountId>,
{
	fn handle_dust(dust: fungibles::Dust<AccountId, Self>) {
		let asset = dust.0;
		let dust_amount = dust.1;
		if TestKey::contains(&asset) {
			let fungible_dust = fungible::Dust::<AccountId, A>(dust_amount);
			A::handle_dust(fungible_dust)
		} else {
			let fungibles_dust = fungibles::Dust::<AccountId, B>(asset, dust_amount);
			B::handle_dust(fungibles_dust)
		}
	}

	fn write_balance(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> Result<Option<Self::Balance>, DispatchError> {
		if TestKey::contains(&asset) {
			A::write_balance(who, amount)
		} else {
			B::write_balance(asset, who, amount)
		}
	}

	/// NOTE: this impl overrides the default implementation of
	/// fungibles::Unbalanced, because orml-tokens override the default the
	/// implementation of fungibles::Unbalanced. Here override for consistency.
	fn increase_balance(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
		precision: Precision,
	) -> Result<Self::Balance, DispatchError> {
		if TestKey::contains(&asset) {
			A::increase_balance(who, amount, precision)
		} else {
			B::increase_balance(asset, who, amount, precision)
		}
	}

	fn set_total_issuance(asset: Self::AssetId, amount: Self::Balance) {
		if TestKey::contains(&asset) {
			A::set_total_issuance(amount)
		} else {
			B::set_total_issuance(asset, amount)
		}
	}
}

pub trait ConvertBalance<A: Bounded, B: Bounded> {
	type AssetId;
	fn convert_balance(amount: A, asset_id: Self::AssetId) -> Result<B, ArithmeticError>;
	fn convert_balance_back(amount: B, asset_id: Self::AssetId) -> Result<A, ArithmeticError>;

	fn convert_balance_saturated(amount: A, asset_id: Self::AssetId) -> B {
		Self::convert_balance(amount, asset_id).unwrap_or_else(|e| match e {
			ArithmeticError::Overflow => B::max_value(),
			ArithmeticError::Underflow => B::min_value(),
			ArithmeticError::DivisionByZero => B::max_value(),
		})
	}
	fn convert_balance_back_saturated(amount: B, asset_id: Self::AssetId) -> A {
		Self::convert_balance_back(amount, asset_id).unwrap_or_else(|e| match e {
			ArithmeticError::Overflow => A::max_value(),
			ArithmeticError::Underflow => A::min_value(),
			ArithmeticError::DivisionByZero => A::max_value(),
		})
	}
}

pub struct Mapper<AccountId, T, C, B, GetCurrencyId>(sp_std::marker::PhantomData<(AccountId, T, C, B, GetCurrencyId)>);
impl<AccountId, T, C, B, GetCurrencyId> fungible::Inspect<AccountId> for Mapper<AccountId, T, C, B, GetCurrencyId>
where
	T: fungibles::Inspect<AccountId>,
	C: ConvertBalance<
		<T as fungibles::Inspect<AccountId>>::Balance,
		B,
		AssetId = <T as fungibles::Inspect<AccountId>>::AssetId,
	>,
	B: BalanceT,
	GetCurrencyId: Get<<T as fungibles::Inspect<AccountId>>::AssetId>,
{
	type Balance = B;

	fn total_issuance() -> Self::Balance {
		C::convert_balance_saturated(T::total_issuance(GetCurrencyId::get()), GetCurrencyId::get())
	}

	fn minimum_balance() -> Self::Balance {
		C::convert_balance_saturated(T::minimum_balance(GetCurrencyId::get()), GetCurrencyId::get())
	}

	fn balance(who: &AccountId) -> Self::Balance {
		C::convert_balance_saturated(T::balance(GetCurrencyId::get(), who), GetCurrencyId::get())
	}

	fn total_balance(who: &AccountId) -> Self::Balance {
		C::convert_balance_saturated(T::total_balance(GetCurrencyId::get(), who), GetCurrencyId::get())
	}

	fn reducible_balance(who: &AccountId, preservation: Preservation, fortitude: Fortitude) -> Self::Balance {
		C::convert_balance_saturated(
			T::reducible_balance(GetCurrencyId::get(), who, preservation, fortitude),
			GetCurrencyId::get(),
		)
	}

	fn can_deposit(who: &AccountId, amount: Self::Balance, provenance: Provenance) -> DepositConsequence {
		let amount = C::convert_balance_back(amount, GetCurrencyId::get());
		let amount = match amount {
			Ok(amount) => amount,
			Err(_) => return DepositConsequence::Overflow,
		};
		T::can_deposit(GetCurrencyId::get(), who, amount, provenance)
	}

	fn can_withdraw(who: &AccountId, amount: Self::Balance) -> WithdrawConsequence<Self::Balance> {
		use WithdrawConsequence::*;

		let amount = C::convert_balance_back(amount, GetCurrencyId::get());
		let amount = match amount {
			Ok(amount) => amount,
			Err(ArithmeticError::Overflow) => return Overflow,
			Err(ArithmeticError::Underflow) => return Underflow,
			Err(ArithmeticError::DivisionByZero) => return Overflow,
		};

		let res = T::can_withdraw(GetCurrencyId::get(), who, amount);
		match res {
			WithdrawConsequence::ReducedToZero(b) => {
				WithdrawConsequence::ReducedToZero(C::convert_balance_saturated(b, GetCurrencyId::get()))
			}
			BalanceLow => BalanceLow,
			WouldDie => WouldDie,
			UnknownAsset => UnknownAsset,
			Underflow => Underflow,
			Overflow => Overflow,
			Frozen => Frozen,
			Success => Success,
		}
	}
}

impl<AccountId, T, C, B, GetCurrencyId> fungible::Mutate<AccountId> for Mapper<AccountId, T, C, B, GetCurrencyId>
where
	T: fungibles::Mutate<AccountId, Balance = B>,
	C: ConvertBalance<
		<T as fungibles::Inspect<AccountId>>::Balance,
		B,
		AssetId = <T as fungibles::Inspect<AccountId>>::AssetId,
	>,
	B: BalanceT,
	GetCurrencyId: Get<<T as fungibles::Inspect<AccountId>>::AssetId>,
{
	fn mint_into(dest: &AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError> {
		T::mint_into(
			GetCurrencyId::get(),
			dest,
			C::convert_balance_back(amount, GetCurrencyId::get())?,
		)
	}

	fn burn_from(
		dest: &AccountId,
		amount: Self::Balance,
		precision: Precision,
		fortitude: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		T::burn_from(
			GetCurrencyId::get(),
			dest,
			C::convert_balance_back(amount, GetCurrencyId::get())?,
			precision,
			fortitude,
		)
	}

	fn transfer(
		source: &AccountId,
		dest: &AccountId,
		amount: B,
		preservation: Preservation,
	) -> Result<B, DispatchError> {
		T::transfer(
			GetCurrencyId::get(),
			source,
			dest,
			C::convert_balance_back(amount, GetCurrencyId::get())?,
			preservation,
		)
	}
}

impl<AccountId, T, C, B, GetCurrencyId> fungible::Unbalanced<AccountId> for Mapper<AccountId, T, C, B, GetCurrencyId>
where
	T: fungibles::Unbalanced<AccountId, Balance = B>,
	C: ConvertBalance<
		<T as fungibles::Inspect<AccountId>>::Balance,
		B,
		AssetId = <T as fungibles::Inspect<AccountId>>::AssetId,
	>,
	B: BalanceT,
	GetCurrencyId: Get<<T as fungibles::Inspect<AccountId>>::AssetId>,
{
	fn handle_dust(dust: fungible::Dust<AccountId, Self>) {
		let dust_amount = dust.0;
		let asset = GetCurrencyId::get();
		let fungibles_dust = fungibles::Dust::<AccountId, T>(asset, dust_amount);
		T::handle_dust(fungibles_dust)
	}

	fn write_balance(who: &AccountId, amount: Self::Balance) -> Result<Option<Self::Balance>, DispatchError> {
		T::write_balance(GetCurrencyId::get(), who, amount)
	}

	/// NOTE: this impl overrides the default implementation of
	/// fungible::Unbalanced, because orml-tokens override the default the
	/// implementation of fungibles::Unbalanced. Here override for consistency.
	fn increase_balance(
		who: &AccountId,
		amount: Self::Balance,
		precision: Precision,
	) -> Result<Self::Balance, DispatchError> {
		T::increase_balance(GetCurrencyId::get(), who, amount, precision)
	}

	fn set_total_issuance(amount: Self::Balance) {
		T::set_total_issuance(GetCurrencyId::get(), amount)
	}
}

// The adapter for specific token, which implements the
// frame_support::traits::Currency traits, orml Currency traits and fungible
// traits.
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
		<T::CurrencyHooks as MutationHooks<T::AccountId, T::CurrencyId, T::Balance>>::OnSlash::on_slash(
			currency_id,
			who,
			value,
		);
		let (actual, remaining_slash) =
			Pallet::<T>::mutate_account_handling_dust(currency_id, who, |account| -> (Self::Balance, Self::Balance) {
				let free_slashed_amount = account.free.min(value);
				account.free = account.free.defensive_saturating_sub(free_slashed_amount);

				Pallet::<T>::deposit_event(Event::Slashed {
					currency_id,
					who: who.clone(),
					free_amount: free_slashed_amount,
					reserved_amount: Zero::zero(),
				});

				(free_slashed_amount, value.saturating_sub(free_slashed_amount))
			});

		(Self::NegativeImbalance::new(actual), remaining_slash)
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
		Pallet::<T>::try_mutate_account_handling_dust(
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
				ensure!(value >= ed || T::DustRemovalWhitelist::contains(who) || !is_new, ());

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
		status: BalanceStatus,
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
		status: BalanceStatus,
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
	fn done_mint_into(who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Deposited {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_burn_from(who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Withdrawn {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_shelve(who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Withdrawn {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_restore(who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Deposited {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_transfer(source: &T::AccountId, dest: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Transfer {
			currency_id: GetCurrencyId::get(),
			from: source.clone(),
			to: dest.clone(),
			amount: amount,
		});
	}
}

impl<T, GetCurrencyId> fungible::Unbalanced<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn handle_dust(dust: fungible::Dust<T::AccountId, Self>) {
		let dust_amount = dust.0;
		let asset = GetCurrencyId::get();
		let fungibles_dust = fungibles::Dust::<T::AccountId, Pallet<T>>(asset, dust_amount);
		<Pallet<T> as fungibles::Unbalanced<_>>::handle_dust(fungibles_dust)
	}

	fn write_balance(who: &T::AccountId, amount: Self::Balance) -> Result<Option<Self::Balance>, DispatchError> {
		<Pallet<T> as fungibles::Unbalanced<_>>::write_balance(GetCurrencyId::get(), who, amount)
	}

	fn set_total_issuance(amount: Self::Balance) {
		<Pallet<T> as fungibles::Unbalanced<_>>::set_total_issuance(GetCurrencyId::get(), amount)
	}

	/// NOTE: this impl overrides the default implementation of
	/// fungible::Unbalanced, because orml-tokens override the default the
	/// implementation of fungibles::Unbalanced. Here override for consistency.
	fn increase_balance(
		who: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
	) -> Result<Self::Balance, DispatchError> {
		<Pallet<T> as fungibles::Unbalanced<_>>::increase_balance(GetCurrencyId::get(), who, amount, precision)
	}
}

impl<T, GetCurrencyId> fungible::Balanced<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	type OnDropCredit = fungible::DecreaseIssuance<T::AccountId, Self>;
	type OnDropDebt = fungible::IncreaseIssuance<T::AccountId, Self>;

	fn done_deposit(who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Deposited {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_withdraw(who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Withdrawn {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount: amount,
		});
	}

	fn done_issue(amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::Issued {
			currency_id: GetCurrencyId::get(),
			amount,
		});
	}

	fn done_rescind(amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::Rescinded {
			currency_id: GetCurrencyId::get(),
			amount,
		});
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
	fn done_hold(_reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Reserved {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount,
		});
	}

	fn done_release(_reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Unreserved {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			amount,
		});
	}

	fn done_burn_held(_reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Slashed {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			free_amount: Zero::zero(),
			reserved_amount: amount,
		});
	}

	fn done_transfer_on_hold(
		_reason: &Self::Reason,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
	) {
		// TODO: fungibles::MutateHold::transfer_on_hold did not pass the mode to this
		// hook, use `BalanceStatus::Reserved` temporarily, need to fix it
		Pallet::<T>::deposit_event(Event::<T>::ReserveRepatriated {
			currency_id: GetCurrencyId::get(),
			from: source.clone(),
			to: dest.clone(),
			amount: amount,
			status: BalanceStatus::Reserved,
		});
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

impl<T, GetCurrencyId> fungible::BalancedHold<T::AccountId> for CurrencyAdapter<T, GetCurrencyId>
where
	T: Config,
	GetCurrencyId: Get<T::CurrencyId>,
{
	fn done_slash(_reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) {
		Pallet::<T>::deposit_event(Event::<T>::Slashed {
			currency_id: GetCurrencyId::get(),
			who: who.clone(),
			free_amount: amount,
			reserved_amount: Zero::zero(),
		});
	}
}
