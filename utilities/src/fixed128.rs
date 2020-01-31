use codec::{Decode, Encode};
use primitives::U256;
use rstd::convert::{Into, TryFrom, TryInto};
use sp_runtime::{
	traits::{Bounded, Saturating, UniqueSaturatedInto},
	Perbill, Percent, Permill, Perquintill,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize, Serializer};

/// An unsigned fixed point number. Can hold any value in the range [0, 340_282_366_920_938_463_464]
/// with fixed point accuracy of 10 ** 18.
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Deserialize))]
pub struct FixedU128(u128);

const DIV: u128 = 1_000_000_000_000_000_000;

impl FixedU128 {
	/// Create self from a natural number.
	///
	/// Note that this might be lossy.
	pub fn from_natural(int: u128) -> Self {
		Self(int.saturating_mul(DIV))
	}

	/// Accuracy of `FixedU128`.
	pub const fn accuracy() -> u128 {
		DIV
	}

	/// Raw constructor. Equal to `parts / DIV`.
	pub fn from_parts(parts: u128) -> Self {
		Self(parts)
	}

	/// Creates self from a rational number. Equal to `n/d`.
	///
	/// Note that this might be lossy.
	pub fn from_rational<N: UniqueSaturatedInto<u128>>(n: N, d: N) -> Self {
		// this should really be `N: Into<U256>` or else might give wrong result
		// TODO: Should have a better way to enforce this requirement
		let n = n.unique_saturated_into();
		let n = U256::from(n);
		let d = d.unique_saturated_into();
		let d = U256::from(d);
		Self(
			(n.saturating_mul(DIV.into()) / d.max(U256::one()))
				.try_into()
				.unwrap_or_else(|_| Bounded::max_value()),
		)
	}

	/// Consume self and return the inner raw `u128` value.
	///
	/// Note this is a low level function, as the returned value is represented with accuracy.
	pub fn deconstruct(self) -> u128 {
		self.0
	}

	/// Takes the reciprocal(inverse) of FixedU128, 1/x
	pub fn recip(&self) -> Option<Self> {
		Self::from_natural(1u128).checked_div(self)
	}

	/// Checked add. Same semantic to `num_traits::CheckedAdd`.
	pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
		self.0.checked_add(rhs.0).map(Self)
	}

	/// Checked sub. Same semantic to `num_traits::CheckedSub`.
	pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
		self.0.checked_sub(rhs.0).map(Self)
	}

	/// Checked mul. Same semantic to `num_traits::CheckedMul`.
	pub fn checked_mul(&self, rhs: &Self) -> Option<Self> {
		if let Some(r) = U256::from(self.0)
			.checked_mul(U256::from(rhs.0))
			.and_then(|n| n.checked_div(U256::from(DIV)))
		{
			if let Ok(r) = TryInto::<u128>::try_into(r) {
				return Some(Self(r));
			}
		}

		None
	}

	/// Checked div. Same semantic to `num_traits::CheckedDiv`.
	pub fn checked_div(&self, rhs: &Self) -> Option<Self> {
		if let Some(r) = U256::from(self.0)
			.checked_mul(U256::from(DIV))
			.and_then(|n| n.checked_div(U256::from(rhs.0)))
		{
			if let Ok(r) = TryInto::<u128>::try_into(r) {
				return Some(Self(r));
			}
		}

		None
	}

	/// Checked mul for int type `N`.
	pub fn checked_mul_int<N>(&self, other: &N) -> Option<N>
	where
		N: Copy + TryFrom<u128> + TryInto<u128>,
	{
		if let Ok(n) = N::try_into(*other) {
			if let Some(n) = U256::from(self.0)
				.checked_mul(U256::from(n))
				.and_then(|n| n.checked_div(U256::from(DIV)))
			{
				if let Ok(r) = TryInto::<u128>::try_into(n) {
					if let Ok(r) = TryInto::<N>::try_into(r) {
						return Some(r);
					}
				}
			}
		}

		None
	}

	/// Checked mul for int type `N`.
	pub fn saturating_mul_int<N>(&self, other: &N) -> N
	where
		N: Copy + TryFrom<u128> + TryInto<u128> + Bounded,
	{
		self.checked_mul_int(other).unwrap_or_else(Bounded::max_value)
	}

	/// Checked div for int type `N`.
	pub fn checked_div_int<N>(&self, other: &N) -> Option<N>
	where
		N: Copy + TryFrom<u128> + TryInto<u128>,
	{
		if let Ok(n) = N::try_into(*other) {
			if let Some(n) = self.0.checked_div(n).and_then(|n| n.checked_div(DIV)) {
				if let Ok(r) = TryInto::<N>::try_into(n) {
					return Some(r);
				}
			}
		}

		None
	}

	pub fn zero() -> Self {
		Self(0)
	}

	pub fn is_zero(&self) -> bool {
		self.0 == 0
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

impl rstd::fmt::Display for FixedU128 {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut rstd::fmt::Formatter) -> rstd::fmt::Result {
		rstd::fmt::Debug::fmt(self, f)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, f: &mut rstd::fmt::Formatter) -> rstd::fmt::Result {
		rstd::fmt::Debug::fmt(self, f)
	}
}

macro_rules! impl_perthing_into_fixed_u128 {
	($perthing:ty) => {
		impl Into<FixedU128> for $perthing {
			fn into(self) -> FixedU128 {
				FixedU128::from_rational(self.deconstruct(), <$perthing>::accuracy())
			}
		}
	};
}

impl_perthing_into_fixed_u128!(Percent);
impl_perthing_into_fixed_u128!(Permill);
impl_perthing_into_fixed_u128!(Perbill);
impl_perthing_into_fixed_u128!(Perquintill);

