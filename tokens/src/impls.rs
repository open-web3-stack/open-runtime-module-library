use frame_support::traits::tokens::{Fortitude, Precision, Preservation, Provenance};
use frame_support::traits::{
	fungible, fungibles,
	tokens::{Balance as BalanceT, DepositConsequence, WithdrawConsequence},
	Contains, Get,
};
use sp_arithmetic::{traits::Bounded, ArithmeticError};
use sp_runtime::DispatchError;

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
	AccountId: Eq,
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
	AccountId: Eq,
{
	fn handle_dust(_dust: fungibles::Dust<AccountId, Self>) {
		// FIXME: only way to access internals of Dust is into_credit, but T is
		// not balanced
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
	AccountId: Eq,
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
	fn handle_dust(_dust: fungible::Dust<AccountId, Self>) {
		// FIXME: only way to access internals of Dust is into_credit, but T is
		// not balanced
	}

	fn write_balance(who: &AccountId, amount: Self::Balance) -> Result<Option<Self::Balance>, DispatchError> {
		T::write_balance(GetCurrencyId::get(), who, amount)
	}

	fn set_total_issuance(amount: Self::Balance) {
		T::set_total_issuance(GetCurrencyId::get(), amount)
	}
}
