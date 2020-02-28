use codec::{Decode, Encode};
use primitives::U256;
use rstd::convert::{Into, TryFrom, TryInto};
use sp_runtime::{
	traits::{Bounded, Saturating, UniqueSaturatedInto},
	PerThing,
};

#[cfg(feature = "std")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sp_runtime::traits::SaturatedConversion;

/// A signed fixed-point number. Can hold any value in the range [-170_141_183_460_469_231_731, 170_141_183_460_469_231_731]
/// with fixed-point accuracy of 10 ** 18.
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed128(i128);

const DIV: i128 = 1_000_000_000_000_000_000;

impl Fixed128 {
	/// Create self from a natural number.
	///
	/// Note that this might be lossy.
	pub fn from_natural(int: i128) -> Self {
		Self(int.saturating_mul(DIV))
	}

	/// Accuracy of `Fixed128`.
	pub const fn accuracy() -> i128 {
		DIV
	}

	/// Raw constructor. Equal to `parts / DIV`.
	pub fn from_parts(parts: i128) -> Self {
		Self(parts)
	}

	/// Creates self from a rational number. Equal to `n/d`.
	///
	/// Note that this might be lossy.
	pub fn from_rational<N: UniqueSaturatedInto<i128>>(n: N, d: u128) -> Self {
		// TODO: Should have a better way to enforce this requirement
		let n = n.unique_saturated_into();
		let d: i128 = d.unique_saturated_into();
		Self(
			(n.saturating_mul(DIV.into()) / d.max(1))
				.try_into()
				.unwrap_or_else(|_| {
					if n.is_negative() {
						Bounded::min_value()
					} else {
						Bounded::max_value()
					}
				}),
		)
	}

	/// Consume self and return the inner raw `i128` value.
	///
	/// Note this is a low level function, as the returned value is represented with accuracy.
	pub fn deconstruct(self) -> i128 {
		self.0
	}

	/// Takes the reciprocal(inverse) of Fixed128, 1/x
	pub fn recip(&self) -> Option<Self> {
		Self::from_natural(1i128).checked_div(self)
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
		let signum = self.0.signum() * rhs.0.signum();
		let mut lhs = self.0;
		if lhs.is_negative() {
			lhs = lhs.saturating_mul(-1);
		}
		let mut rhs: i128 = rhs.0.saturated_into();
		if rhs.is_negative() {
			rhs = rhs.saturating_mul(-1);
		}
		if let Some(r) = U256::from(lhs)
			.checked_mul(U256::from(rhs))
			.and_then(|n| n.checked_div(U256::from(DIV)))
		{
			if let Ok(r) = TryInto::<i128>::try_into(r) {
				return Some(Self(r * signum));
			}
		}

		None
	}

	/// Checked div. Same semantic to `num_traits::CheckedDiv`.
	pub fn checked_div(&self, rhs: &Self) -> Option<Self> {
		if rhs.0.signum() == 0 {
			return None;
		}
		let signum = self.0.signum() / rhs.0.signum();
		let mut lhs: i128 = self.0;
		if lhs.is_negative() {
			lhs = lhs.saturating_mul(-1);
		}
		let mut rhs: i128 = rhs.0.saturated_into();
		if rhs.is_negative() {
			rhs = rhs.saturating_mul(-1);
		}
		U256::from(lhs)
			.checked_mul(U256::from(DIV))
			.and_then(|n| n.checked_div(U256::from(rhs)))
			.and_then(|n| TryInto::<i128>::try_into(n).ok())
			.map(|n| Self(n / signum))
	}

	/// Checked mul for int type `N`.
	pub fn checked_mul_int<N>(&self, other: &N) -> Option<N>
	where
		N: Copy + TryFrom<i128> + TryInto<i128>,
	{
		N::try_into(*other).ok().and_then(|rhs| {
			let mut lhs = self.0;
			if lhs.is_negative() {
				lhs = lhs.saturating_mul(-1);
			}
			let mut rhs: i128 = rhs.saturated_into();
			let signum = self.0.signum() * rhs.signum();
			if rhs.is_negative() {
				rhs = rhs.saturating_mul(-1);
			}

			U256::from(lhs)
				.checked_mul(U256::from(rhs))
				.and_then(|n| n.checked_div(U256::from(DIV)))
				.and_then(|n| TryInto::<i128>::try_into(n).ok())
				.and_then(|n| TryInto::<N>::try_into(n * signum).ok())
		})
	}

