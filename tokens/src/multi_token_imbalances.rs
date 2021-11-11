// wrapping these imbalances in a private module is necessary to ensure absolute
// privacy of the inner member.
use crate::{TotalIssuance, Config};
use frame_support::traits::{Imbalance, SameOrOther, TryDrop};
use sp_runtime::traits::{Saturating, Zero};
use sp_std::{mem, result};

pub trait MultiTokenImbalanceWithZeroTrait<CurrencyId> {
	fn from_zero(currency_id: CurrencyId) -> Self;
}

impl<T: Config> MultiTokenImbalanceWithZeroTrait<T::CurrencyId> for PositiveImbalance<T> {
	fn from_zero(currency_id: T::CurrencyId) -> Self {
		Self::zero(currency_id)
	}
}

impl<T: Config> MultiTokenImbalanceWithZeroTrait<T::CurrencyId> for NegativeImbalance<T> {
	fn from_zero(currency_id: T::CurrencyId) -> Self {
		Self::zero(currency_id)
	}
}

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been created without any equal and opposite
/// accounting.
#[must_use]
pub struct PositiveImbalance<T: Config>(T::CurrencyId, T::Balance);

impl<T: Config> PositiveImbalance<T> {
	/// Create a new positive imbalance from a balance.
	pub fn new(currency_id: T::CurrencyId, amount: T::Balance) -> Self {
		PositiveImbalance(currency_id, amount)
	}

	pub fn zero(currency_id: T::CurrencyId) -> Self {
		PositiveImbalance(currency_id, Zero::zero())
	}
}

/// Opaque, move-only struct with private fields that serves as a token
/// denoting that funds have been destroyed without any equal and opposite
/// accounting.
#[must_use]
pub struct NegativeImbalance<T: Config>(pub T::CurrencyId, T::Balance);

impl<T: Config> NegativeImbalance<T> {
	/// Create a new negative imbalance from a balance.
	pub fn new(currency_id: T::CurrencyId, amount: T::Balance) -> Self {
		NegativeImbalance(currency_id, amount)
	}

	pub fn zero(currency_id: T::CurrencyId) -> Self {
		NegativeImbalance(currency_id, Zero::zero())
	}
}

impl<T: Config> TryDrop for PositiveImbalance<T> {
	fn try_drop(self) -> result::Result<(), Self> {
		self.drop_zero()
	}
}

impl<T: Config> Imbalance<T::Balance> for PositiveImbalance<T> {
	type Opposite = NegativeImbalance<T>;

	fn zero() -> Self {
		unimplemented!("PositiveImbalance::zero is not implemented");
	}

	fn drop_zero(self) -> result::Result<(), Self> {
		if self.1.is_zero() {
			Ok(())
		} else {
			Err(self)
		}
	}
	fn split(self, amount: T::Balance) -> (Self, Self) {
		let first = self.1.min(amount);
		let second = self.1 - first;
		let currency_id = self.0;

		mem::forget(self);
		(Self::new(currency_id, first), Self::new(currency_id, second))
	}
	fn merge(mut self, other: Self) -> Self {
		assert_eq!(self.0, other.0);
		self.1 = self.1.saturating_add(other.1);
		mem::forget(other);
		self
	}
	fn subsume(&mut self, other: Self) {
		assert_eq!(self.0, other.0);
		self.1 = self.1.saturating_add(other.1);
		mem::forget(other);
	}
	// allow to make the impl same with `pallet-balances`
	#[allow(clippy::comparison_chain)]
	fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
		assert_eq!(self.0, other.0);
		let (a, b) = (self.1, other.1);
		let currency_id = self.0;
		mem::forget((self, other));

		if if a > b {
			SameOrOther::Same(Self::new(currency_id, a - b))
		} else if b > a {
			SameOrOther::Other(NegativeImbalance::new(currency_id, b - a))
		} else {
			SameOrOther::None
		}
	}
	fn peek(&self) -> T::Balance {
		self.1
	}
}

impl<T: Config> TryDrop for NegativeImbalance<T> {
	fn try_drop(self) -> result::Result<(), Self> {
		self.drop_zero()
	}
}

impl<T: Config> Imbalance<T::Balance> for NegativeImbalance<T> {
	type Opposite = PositiveImbalance<T>;

	fn zero() -> Self {
		unimplemented!("NegativeImbalance::zero is not implemented");
	}
	fn drop_zero(self) -> result::Result<(), Self> {
		if self.1.is_zero() {
			Ok(())
		} else {
			Err(self)
		}
	}
	fn split(self, amount: T::Balance) -> (Self, Self) {
		let first = self.1.min(amount);
		let second = self.1 - first;
		let currency_id = self.0;

		mem::forget(self);
		(Self::new(currency_id, first), Self::new(currency_id, second))
	}
	fn merge(mut self, other: Self) -> Self {
		assert_eq!(self.0, other.0);
		self.1 = self.1.saturating_add(other.1);
		mem::forget(other);
		self
	}
	fn subsume(&mut self, other: Self) {
		assert_eq!(self.0, other.0);
		self.1 = self.1.saturating_add(other.1);
		mem::forget(other);
	}
	// allow to make the impl same with `pallet-balances`
	#[allow(clippy::comparison_chain)]
	fn offset(self, other: Self::Opposite) -> SameOrOther<Self, Self::Opposite> {
		assert_eq!(self.0, other.0);
		let (a, b) = (self.1, other.1);
		let currency_id = self.0;
		mem::forget((self, other));
		if a > b {
			SameOrOther::Same(Self::new(currency_id, a - b))
		} else if b > a {
			SameOrOther::Other(PositiveImbalance::new(currency_id, b - a))
		} else {
			SameOrOther::None
		}
	}
	fn peek(&self) -> T::Balance {
		self.1
	}
}

impl<T: Config> Drop for PositiveImbalance<T> {
	/// Basic drop handler will just square up the total issuance.
	fn drop(&mut self) {
		<TotalIssuance<T>>::mutate(self.0, |v| *v = v.saturating_add(self.1));
	}
}

impl<T: Config> Drop for NegativeImbalance<T> {
	/// Basic drop handler will just square up the total issuance.
	fn drop(&mut self) {
		<TotalIssuance<T>>::mutate(self.0, |v| *v = v.saturating_sub(self.1));
	}
}
