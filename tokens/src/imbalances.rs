use sr_primitives::traits::Saturating;
use srml_support::StorageMap;

use super::{TotalIssuance, Trait};
use traits::{Imbalance, Rebalance};

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

pub struct PositiveImbalance<T: Trait> {
	currency_id: T::CurrencyId,
	amount: T::Balance,
}

impl<T: Trait> PositiveImbalance<T> {
	pub fn new(currency_id: T::CurrencyId, amount: T::Balance) -> Self {
		PositiveImbalance { currency_id, amount }
	}
}

impl<T: Trait> Imbalance for PositiveImbalance<T> {
	type Balance = T::Balance;
	type CurrencyId = T::CurrencyId;
	type Opposite = NegativeImbalance<T>;
	type Rebalance = RebalancePositive<T>;

	fn currency_id(&self) -> Self::CurrencyId {
		self.currency_id
	}

	fn amount(&self) -> Self::Balance {
		self.amount
	}
}

impl<T: Trait> Drop for PositiveImbalance<T> {
	fn drop(&mut self) {
		<Self as Imbalance>::Rebalance::rebalance(self.currency_id, self.amount);
	}
}

pub struct NegativeImbalance<T: Trait> {
	currency_id: T::CurrencyId,
	amount: T::Balance,
}

impl<T: Trait> NegativeImbalance<T> {
	pub fn new(currency_id: T::CurrencyId, amount: T::Balance) -> Self {
		NegativeImbalance { currency_id, amount }
	}
}

impl<T: Trait> Imbalance for NegativeImbalance<T> {
	type Balance = T::Balance;
	type CurrencyId = T::CurrencyId;
	type Opposite = PositiveImbalance<T>;
	type Rebalance = RebalanceNegative<T>;

	fn currency_id(&self) -> Self::CurrencyId {
		self.currency_id
	}

	fn amount(&self) -> Self::Balance {
		self.amount
	}
}

impl<T: Trait> Drop for NegativeImbalance<T> {
	fn drop(&mut self) {
		<Self as Imbalance>::Rebalance::rebalance(self.currency_id, self.amount);
	}
}