	/// Checked mul for int type `N`.
	pub fn saturating_mul_int<N>(&self, other: &N) -> N
	where
		N: Copy + TryFrom<i128> + TryInto<i128> + Bounded,
	{
		self.checked_mul_int(other).unwrap_or_else(|| {
			N::try_into(*other)
				.map(|n| n.signum())
				.map(|n| n * self.0.signum())
				.map(|signum| {
					if signum.is_negative() {
						Bounded::min_value()
					} else {
						Bounded::max_value()
					}
				})
				.unwrap_or(Bounded::max_value())
		})
	}

	/// Checked div for int type `N`.
	pub fn checked_div_int<N>(&self, other: &N) -> Option<N>
	where
		N: Copy + TryFrom<i128> + TryInto<i128>,
	{
		N::try_into(*other)
			.ok()
			.and_then(|n| self.0.checked_div(n))
			.and_then(|n| n.checked_div(DIV))
			.and_then(|n| TryInto::<N>::try_into(n).ok())
	}

	pub fn zero() -> Self {
		Self(0)
	}

	pub fn is_zero(&self) -> bool {
		self.0 == 0
	}
}

impl Saturating for Fixed128 {
	fn saturating_add(self, rhs: Self) -> Self {
		Self(self.0.saturating_add(rhs.0))
	}

	fn saturating_sub(self, rhs: Self) -> Self {
		Self(self.0.saturating_sub(rhs.0))
	}

	fn saturating_mul(self, rhs: Self) -> Self {
		Self(self.0.saturating_mul(rhs.0) / DIV)
	}
}

impl Bounded for Fixed128 {
	fn min_value() -> Self {
		Self(Bounded::min_value())
	}

	fn max_value() -> Self {
		Self(Bounded::max_value())
	}
}

impl rstd::fmt::Debug for Fixed128 {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut rstd::fmt::Formatter) -> rstd::fmt::Result {
		write!(f, "Fixed128({},{})", self.0 / DIV, (self.0 % DIV) / 1000)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut rstd::fmt::Formatter) -> rstd::fmt::Result {
		Ok(())
	}
}

impl<P: PerThing> From<P> for Fixed128 {
	fn from(val: P) -> Self {
		let accuracy = P::ACCURACY.saturated_into() as u128;
		let value = val.deconstruct().saturated_into() as i128;
		Fixed128::from_rational(value, accuracy)
	}
}

#[cfg(feature = "std")]
impl Fixed128 {
	fn i128_str(&self) -> String {
		format!("{}", &self.0)
	}

	fn try_from_i128_str(s: &str) -> Result<Self, &'static str> {
		let parts: i128 = s.parse().map_err(|_| "invalid string input")?;
		Ok(Self::from_parts(parts))
	}
}

// Manual impl `Serialize` as serde_json does not support i128.
// TODO: remove impl if issue https://github.com/serde-rs/json/issues/548 fixed.
#[cfg(feature = "std")]
impl Serialize for Fixed128 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.i128_str())
	}
}

