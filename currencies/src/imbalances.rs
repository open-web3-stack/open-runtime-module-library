// wrapping these imbalances in a private module is necessary to ensure absolute
// privacy of the inner member.
pub mod imbalances {
	use crate::{BalanceOf, Trait};
	use frame_support::traits::{Imbalance, TryDrop};
	use sp_runtime::traits::{Saturating, Zero};
	use sp_std::mem;
	use sp_std::result;

	/// Opaque, move-only struct with private fields that serves as a token
	/// denoting that funds have been created without any equal and opposite
	/// accounting.
	#[must_use]
	pub struct PositiveImbalance<T: Trait>(BalanceOf<T>);

	impl<T: Trait> PositiveImbalance<T> {
		/// Create a new positive imbalance from a balance.
		pub fn new(amount: BalanceOf<T>) -> Self {
			PositiveImbalance(amount)
		}
	}

	/// Opaque, move-only struct with private fields that serves as a token
	/// denoting that funds have been destroyed without any equal and opposite
	/// accounting.
	#[must_use]
	pub struct NegativeImbalance<T: Trait>(BalanceOf<T>);

	impl<T: Trait> NegativeImbalance<T> {
		/// Create a new negative imbalance from a balance.
		pub fn new(amount: BalanceOf<T>) -> Self {
			NegativeImbalance(amount)
		}
	}

	impl<T: Trait> TryDrop for PositiveImbalance<T> {
		fn try_drop(self) -> result::Result<(), Self> {
			self.drop_zero()
		}
	}

	impl<T: Trait> Imbalance<BalanceOf<T>> for PositiveImbalance<T> {
		type Opposite = NegativeImbalance<T>;

		fn zero() -> Self {
			Self(Zero::zero())
		}
		fn drop_zero(self) -> result::Result<(), Self> {
			if self.0.is_zero() {
				Ok(())
			} else {
				Err(self)
			}
		}
		fn split(self, amount: BalanceOf<T>) -> (Self, Self) {
			let first = self.0.min(amount);
			let second = self.0 - first;

			mem::forget(self);
			(Self(first), Self(second))
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
		fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
			let (a, b) = (self.0, other.0);
			mem::forget((self, other));

			if a >= b {
				Ok(Self(a - b))
			} else {
				Err(NegativeImbalance::new(b - a))
			}
		}
		fn peek(&self) -> BalanceOf<T> {
			self.0.clone()
		}
	}

	impl<T: Trait> TryDrop for NegativeImbalance<T> {
		fn try_drop(self) -> result::Result<(), Self> {
			self.drop_zero()
		}
	}

	impl<T: Trait> Imbalance<BalanceOf<T>> for NegativeImbalance<T> {
		type Opposite = PositiveImbalance<T>;

		fn zero() -> Self {
			Self(Zero::zero())
		}
		fn drop_zero(self) -> result::Result<(), Self> {
			if self.0.is_zero() {
				Ok(())
			} else {
				Err(self)
			}
		}
		fn split(self, amount: BalanceOf<T>) -> (Self, Self) {
			let first = self.0.min(amount);
			let second = self.0 - first;

			mem::forget(self);
			(Self(first), Self(second))
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
		fn offset(self, other: Self::Opposite) -> result::Result<Self, Self::Opposite> {
			let (a, b) = (self.0, other.0);
			mem::forget((self, other));

			if a >= b {
				Ok(Self(a - b))
			} else {
				Err(PositiveImbalance::new(b - a))
			}
		}
		fn peek(&self) -> BalanceOf<T> {
			self.0.clone()
		}
	}

	impl<T: Trait> Drop for PositiveImbalance<T> {
		/// Basic drop handler will just square up the total issuance.
		fn drop(&mut self) {}
	}

	impl<T: Trait> Drop for NegativeImbalance<T> {
		/// Basic drop handler will just square up the total issuance.
		fn drop(&mut self) {}
	}
}