impl FixedU128 {
	#[cfg(feature = "std")]
	fn str_with_precision(&self) -> String {
		format!("{}.{}", &self.0 / DIV, &self.0 % DIV)
	}
}

// Manual impl `Serialize` as serde_json does not support u128.
// TODO: remove impl if issue https://github.com/serde-rs/json/issues/548 fixed.
#[cfg(feature = "std")]
impl Serialize for FixedU128 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.str_with_precision())
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
		assert_eq!(a.checked_add(&b), Some(FixedU128::from_natural(1 + 2)));
		assert_eq!(a.checked_sub(&b), Some(FixedU128::from_natural(2 - 1)));
		assert_eq!(a.checked_mul(&b), Some(FixedU128::from_natural(1 * 2)));
		assert_eq!(a.checked_div(&b), Some(FixedU128::from_rational(2, 1)));

		let a = FixedU128::from_rational(5, 2);
		let b = FixedU128::from_rational(3, 2);
		assert_eq!(a.checked_add(&b), Some(FixedU128::from_rational(8, 2)));
		assert_eq!(a.checked_sub(&b), Some(FixedU128::from_rational(2, 2)));
		assert_eq!(a.checked_mul(&b), Some(FixedU128::from_rational(15, 4)));
		assert_eq!(a.checked_div(&b), Some(FixedU128::from_rational(10, 6)));

		let a = FixedU128::from_natural(120);
		let b = 2i32;
		assert_eq!(a.checked_div_int::<i32>(&b), Some(60));

		let a = FixedU128::from_rational(20, 1);
		let b = 2i32;
		assert_eq!(a.checked_div_int::<i32>(&b), Some(10));

		let a = FixedU128::from_natural(120);
		let b = 2i32;
		assert_eq!(a.checked_mul_int::<i32>(&b), Some(240));

		let a = FixedU128::from_rational(1, 2);
		let b = 20i32;
		assert_eq!(a.checked_mul_int::<i32>(&b), Some(10));
	}

	#[test]
	fn saturating_mul_int_works() {
		let a = FixedU128::from_rational(10, 1);
		let b = u32::max_value() / 5;
		assert_eq!(a.saturating_mul_int(&b), u32::max_value());

		let a = FixedU128::from_rational(3, 1);
		let b = 100u8;
		assert_eq!(a.saturating_mul_int(&b), 255u8);

		let a = FixedU128::from_rational(10, 1);
		let b = 123;
		assert_eq!(a.saturating_mul_int(&b), 1230);
	}

	#[test]
	fn zero_works() {
		assert_eq!(FixedU128::zero(), FixedU128::from_natural(0));
	}

	#[test]
	fn is_zero_works() {
		assert!(FixedU128::zero().is_zero());
		assert!(!FixedU128::from_natural(1).is_zero());
	}

	#[test]
	fn checked_div_with_zero_should_be_none() {
		let a = FixedU128::from_natural(1);
		let b = FixedU128::from_natural(0);
		assert_eq!(a.checked_div(&b), None);
	}

	#[test]
	fn checked_div_int_with_zero_should_be_none() {
		let a = FixedU128::from_natural(1);
		let b = 0i32;
		assert_eq!(a.checked_div_int(&b), None);
	}

	#[test]
	fn under_flow_should_be_none() {
		let a = FixedU128::from_natural(2);
		let b = FixedU128::from_natural(3);
		assert_eq!(a.checked_sub(&b), None);
	}

	#[test]
	fn over_flow_should_be_none() {
		let a = FixedU128::from_parts(u128::max_value() - 1);
		let b = FixedU128::from_parts(2);
		assert_eq!(a.checked_add(&b), None);

		let a = FixedU128::max_value();
		let b = FixedU128::from_rational(2, 1);
		assert_eq!(a.checked_mul(&b), None);

		let a = FixedU128::from_natural(255);
		let b = 2u8;
		assert_eq!(a.checked_mul_int(&b), None);

		let a = FixedU128::from_natural(256);
		let b = 1u8;
		assert_eq!(a.checked_div_int(&b), None);
	}

	#[test]
	fn perthing_into_fixed_u128() {
		let ten_percent_percent: FixedU128 = Percent::from_percent(10).into();
		assert_eq!(ten_percent_percent.deconstruct(), DIV / 10);

		let ten_percent_permill: FixedU128 = Permill::from_percent(10).into();
		assert_eq!(ten_percent_permill.deconstruct(), DIV / 10);

		let ten_percent_perbill: FixedU128 = Perbill::from_percent(10).into();
		assert_eq!(ten_percent_perbill.deconstruct(), DIV / 10);

		let ten_percent_perquintill: FixedU128 = Perquintill::from_percent(10).into();
		assert_eq!(ten_percent_perquintill.deconstruct(), DIV / 10);
	}

	#[test]
	fn recip_should_work() {
		let a = FixedU128::from_natural(2);
		assert_eq!(a.recip(), Some(FixedU128::from_rational(1, 2)));

		let a = FixedU128::from_natural(2);
		assert_eq!(a.recip().unwrap().checked_mul_int(&4i32), Some(2i32));

		let a = FixedU128::from_rational(100, 121);
		assert_eq!(a.recip(), Some(FixedU128::from_rational(121, 100)));

		let a = FixedU128::from_rational(1, 2);
		assert_eq!(a.recip().unwrap().checked_mul(&a), Some(FixedU128::from_natural(1)));

		let a = FixedU128::from_natural(0);
		assert_eq!(a.recip(), None);
	}
}