// Manual impl `Serialize` as serde_json does not support i128.
// TODO: remove impl if issue https://github.com/serde-rs/json/issues/548 fixed.
#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for Fixed128 {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		Fixed128::try_from_i128_str(&s).map_err(|err_str| de::Error::custom(err_str))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_runtime::{Perbill, Percent, Permill, Perquintill};

	fn max() -> Fixed128 {
		Fixed128::max_value()
	}

	fn min() -> Fixed128 {
		Fixed128::min_value()
	}

	#[test]
	fn fixed128_semantics() {
		assert_eq!(Fixed128::from_rational(5, 2).0, 5 * 1_000_000_000_000_000_000 / 2);
		assert_eq!(Fixed128::from_rational(5, 2), Fixed128::from_rational(10, 4));
		assert_eq!(Fixed128::from_rational(5, 0), Fixed128::from_rational(5, 1));
		assert_eq!(Fixed128::from_rational(-5, 0), Fixed128::from_natural(-5));

		// biggest value that can be created.
		assert_ne!(max(), Fixed128::from_natural(170_141_183_460_469_231_731));
		assert_eq!(max(), Fixed128::from_natural(170_141_183_460_469_231_732));

		// the smallest value that can be created.
		assert_ne!(min(), Fixed128::from_natural(-170_141_183_460_469_231_731));
		assert_eq!(min(), Fixed128::from_natural(-170_141_183_460_469_231_732));
	}

	#[test]
	fn fixed128_operation() {
		let a = Fixed128::from_natural(2);
		let b = Fixed128::from_natural(1);
		assert_eq!(a.checked_add(&b), Some(Fixed128::from_natural(1 + 2)));
		assert_eq!(a.checked_sub(&b), Some(Fixed128::from_natural(2 - 1)));
		assert_eq!(a.checked_mul(&b), Some(Fixed128::from_natural(1 * 2)));
		assert_eq!(a.checked_div(&b), Some(Fixed128::from_rational(2, 1)));

		let a = Fixed128::from_rational(5, 2);
		let b = Fixed128::from_rational(3, 2);
		assert_eq!(a.checked_add(&b), Some(Fixed128::from_rational(8, 2)));
		assert_eq!(a.checked_sub(&b), Some(Fixed128::from_rational(2, 2)));
		assert_eq!(a.checked_mul(&b), Some(Fixed128::from_rational(15, 4)));
		assert_eq!(a.checked_div(&b), Some(Fixed128::from_rational(10, 6)));

		let a = Fixed128::from_natural(120);
		assert_eq!(a.checked_div_int(&2i32), Some(60));

		let a = Fixed128::from_rational(20, 1);
		assert_eq!(a.checked_div_int(&2i32), Some(10));

		let a = Fixed128::from_natural(120);
		assert_eq!(a.checked_mul_int(&2i32), Some(240));

		let a = Fixed128::from_rational(1, 2);
		assert_eq!(a.checked_mul_int(&20i32), Some(10));

		let a = Fixed128::from_rational(-1, 2);
		assert_eq!(a.checked_mul_int(&20i32), Some(-10));
	}

	#[test]
	fn saturating_mul_int_works() {
		let a = Fixed128::from_rational(10, 1);
		assert_eq!(a.saturating_mul_int(&i32::max_value()), i32::max_value());

		let a = Fixed128::from_rational(-10, 1);
		assert_eq!(a.saturating_mul_int(&i32::max_value()), i32::min_value());

		let a = Fixed128::from_rational(3, 1);
		assert_eq!(a.saturating_mul_int(&100i8), i8::max_value());

		let a = Fixed128::from_rational(10, 1);
		assert_eq!(a.saturating_mul_int(&123i128), 1230);

		let a = Fixed128::from_rational(-10, 1);
		assert_eq!(a.saturating_mul_int(&123i128), -1230);

		assert_eq!(max().saturating_mul_int(&2i128), 340_282_366_920_938_463_463);

		assert_eq!(max().saturating_mul_int(&i128::min_value()), i128::min_value());

		assert_eq!(min().saturating_mul_int(&i128::max_value()), i128::min_value());

		assert_eq!(min().saturating_mul_int(&i128::min_value()), i128::max_value());
	}

	#[test]
	fn zero_works() {
		assert_eq!(Fixed128::zero(), Fixed128::from_natural(0));
	}

	#[test]
	fn is_zero_works() {
		assert!(Fixed128::zero().is_zero());
		assert!(!Fixed128::from_natural(1).is_zero());
	}

	#[test]
	fn checked_div_with_zero_should_be_none() {
		let a = Fixed128::from_natural(1);
		let b = Fixed128::from_natural(0);
		assert_eq!(a.checked_div(&b), None);
	}

	#[test]
	fn checked_div_int_with_zero_should_be_none() {
		let a = Fixed128::from_natural(1);
		assert_eq!(a.checked_div_int(&0i32), None);
	}

	#[test]
	fn under_flow_should_be_none() {
		let b = Fixed128::from_natural(1);
		assert_eq!(min().checked_sub(&b), None);
	}

	#[test]
	fn over_flow_should_be_none() {
		let a = Fixed128::from_parts(i128::max_value() - 1);
		let b = Fixed128::from_parts(2);
		assert_eq!(a.checked_add(&b), None);

		let a = Fixed128::max_value();
		let b = Fixed128::from_rational(2, 1);
		assert_eq!(a.checked_mul(&b), None);

		let a = Fixed128::from_natural(255);
		let b = 2u8;
		assert_eq!(a.checked_mul_int(&b), None);

		let a = Fixed128::from_natural(256);
		let b = 1u8;
		assert_eq!(a.checked_div_int(&b), None);

		let a = Fixed128::from_natural(256);
		let b = -1i8;
		assert_eq!(a.checked_div_int(&b), None);
	}

	#[test]
	fn checked_div_int_should_work() {
		// 256 / 10 = 25 (25.6 as int = 25)
		let a = Fixed128::from_natural(256);
		let result = a.checked_div_int(&10i128).unwrap();
		assert_eq!(result, 25);

		// 256 / 100 = 2 (2.56 as int = 2)
		let a = Fixed128::from_natural(256);
		let result = a.checked_div_int(&100i128).unwrap();
		assert_eq!(result, 2);

		// 256 / 1000 = 0 (0.256 as int = 0)
		let a = Fixed128::from_natural(256);
		let result = a.checked_div_int(&1000i128).unwrap();
		assert_eq!(result, 0);

		// 256 / -1 = -256
		let a = Fixed128::from_natural(256);
		let result = a.checked_div_int(&-1i128).unwrap();
		assert_eq!(result, -256);

		// -256 / -1 = 256
		let a = Fixed128::from_natural(-256);
		let result = a.checked_div_int(&-1i128).unwrap();
		assert_eq!(result, 256);

		// 10 / -5 = -2
		let a = Fixed128::from_rational(20, 2);
		let result = a.checked_div_int(&-5i128).unwrap();
		assert_eq!(result, -2);

		// -170_141_183_460_469_231_731 / -2 = 85_070_591_730_234_615_865
		let result = min().checked_div_int(&-2i128).unwrap();
		assert_eq!(result, 85_070_591_730_234_615_865);

		// 85_070_591_730_234_615_865 * -2 = -170_141_183_460_469_231_730
		let result = Fixed128::from_natural(result).checked_mul_int(&-2i128).unwrap();
		assert_eq!(result, -170_141_183_460_469_231_730);
	}

	#[test]
	fn perthing_into_fixed_i128() {
		let ten_percent_percent: Fixed128 = Percent::from_percent(10).into();
		assert_eq!(ten_percent_percent.deconstruct(), DIV / 10);

		let ten_percent_permill: Fixed128 = Permill::from_percent(10).into();
		assert_eq!(ten_percent_permill.deconstruct(), DIV / 10);

		let ten_percent_perbill: Fixed128 = Perbill::from_percent(10).into();
		assert_eq!(ten_percent_perbill.deconstruct(), DIV / 10);

		let ten_percent_perquintill: Fixed128 = Perquintill::from_percent(10).into();
		assert_eq!(ten_percent_perquintill.deconstruct(), DIV / 10);
	}

	#[test]
	fn recip_should_work() {
		let a = Fixed128::from_natural(2);
		assert_eq!(a.recip(), Some(Fixed128::from_rational(1, 2)));

		let a = Fixed128::from_natural(2);
		assert_eq!(a.recip().unwrap().checked_mul_int(&4i32), Some(2i32));

		let a = Fixed128::from_rational(100, 121);
		assert_eq!(a.recip(), Some(Fixed128::from_rational(121, 100)));

		let a = Fixed128::from_rational(1, 2);
		assert_eq!(a.recip().unwrap().checked_mul(&a), Some(Fixed128::from_natural(1)));

		let a = Fixed128::from_natural(0);
		assert_eq!(a.recip(), None);

		let a = Fixed128::from_rational(-1, 2);
		assert_eq!(a.recip(), Some(Fixed128::from_natural(-2)));
	}

	#[test]
	fn serialize_deserialize_should_work() {
		let two_point_five = Fixed128::from_rational(5, 2);
		let serialized = serde_json::to_string(&two_point_five).unwrap();
		assert_eq!(serialized, "\"2500000000000000000\"");
		let deserialized: Fixed128 = serde_json::from_str(&serialized).unwrap();
		assert_eq!(deserialized, two_point_five);

		let minus_two_point_five = Fixed128::from_rational(-5, 2);
		let serialized = serde_json::to_string(&minus_two_point_five).unwrap();
		assert_eq!(serialized, "\"-2500000000000000000\"");
		let deserialized: Fixed128 = serde_json::from_str(&serialized).unwrap();
		assert_eq!(deserialized, minus_two_point_five);
	}
}
