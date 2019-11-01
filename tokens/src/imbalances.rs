use sr_primitives::traits::Saturating;
use srml_support::StorageMap;

use super::{TotalIssuance, Trait};
use traits::{Imbalance, Rebalance};

pub struct PositiveImbalance<T: Trait>(T::CurrencyId, T::Balance);
pub struct NegativeImbalance<T: Trait>(T::CurrencyId, T::Balance);

pub struct RebalancePositive<T>(rstd::marker::PhantomData<T>);
pub struct RebalanceNegative<T>(rstd::marker::PhantomData<T>);

impl<T: Trait> Rebalance<T::CurrencyId, T::Balance> for RebalancePositive<T> {
	fn rebalance(currency_id: T::CurrencyId, amount: T::Balance) {
		<TotalIssuance<T>>::mutate(currency_id, |v| *v = v.saturating_add(amount));
	}
}

impl<T: Trait> Rebalance<T::CurrencyId, T::Balance> for RebalanceNegative<T> {
	fn rebalance(currency_id: T::CurrencyId, amount: T::Balance) {
		<TotalIssuance<T>>::mutate(currency_id, |v| *v = v.saturating_sub(amount));
	}
}

impl<T: Trait> Imbalance for PositiveImbalance<T> {
	type Balance = T::Balance;
	type CurrencyId = T::CurrencyId;
	type Opposite = NegativeImbalance<T>;
	type Rebalance = RebalancePositive<T>;

	fn currency_id(self: &Self) -> Self::CurrencyId {
		self.0
	}

	fn amount(self: &Self) -> Self::Balance {
		self.1
	}
}

impl<T: Trait> Imbalance for NegativeImbalance<T> {
	type Balance = T::Balance;
	type CurrencyId = T::CurrencyId;
	type Opposite = PositiveImbalance<T>;
	type Rebalance = RebalanceNegative<T>;

	fn currency_id(self: &Self) -> Self::CurrencyId {
		self.0
	}

	fn amount(self: &Self) -> Self::Balance {
		self.1
	}
}

// TODO: impl `Drop`
