use codec::{Decode, Encode};
use primitives::U256;
use rstd::{
	convert::{Into, TryFrom, TryInto},
	ops,
	prelude::*,
};
use sr_primitives::traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating, Zero};
/// [0 +340_282_366_920_938_463_462]

#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedU128(u128);

const DIV: u128 = 1_000_000_000_000_000_000;

impl FixedU128 {
	/// create self from a natural number
	///
	/// Note that this might be lossy
	pub fn from_natural(int: u128) -> Self {
		Self(int.saturating_mul(DIV))
	}

	pub fn accuracy() -> u128 {
		DIV
	}

	/// raw constructor. Equal to `parts / DIV`.
	pub fn from_parts(parts: u128) -> Self {
		Self(parts)
	}

	/// creates self from a rational number. Equal to `n/d`
	///
	/// Note that this might be lossy
	pub fn from_rational(n: u128, d: u128) -> Self {
		Self(
			(U256::from(n).saturating_mul(U256::from(DIV)) / U256::from(d).max(U256::one()))
				.try_into()
				.unwrap_or_else(|_| Bounded::max_value()),
		)
	}

	/// consume self and return the inner value.
	///
	/// This should only be used for testing.
	#[cfg(any(feature = "std", test))]
	pub fn into_inner(self) -> u128 {
		self.0
	}

	/// checked div for type N
	pub fn checked_div_other<N>(&self, other: &N) -> Option<N>
	where
		N: Copy + TryFrom<u128> + TryInto<u128> + Bounded + Zero,
	{
		if other.is_zero() {
			return None;
		}

		Some(*self / *other)
	}
}

impl Saturating for FixedU128 {
	fn saturating_add(self, rhs: Self) -> Self {
		Self(self.0.saturating_add(rhs.0))
	}

	fn saturating_mul(self, rhs: Self) -> Self {
		Self(
			(U256::from(self.0).saturating_mul(U256::from(rhs.0)) / U256::from(DIV))
				.try_into()
				.unwrap_or_else(|_| Bounded::max_value()),
		)
	}

	fn saturating_sub(self, rhs: Self) -> Self {
		Self(self.0.saturating_sub(rhs.0))
	}
}

impl Bounded for FixedU128 {
	fn max_value() -> Self {
		Self(u128::max_value())
	}

	fn min_value() -> Self {
		Self(0u128)
	}
}

impl ops::Add for FixedU128 {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		Self(self.0 + rhs.0)
	}
}

impl ops::Sub for FixedU128 {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Self(self.0 - rhs.0)
	}
}

/// mul self
///
/// Note that this might be lossy
impl ops::Mul for FixedU128 {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		self.saturating_mul(rhs)
	}
}

/// mul other type which can convert to u128
///
/// Note that this might be lossy
impl<N> ops::Mul<N> for FixedU128
where
	N: TryFrom<u128> + TryInto<u128> + Bounded,
{
	type Output = N;

	fn mul(self, rhs: N) -> Self::Output {
		let n: u128 = rhs.try_into().unwrap_or(u128::max_value());
		let r: Self = self.saturating_mul(Self(n));
		r.0.try_into().unwrap_or(N::max_value())
	}
}

/// div self
///
/// Note that this might be lossy
impl ops::Div for FixedU128 {
	type Output = Self;

	fn div(self, rhs: Self) -> Self::Output {
		if rhs.0 == 0 {
			let zero = 0;
			return FixedU128::from_parts(self.0 / zero);
		}
		Self::from_rational(self.0, rhs.0)
	}
}

/// div other type which can convert to u128
///
/// Note that this might be lossy
impl<N> ops::Div<N> for FixedU128
where
	N: Copy + TryFrom<u128> + TryInto<u128> + Bounded + Zero,
{
	type Output = N;

	fn div(self, rhs: N) -> Self::Output {
		let n: u128 = rhs.try_into().unwrap_or(u128::max_value());
		// will panic when n is zero
		(self.0 / n / DIV).try_into().unwrap_or(N::max_value())
	}
}

impl CheckedAdd for FixedU128 {
	fn checked_add(&self, rhs: &Self) -> Option<Self> {
		self.0.checked_add(rhs.0).map(Self)
	}
}

impl CheckedSub for FixedU128 {
	fn checked_sub(&self, rhs: &Self) -> Option<Self> {
		self.0.checked_sub(rhs.0).map(Self)
	}
}

impl CheckedMul for FixedU128 {
	fn checked_mul(&self, rhs: &Self) -> Option<Self> {
		Some(*self * *rhs)
	}
}

impl CheckedDiv for FixedU128 {
	fn checked_div(&self, rhs: &Self) -> Option<Self> {
		if rhs.0 == 0 {
			return None;
		}
		Some(*self / *rhs)
	}
}

impl rstd::fmt::Debug for FixedU128 {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut rstd::fmt::Formatter) -> rstd::fmt::Result {
		write!(f, "FixedU128({},{})", self.0 / DIV, (self.0 % DIV) / 1000)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut rstd::fmt::Formatter) -> rstd::fmt::Result {
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn max() -> FixedU128 {
		FixedU128::from_parts(u128::max_value())
	}

	#[test]
	fn fixed128_semantics() {
		assert_eq!(FixedU128::from_rational(5, 2).0, 5 * 1_000_000_000_000_000_000 / 2);
		assert_eq!(FixedU128::from_rational(5, 2), FixedU128::from_rational(10, 4));
		assert_eq!(FixedU128::from_rational(5, 0), FixedU128::from_rational(5, 1));

		// biggest value that can be created.
		assert_ne!(max(), FixedU128::from_natural(340_282_366_920_938_463_463));
		assert_eq!(max(), FixedU128::from_natural(340_282_366_920_938_463_464));
	}

	#[test]
	fn fixed128_operation() {
		let a = FixedU128::from_natural(2);
		let b = FixedU128::from_natural(1);
		assert_eq!(a + b, FixedU128::from_natural(1 + 2));
		assert_eq!(a - b, FixedU128::from_natural(2 - 1));
		assert_eq!(a / b, FixedU128::from_rational(2, 1));
		assert_eq!(a * b, FixedU128::from_natural(1 * 2));

		let a = FixedU128::from_rational(5, 2);
		let b = FixedU128::from_rational(3, 2);
		assert_eq!(a + b, FixedU128::from_rational(8, 2));
		assert_eq!(a - b, FixedU128::from_rational(2, 2));
		assert_eq!(a / b, FixedU128::from_rational(10, 6));
		assert_eq!(a * b, FixedU128::from_rational(15, 4));

		// a equals 10
		let a = FixedU128::from_natural(10);
		// 10 * 10 = 100
		assert_eq!(a * 10u128, 100u128);
		// 10 / 2 = 5
		assert_eq!(a / 2u128, 5u128);

		// a equals 0.5
		let a = FixedU128::from_parts(5 * DIV / 10);
		// 0.5 * 10 = 5
		assert_eq!(a * 10u128, 5u128);
		// 0.5 / 2 = 0
		assert_eq!(a / 2u128, 0u128);

		// a eques 1/2
		let a = FixedU128::from_rational(1, 2);
		// 1/2 * 10 = 5
		assert_eq!(a * 10u128, 5u128);
		// 1/2 / 2 = 0
		assert_eq!(a / 2u128, 0u128);

		// overflow will not panic
		let a = FixedU128::from_natural(2);
		assert_eq!(a * u8::max_value(), u8::max_value());
		assert_eq!(a * u32::max_value(), u32::max_value());
		assert_eq!(a * u64::max_value(), u64::max_value());
	}

	#[test]
	#[should_panic(expected = "attempt to divide by zero")]
	fn divide_fixed_u128_0_should_panic() {
		let a = FixedU128::from_natural(1);
		let _r = a / FixedU128::from_natural(0);
	}

	#[test]
	#[should_panic(expected = "attempt to divide by zero")]
	fn divide_0_should_panic() {
		let a = FixedU128::from_natural(1);
		let _r = a / 0i128;
	}

	#[test]
	fn checked_div_should_work() {
		let a = FixedU128::from_rational(1, 2);
		let b = FixedU128::from_natural(0);
		assert_eq!(a.checked_div(&b), None);

		let a = FixedU128::from_rational(1, 2);
		let b = FixedU128::from_rational(2, 1);
		assert_eq!(a.checked_div(&b), Some(FixedU128::from_rational(1, 4)));
	}

	#[test]
	fn checked_div_other_should_work() {
		let a = FixedU128::from_rational(1, 2);
		let b = 0i32;
		assert_eq!(a.checked_div_other::<i32>(&b), None);

		let a = FixedU128::from_rational(2, 1);
		let b = 1i32;
		assert_eq!(a.checked_div_other::<i32>(&b), Some(2));
	}
}
