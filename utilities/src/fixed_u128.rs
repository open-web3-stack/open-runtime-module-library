use codec::{CompactAs, Decode, Encode};
use sp_arithmetic::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero};
use sp_core::U256;
use sp_runtime::{
	traits::{Bounded, Saturating, UniqueSaturatedInto},
	PerThing, Perbill, Percent, Permill, Perquintill,
};
use sp_std::{
	convert::{Into, TryFrom, TryInto},
	fmt::{self, Debug},
	ops::{self, Add, Div, Mul, Sub},
};

#[cfg(feature = "std")]
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// An unsigned fixed point number. Can hold any value in the range [0, 340_282_366_920_938_463_464]
/// with fixed point accuracy of 10 ** 18.
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, CompactAs)]
pub struct FixedU128(u128);

const DIV: u128 = 1_000_000_000_000_000_000;

/// Integer types that can be used to interact with `FixedPointNumber` implementations.
pub trait FixedPointOperand:
	Copy
	+ Clone
	+ Bounded
	+ Zero
	+ Saturating
	+ PartialOrd
	+ UniqueSaturatedInto<u128>
	+ TryFrom<u128>
	+ TryInto<u128>
	+ TryFrom<U256>
{
}

impl FixedPointOperand for i128 {}
impl FixedPointOperand for u128 {}
impl FixedPointOperand for i64 {}
impl FixedPointOperand for u64 {}
impl FixedPointOperand for i32 {}
impl FixedPointOperand for u32 {}
impl FixedPointOperand for i16 {}
impl FixedPointOperand for u16 {}
impl FixedPointOperand for i8 {}
impl FixedPointOperand for u8 {}

pub trait FixedUnSignedNumber:
	Sized
	+ Copy
	+ Default
	+ fmt::Debug
	+ Saturating
	+ Bounded
	+ Eq
	+ PartialEq
	+ Ord
	+ PartialOrd
	+ CheckedSub
	+ CheckedAdd
	+ CheckedMul
	+ CheckedDiv
	+ Add
	+ Sub
	+ Div
	+ Mul
	+ Zero
	+ One
{
	type Inner: Debug + One + CheckedMul + CheckedDiv + FixedPointOperand + Into<U256> + Into<u128>;

	/// Precision of this fixed point implementation. It should be a power of `10`.
	const DIV: Self::Inner;

	/// Precision of this fixed point implementation.
	fn accuracy() -> Self::Inner {
		Self::DIV
	}

	/// Create `self` from a natural number.
	///
	/// Note that this might be lossy.
	fn from_natural(int: Self::Inner) -> Self;

	/// Builds this type from an integer number.
	fn from_inner(int: Self::Inner) -> Self;

	/// Consumes `self` and returns the inner raw value.
	fn into_inner(self) -> Self::Inner;

	/// Creates self from an integer number `int`.
	///
	/// Returns `Self::max` or `Self::min` if `int` exceeds accuracy.
	fn saturating_from_integer<N: UniqueSaturatedInto<Self::Inner> + PartialOrd + Zero>(int: N) -> Self {
		if int < N::zero() {
			return Self::min_value();
		}

		Self::from_inner(int.unique_saturated_into().saturating_mul(Self::DIV))
	}

	/// Creates `self` from an integer number `int`.
	///
	/// Returns `None` if `int` exceeds accuracy.
	fn checked_from_integer(int: Self::Inner) -> Option<Self> {
		int.checked_mul(&Self::DIV).map(|inner| Self::from_inner(inner))
	}

	/// Creates `self` from a rational number. Equal to `n / d`.
	///
	/// Panics if `d = 0`. Returns `Self::max` or `Self::min` if `n / d` exceeds accuracy.
	fn saturating_from_rational<N: FixedPointOperand, D: FixedPointOperand>(n: N, d: D) -> Self {
		if d == D::zero() {
			panic!("attempt to divide by zero")
		}
		Self::checked_from_rational(n, d).unwrap_or(to_bound(n, d))
	}

	/// Creates `self` from a rational number. Equal to `n / d`.
	///
	/// Returns `None` if `d == 0` or `n / d` exceeds accuracy.
	fn checked_from_rational<N: FixedPointOperand, D: FixedPointOperand>(n: N, d: D) -> Option<Self> {
		if d == D::zero() {
			return None;
		}

		// this should really be `N: Into<U256>` or else might give wrong result
		// TODO: Should have a better way to enforce this requirement
		// let n = n.unique_saturated_into();
		// let d = d.unique_saturated_into();
		// let d = U256::from(d);

		let n: u128 = (n).try_into().ok()?;
		let n = U256::from(n);
		let d: u128 = (d).try_into().ok()?;
		let d = U256::from(d);

		n.checked_mul(U256::from(Self::DIV.unique_saturated_into()))
			.and_then(|n| n.checked_div(d))
			.and_then(|n| n.try_into().ok())
			.and_then(|n| Some(Self::from_inner(n)))
	}

	/// Checked mul for int type `N`. Equal to `self *  n`.
	///
	/// Returns `None` if the result does not fit in `N`.
	fn checked_mul_int<N: FixedPointOperand>(&self, other: &N) -> Option<N> {
		let lhs: U256 = self.into_inner().into();
		let rhs: u128 = (*other).try_into().ok()?;
		let rhs: U256 = U256::from(rhs);

		lhs.checked_mul(rhs)
			.and_then(|n| n.checked_div(U256::from(Self::DIV.unique_saturated_into())))
			.and_then(|n| n.try_into().ok())
			.and_then(|n| from_u128(n))
	}

	/// Saturating multiplication for integer type `N`. Equal to `self * n`.
	///
	/// Returns `N::min` or `N::max` if the result does not fit in `N`.
	fn saturating_mul_int<N: FixedPointOperand>(self, n: &N) -> N {
		self.checked_mul_int(n).unwrap_or(to_bound(self.into_inner(), *n))
	}

	/// Checked division for integer type `N`. Equal to `self / d`.
	///
	/// Returns `None` if the result does not fit in `N` or `d == 0`.
	fn checked_div_int<N: FixedPointOperand>(&self, other: &N) -> Option<N> {
		let lhs: u128 = self.into_inner().into();
		let rhs: u128 = (*other).try_into().ok()?;

		lhs.checked_div(rhs)
			.and_then(|n| n.checked_div(Self::DIV.unique_saturated_into()))
			.and_then(|n| from_u128(n))
	}

	/// Saturating division for integer type `N`. Equal to `self / d`.
	///
	/// Panics if `d == 0`. Returns `N::min` or `N::max` if the result does not fit in `N`.
	fn saturating_div_int<N: FixedPointOperand>(self, d: &N) -> N {
		if *d == N::zero() {
			panic!("attempt to divide by zero")
		}
		self.checked_div_int(d).unwrap_or(to_bound(self.into_inner(), *d))
	}

	/// Saturating multiplication for integer type `N`, adding the result back.
	/// Equal to `self * n + n`.
	///
	/// Returns `N::min` or `N::max` if the multiplication or final result does not fit in `N`.
	fn saturating_mul_acc_int<N: FixedPointOperand>(self, n: N) -> N {
		self.saturating_mul_int(&n).saturating_add(n)
	}

	/// Takes the reciprocal (inverse). Equal to `1 / self`.
	///
	/// Returns `None` if `self = 0`.
	fn reciprocal(self) -> Option<Self> {
		Self::one().checked_div(&self)
	}

	/// Returns the integer part.
	fn trunc(self) -> Self {
		self.into_inner()
			.checked_div(&Self::DIV)
			.expect("panics only if DIV is zero, DIV is not zero; qed")
			.checked_mul(&Self::DIV)
			.map(|inner| Self::from_inner(inner))
			.expect("can not overflow since fixed number is >= integer part")
	}

	/// Returns the fractional part.
	///
	/// Note: the returned fraction will be non-negative for negative numbers,
	/// except in the case where the integer part is zero.
	fn frac(self) -> Self {
		let integer = self.trunc();
		let fractional = self.saturating_sub(integer);

		fractional
	}

	/// Returns the smallest integer greater than or equal to a number.
	///
	/// Saturates to `Self::max` (truncated) if the result does not fit.
	fn ceil(self) -> Self {
		if self.is_zero() {
			return self.trunc();
		}
		self.saturating_add(Self::one()).trunc()
	}

	/// Returns the largest integer less than or equal to a number.
	///
	/// Saturates to `Self::min` (truncated) if the result does not fit.
	fn floor(self) -> Self {
		self.trunc()
	}

	/// Returns the number rounded to the nearest integer. Rounds half-way cases away from 0.0.
	///
	/// Saturates to `Self::min` or `Self::max` (truncated) if the result does not fit.
	fn round(self) -> Self {
		let n = self.frac().saturating_mul(Self::saturating_from_integer(10));
		if n < Self::saturating_from_integer(5) {
			self.floor()
		} else {
			self.ceil()
		}
	}
}

impl FixedUnSignedNumber for FixedU128 {
	type Inner = u128;

	const DIV: Self::Inner = DIV;

	fn from_natural(n: Self::Inner) -> Self {
		Self::from_inner(n.saturating_mul(Self::DIV))
	}

	fn from_inner(n: Self::Inner) -> Self {
		Self(n)
	}

	fn into_inner(self) -> Self::Inner {
		self.0
	}
}

impl FixedU128 {
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
}

/// Returns `R::max` if the sign of `n * m` is positive, `R::min` otherwise.
fn to_bound<N: FixedPointOperand, D: FixedPointOperand, R: Bounded + Zero>(n: N, m: D) -> R {
	if n < N::zero() {
		return R::zero();
	}
	if m < D::zero() {
		return R::zero();
	}

	R::max_value()
}

fn from_u128<N: FixedPointOperand>(n: u128) -> Option<N> {
	let r: N = n.try_into().ok()?;
	Some(r)
}

impl Zero for FixedU128 {
	fn zero() -> Self {
		Self(0)
	}

	fn is_zero(&self) -> bool {
		self.0 == 0
	}
}

impl One for FixedU128 {
	fn one() -> Self {
		Self(DIV)
	}

	fn is_one(&self) -> bool {
		self.0 == DIV
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

	fn saturating_pow(self, exp: usize) -> Self {
		if exp == 0 {
			return Self::saturating_from_integer(1);
		}

		let exp = exp as u64;
		let msb_pos = 64 - exp.leading_zeros();

		let mut result = Self::saturating_from_integer(1);
		let mut pow_val = self;
		for i in 0..msb_pos {
			if ((1 << i) & exp) > 0 {
				result = result.saturating_mul(pow_val);
			}
			pow_val = pow_val.saturating_mul(pow_val);
		}
		result
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

impl ops::Mul for FixedU128 {
	type Output = Self;

	fn mul(self, rhs: Self) -> Self::Output {
		self.checked_mul(&rhs)
			.unwrap_or_else(|| panic!("attempt to multiply with overflow"))
	}
}

impl ops::Div for FixedU128 {
	type Output = Self;

	fn div(self, rhs: Self) -> Self::Output {
		if rhs.0 == 0 {
			panic!("attempt to divide by zero")
		}

		self.checked_div(&rhs)
			.unwrap_or_else(|| panic!("attempt to divide with overflow"))
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
		U256::from(self.0)
			.checked_mul(U256::from(rhs.0))
			.and_then(|n| n.checked_div(U256::from(DIV)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.and_then(|n| Some(Self(n)))
	}
}

impl CheckedDiv for FixedU128 {
	fn checked_div(&self, rhs: &Self) -> Option<Self> {
		U256::from(self.0)
			.checked_mul(U256::from(DIV))
			.and_then(|n| n.checked_div(U256::from(rhs.0)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.and_then(|n| Some(Self(n)))
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

impl fmt::Debug for FixedU128 {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let fractional = format!("{:0>18}", self.0 % DIV);
		write!(f, "FixedU128({}.{})", self.0 / DIV, fractional)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
		Ok(())
	}
}

#[cfg(feature = "std")]
impl sp_std::fmt::Display for FixedU128 {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

#[cfg(feature = "std")]
impl sp_std::str::FromStr for FixedU128 {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let inner: u128 = s.parse().map_err(|_| "invalid string input for fixed u128")?;
		Ok(Self::from_inner(inner))
	}
}

impl From<u128> for FixedU128 {
	fn from(n: u128) -> Self {
		Self::saturating_from_integer(n)
	}
}

impl From<Permill> for FixedU128 {
	fn from(val: Permill) -> Self {
		FixedU128::saturating_from_rational(val.deconstruct(), Permill::ACCURACY)
	}
}

impl From<Percent> for FixedU128 {
	fn from(val: Percent) -> Self {
		FixedU128::saturating_from_rational(val.deconstruct(), Percent::ACCURACY)
	}
}

impl From<Perbill> for FixedU128 {
	fn from(val: Perbill) -> Self {
		FixedU128::saturating_from_rational(val.deconstruct(), Perbill::ACCURACY)
	}
}

impl From<Perquintill> for FixedU128 {
	fn from(val: Perquintill) -> Self {
		FixedU128::saturating_from_rational(val.deconstruct(), Perquintill::ACCURACY)
	}
}

#[cfg(feature = "std")]
impl FixedU128 {
	fn u128_str(&self) -> String {
		format!("{}", &self.0)
	}

	fn try_from_u128_str(s: &str) -> Result<Self, &'static str> {
		let parts: u128 = s.parse().map_err(|_| "invalid string input")?;
		Ok(Self::from_inner(parts))
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
		serializer.serialize_str(&self.u128_str())
	}
}

// Manual impl `Serialize` as serde_json does not support u128.
// TODO: remove impl if issue https://github.com/serde-rs/json/issues/548 fixed.
#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for FixedU128 {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		FixedU128::try_from_u128_str(&s).map_err(de::Error::custom)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sp_runtime::{Perbill, Percent, Permill, Perquintill};

	fn max() -> FixedU128 {
		FixedU128::from_inner(u128::max_value())
	}
	#[test]
	fn to_bound_works() {
		let a = 1i32;
		let b = 1i32;

		// Pos + Pos => Max.
		assert_eq!(to_bound::<_, _, i32>(a, b), i32::max_value());

		let a = -1i32;
		let b = -1i32;

		// Neg + Neg => 0.
		assert_eq!(to_bound::<_, _, i32>(a, b), 0);

		let a = 1i32;
		let b = -1i32;

		// Pos + Neg => 0.
		assert_eq!(to_bound::<_, _, i32>(a, b), 0);

		let a = -1i32;
		let b = 1i32;

		// Neg + Pos => 0.
		assert_eq!(to_bound::<_, _, i32>(a, b), 0);

		let a = 1i32;
		let b = -1i32;

		// Pos + Neg => Min (unsigned).
		assert_eq!(to_bound::<_, _, u32>(a, b), 0);
	}

	#[test]
	fn fixed128_semantics() {
		assert_eq!(
			FixedU128::saturating_from_rational(5, 2).into_inner(),
			5 * 1_000_000_000_000_000_000 / 2
		);
		assert_eq!(
			FixedU128::saturating_from_rational(5, 2),
			FixedU128::saturating_from_rational(10, 4)
		);

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
		assert_eq!(a.checked_div(&b), Some(FixedU128::saturating_from_rational(2, 1)));

		let a = FixedU128::saturating_from_rational(5, 2);
		let b = FixedU128::saturating_from_rational(3, 2);
		assert_eq!(a.checked_add(&b), Some(FixedU128::saturating_from_rational(8, 2)));
		assert_eq!(a.checked_sub(&b), Some(FixedU128::saturating_from_rational(2, 2)));
		assert_eq!(a.checked_mul(&b), Some(FixedU128::saturating_from_rational(15, 4)));
		assert_eq!(a.checked_div(&b), Some(FixedU128::saturating_from_rational(10, 6)));

		let a = FixedU128::from_natural(120);
		let b = 2i32;
		assert_eq!(a.checked_div_int::<i32>(&b), Some(60));

		let a = FixedU128::saturating_from_rational(20, 1);
		let b = 2i32;
		assert_eq!(a.checked_div_int::<i32>(&b), Some(10));

		let a = FixedU128::from_natural(120);
		let b = 2i32;
		assert_eq!(a.checked_mul_int::<i32>(&b), Some(240));

		let a = FixedU128::saturating_from_rational(1, 2);
		let b = 20i32;
		assert_eq!(a.checked_mul_int::<i32>(&b), Some(10));
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
	fn op_checked_add_overflow_should_be_none() {
		let a = FixedU128::max_value();
		let b = 1.into();
		assert!(a.checked_add(&b).is_none());
	}

	#[test]
	#[should_panic(expected = "attempt to add with overflow")]
	fn op_add_overflow_should_panic() {
		let a = FixedU128::max_value();
		let b = 1.into();
		let _c = a + b;
	}

	#[test]
	fn op_add_works() {
		let a = FixedU128::saturating_from_rational(1, 2);
		let b = FixedU128::saturating_from_rational(5, 3);
		assert_eq!(FixedU128::saturating_from_rational(13, 6), a + b);
	}

	#[test]
	fn op_checked_sub_underflow_should_be_none() {
		let a = FixedU128::min_value();
		let b = 1.into();
		assert!(a.checked_sub(&b).is_none());
	}

	#[test]
	#[should_panic(expected = "attempt to subtract with overflow")]
	fn op_sub_underflow_should_panic() {
		let a = FixedU128::min_value();
		let b = 1.into();
		let _c = a - b;
	}

	#[test]
	fn op_sub_works() {
		let a = FixedU128::saturating_from_rational(1, 2);
		let b = FixedU128::saturating_from_rational(5, 3);
		assert_eq!(FixedU128::saturating_from_rational(7, 6), b - a);
	}

	#[test]
	fn op_checked_mul_overflow_should_be_none() {
		let a = FixedU128::max_value();
		let b = 2.into();
		assert!(a.checked_mul(&b).is_none());
	}

	#[test]
	#[should_panic(expected = "attempt to multiply with overflow")]
	fn op_mul_overflow_should_panic() {
		let a = FixedU128::max_value();
		let b = 2.into();
		let _c = a * b;
	}

	#[test]
	fn op_mul_works() {
		let a = FixedU128::saturating_from_rational(1, 2);
		let b = FixedU128::saturating_from_rational(5, 3);
		assert_eq!(FixedU128::saturating_from_rational(5, 6), a * b);
	}

	#[test]
	fn op_checked_div_with_zero_should_be_none() {
		let a = FixedU128::min_value();
		let b = 0.into();
		assert!(a.checked_div(&b).is_none());
	}

	#[test]
	#[should_panic(expected = "attempt to divide by zero")]
	fn op_div_zero_should_panic() {
		let a = FixedU128::max_value();
		let b = 0.into();
		let _c = a / b;
	}

	#[test]
	fn op_div_works() {
		let a = FixedU128::saturating_from_rational(1, 2);
		let b = FixedU128::saturating_from_rational(5, 3);
		assert_eq!(FixedU128::saturating_from_rational(3, 10), a / b);
	}

	#[test]
	fn checked_div_int_with_zero_should_be_none() {
		let a = FixedU128::from_natural(1);
		let b = 0i32;
		assert_eq!(a.checked_div_int(&b), None);
	}

	#[test]
	fn saturation_from_integer_works() {
		let inner_max = <FixedU128 as FixedUnSignedNumber>::Inner::max_value();
		let inner_min = <FixedU128 as FixedUnSignedNumber>::Inner::min_value();
		let accuracy = FixedU128::accuracy();

		// Cases where integer fits.
		let a = FixedU128::saturating_from_integer(42);
		assert_eq!(a.into_inner(), 42 * accuracy);

		// Cases where pass an negative number, should return zero
		let a = FixedU128::saturating_from_integer(-42);
		assert_eq!(a.into_inner(), 0);

		// Max/min integers that fit.
		let a = FixedU128::saturating_from_integer(inner_max / accuracy);
		assert_eq!(a.into_inner(), (inner_max / accuracy) * accuracy);

		let a = FixedU128::saturating_from_integer(inner_min / accuracy);
		assert_eq!(a.into_inner(), (inner_min / accuracy) * accuracy);

		// Cases where integer doesn't fit, so it saturates.
		let a = FixedU128::saturating_from_integer(inner_max / accuracy + 1);
		assert_eq!(a.into_inner(), inner_max);
	}

	#[test]
	fn checked_from_integer_works() {
		let inner_max = <FixedU128 as FixedUnSignedNumber>::Inner::max_value();
		let inner_min = <FixedU128 as FixedUnSignedNumber>::Inner::min_value();
		let accuracy = FixedU128::accuracy();

		// Cases where integer fits.
		let a = FixedU128::checked_from_integer(42).expect("42 * accuracy < inner_max, qed");
		assert_eq!(a.into_inner(), 42 * accuracy);

		// Max/min integers that fit.
		let a = FixedU128::checked_from_integer(inner_max / accuracy).expect("inner_max / accuracy < inner_max, qed");
		assert_eq!(a.into_inner(), (inner_max / accuracy) * accuracy);

		let a = FixedU128::checked_from_integer(inner_min).expect("inner_min = 0, qed");
		assert_eq!(a.into_inner(), inner_min);

		// Cases where integer not fit.
		let a = FixedU128::checked_from_integer(inner_max / accuracy + 1);
		assert_eq!(a, None);
	}

	#[test]
	fn from_inner_works() {
		let inner_max = <FixedU128 as FixedUnSignedNumber>::Inner::max_value();
		let inner_min = <FixedU128 as FixedUnSignedNumber>::Inner::min_value();
		assert_eq!(FixedU128::max_value(), FixedU128::from_inner(inner_max));
		assert_eq!(FixedU128::min_value(), FixedU128::from_inner(inner_min));
	}

	#[test]
	fn saturating_from_ration_works() {
		let inner_max = <FixedU128 as FixedUnSignedNumber>::Inner::max_value();
		let inner_min = <FixedU128 as FixedUnSignedNumber>::Inner::min_value();
		let accuracy = FixedU128::accuracy();

		// Cases where parameters fit.
		let a = FixedU128::saturating_from_rational(3, 5);
		assert_eq!(a.into_inner(), 3 * accuracy / 5);

		// Cases where MIX/MIN
		let a = FixedU128::saturating_from_rational(inner_min, 1);
		assert_eq!(a.into_inner(), inner_min);

		let a = FixedU128::saturating_from_rational(inner_max, 1);
		assert_eq!(a.into_inner(), inner_max);

		// Cases where parameters are negative should return zero
		let a = FixedU128::saturating_from_rational(-1, 1);
		assert_eq!(a.into_inner(), 0);

		let a = FixedU128::saturating_from_rational(1, -1);
		assert_eq!(a.into_inner(), 0);

		let a = FixedU128::saturating_from_rational(-3, -5);
		assert_eq!(a.into_inner(), 0);
	}

	#[test]
	#[should_panic(expected = "attempt to divide by zero")]
	fn saturation_from_ration_with_zero_should_panic() {
		let _a = FixedU128::saturating_from_rational(100, 0);
	}

	#[test]
	fn checked_from_rational_works() {
		let inner_max = <FixedU128 as FixedUnSignedNumber>::Inner::max_value();
		let inner_min = <FixedU128 as FixedUnSignedNumber>::Inner::min_value();
		let accuracy = FixedU128::accuracy();

		let a = FixedU128::checked_from_rational(3, 5).expect("3 * accuracy / 5 < inner_max, qed");
		assert_eq!(a.into_inner(), 3 * accuracy / 5);

		// Case: limit
		let a = FixedU128::checked_from_rational(inner_min, 1).expect("inner_min / 1 = inner_min, qed");
		assert_eq!(a.into_inner(), inner_min);

		let a = FixedU128::checked_from_rational(inner_max / accuracy, 1)
			.expect("inner_max / accuracy * accuracy < inner_max, qed");
		assert_eq!(a.into_inner(), inner_max / accuracy * accuracy);

		let a = FixedU128::checked_from_rational(inner_min as i128 - 1, 1);
		assert_eq!(a, None);

		let a = FixedU128::checked_from_rational(inner_max, 1);
		assert_eq!(a, None);

		// Cases where parameters are negative should return None
		let a = FixedU128::checked_from_rational(3, -5);
		assert_eq!(a, None);

		let a = FixedU128::checked_from_rational(-3, 5);
		assert_eq!(a, None);

		let a = FixedU128::checked_from_rational(-3, -5);
		assert_eq!(a, None);

		// Case: divided zero should return None
		let a = FixedU128::checked_from_rational(10, 0);
		assert_eq!(a, None);
	}

	#[test]
	fn checked_mul_int_works() {
		let a = FixedU128::saturating_from_rational(10, 1);
		let b = u32::max_value() / 5;
		assert_eq!(a.checked_mul_int(&b), None);

		let a = FixedU128::saturating_from_integer(120);
		let b = 2i32;
		assert_eq!(a.checked_mul_int::<i32>(&b), Some(240));

		let a = FixedU128::saturating_from_rational(1, 2);
		let b = 20i32;
		assert_eq!(a.checked_mul_int::<i32>(&b), Some(10));

		// Case where the integer is negative should return None
		let a = FixedU128::saturating_from_rational(1, 2);
		let b = -20i32;
		assert_eq!(a.checked_mul_int::<i32>(&b), None);
	}

	#[test]
	fn saturating_mul_int_works() {
		let a = FixedU128::saturating_from_rational(10, 1);
		let b = u32::max_value() / 5;
		assert_eq!(a.saturating_mul_int(&b), u32::max_value());

		let a = FixedU128::saturating_from_rational(10, 1);
		let b = 123;
		assert_eq!(a.saturating_mul_int(&b), 1230);

		// Case where the integer is negative should return zero
		let a = FixedU128::saturating_from_rational(1, 2);
		let b = -20i32;
		assert_eq!(a.saturating_mul_int::<i32>(&b), 0);
	}

	#[test]
	fn checked_div_int_works() {
		let a = FixedU128::max_value();
		let b = 5u32;
		assert_eq!(a.checked_div_int(&b), None);

		let a = FixedU128::saturating_from_integer(120);
		let b = -2i32;
		assert_eq!(a.checked_div_int::<i32>(&b), None);

		let a = FixedU128::saturating_from_rational(1, 2);
		let b = 20i32;
		assert_eq!(a.checked_div_int::<i32>(&b), Some(0));

		let a = FixedU128::saturating_from_integer(100);
		let b = 10u8;
		assert_eq!(a.checked_div_int(&b), Some(10u8));

		// Case where the integer is negative should return None
		let a = FixedU128::saturating_from_integer(100);
		let b = -10;
		assert_eq!(a.checked_div_int(&b), None);

		let a = FixedU128::saturating_from_integer(100);
		let b = 10;
		assert_eq!(a.checked_div_int(&b), Some(10));
	}

	#[test]
	fn saturating_div_int_works() {
		let a = FixedU128::max_value();
		let b = 5u32;
		assert_eq!(a.saturating_div_int(&b), u32::max_value());

		let a = FixedU128::saturating_from_integer(100);
		let b = 10u8;
		assert_eq!(a.saturating_div_int(&b), 10u8);

		// Case where the integer is negative should return zero
		let a = FixedU128::saturating_from_integer(100);
		let b = -10;
		assert_eq!(a.saturating_div_int(&b), 0);

		let a = FixedU128::saturating_from_integer(100);
		let b = 10;
		assert_eq!(a.saturating_div_int(&b), 10);
	}

	#[test]
	fn saturating_mul_acc_int_works() {
		assert_eq!(FixedU128::zero().saturating_mul_acc_int(42u8), 42u8);
		assert_eq!(FixedU128::one().saturating_mul_acc_int(42u8), 2 * 42u8);

		assert_eq!(
			FixedU128::one().saturating_mul_acc_int(i128::max_value()),
			i128::max_value()
		);
		assert_eq!(
			FixedU128::one().saturating_mul_acc_int(i128::min_value()),
			i128::min_value()
		);

		assert_eq!(
			FixedU128::one().saturating_mul_acc_int(u128::max_value() / 2),
			u128::max_value() - 1
		);
		assert_eq!(
			FixedU128::one().saturating_mul_acc_int(u128::min_value()),
			u128::min_value()
		);
	}

	#[test]
	fn checked_div_works() {
		let inner_max = <FixedU128 as FixedUnSignedNumber>::Inner::max_value();
		let inner_min = <FixedU128 as FixedUnSignedNumber>::Inner::min_value();

		let a = FixedU128::from_inner(inner_max);
		let b = FixedU128::from_inner(inner_min);
		let c = FixedU128::zero();
		let d = FixedU128::one();
		let e = FixedU128::saturating_from_integer(6);
		let f = FixedU128::saturating_from_integer(5);

		assert_eq!(e.checked_div(&2.into()), Some(3.into()));
		assert_eq!(
			f.checked_div(&2.into()),
			Some(FixedU128::saturating_from_rational(10, 4))
		);

		assert_eq!(a.checked_div(&inner_max.into()), Some(1.into()));
		assert_eq!(a.checked_div(&2.into()), Some(FixedU128::from_inner(inner_max / 2)));
		assert_eq!(a.checked_div(&FixedU128::max_value()), Some(1.into()));
		assert_eq!(a.checked_div(&d), Some(a));

		// Cases inner_min is zero
		assert_eq!(b.checked_div(&b), None);

		assert_eq!(c.checked_div(&1.into()), Some(0.into()));
		assert_eq!(c.checked_div(&FixedU128::max_value()), Some(0.into()));

		assert_eq!(d.checked_div(&1.into()), Some(1.into()));

		assert_eq!(a.checked_div(&FixedU128::one()), Some(a));
		assert_eq!(b.checked_div(&FixedU128::one()), Some(b));
		assert_eq!(c.checked_div(&FixedU128::one()), Some(c));
		assert_eq!(d.checked_div(&FixedU128::one()), Some(d));

		assert_eq!(a.checked_div(&FixedU128::zero()), None);
		assert_eq!(b.checked_div(&FixedU128::zero()), None);
		assert_eq!(c.checked_div(&FixedU128::zero()), None);
		assert_eq!(d.checked_div(&FixedU128::zero()), None);
	}

	#[test]
	fn trunc_works() {
		let n = FixedU128::saturating_from_rational(5, 2).trunc();
		assert_eq!(n, FixedU128::saturating_from_integer(2));
	}

	#[test]
	fn frac_works() {
		let n = FixedU128::saturating_from_rational(5, 2);
		let i = n.trunc();
		let f = n.frac();

		assert_eq!(n, i + f);

		let n = FixedU128::saturating_from_rational(5, 2)
			.frac()
			.saturating_mul(10.into());
		assert_eq!(n, 5.into());

		let n = FixedU128::saturating_from_rational(1, 2)
			.frac()
			.saturating_mul(10.into());
		assert_eq!(n, 5.into());
	}

	#[test]
	fn ceil_works() {
		let n = FixedU128::saturating_from_rational(5, 2);
		assert_eq!(n.ceil(), 3.into());

		// On the limits:
		let n = FixedU128::max_value();
		assert_eq!(n.ceil(), n.trunc());

		let n = FixedU128::min_value();
		assert_eq!(n.ceil(), n.trunc());
	}

	#[test]
	fn floor_works() {
		let n = FixedU128::saturating_from_rational(5, 2);
		assert_eq!(n.floor(), 2.into());

		// On the limits:
		let n = FixedU128::max_value();
		assert_eq!(n.floor(), n.trunc());

		let n = FixedU128::min_value();
		assert_eq!(n.floor(), n.trunc());
	}

	#[test]
	fn round_works() {
		let n = FixedU128::zero();
		assert_eq!(n.round(), n);

		let n = FixedU128::one();
		assert_eq!(n.round(), n);

		let n = FixedU128::saturating_from_rational(5, 2);
		assert_eq!(n.round(), 3.into());

		let n = FixedU128::max_value();
		assert_eq!(n.round(), n.trunc());

		let n = FixedU128::min_value();
		assert_eq!(n.round(), n.trunc());

		// On the limit:

		// floor(max - 1) + 0.33..
		let n = FixedU128::max_value()
			.saturating_sub(1.into())
			.trunc()
			.saturating_add(FixedU128::saturating_from_rational(1, 3));

		assert_eq!(n.round(), (FixedU128::max_value() - 1.into()).trunc());

		// floor(min + 1) - 0.33..
		let n = FixedU128::min_value()
			.saturating_add(1.into())
			.trunc()
			.saturating_sub(FixedU128::saturating_from_rational(1, 3));

		assert_eq!(n.round(), (FixedU128::min_value() + 1.into()).trunc());

		// floor(max - 1) + 0.6
		let n = FixedU128::max_value()
			.saturating_sub(1.into())
			.trunc()
			.saturating_add(FixedU128::saturating_from_rational(1, 2));

		assert_eq!(n.round(), FixedU128::max_value().trunc());

		// floor(min + 1) - 0.6
		let n = FixedU128::min_value()
			.saturating_add(1.into())
			.trunc()
			.saturating_sub(FixedU128::saturating_from_rational(10, 6));

		assert_eq!(n.round(), FixedU128::min_value().trunc());
	}

	#[test]
	fn perthing_into_fixed_u128() {
		let ten_percent_percent: FixedU128 = Percent::from_percent(10).into();
		assert_eq!(ten_percent_percent.into_inner(), DIV / 10);

		let ten_percent_permill: FixedU128 = Permill::from_percent(10).into();
		assert_eq!(ten_percent_permill.into_inner(), DIV / 10);

		let ten_percent_perbill: FixedU128 = Perbill::from_percent(10).into();
		assert_eq!(ten_percent_perbill.into_inner(), DIV / 10);

		let ten_percent_perquintill: FixedU128 = Perquintill::from_percent(10).into();
		assert_eq!(ten_percent_perquintill.into_inner(), DIV / 10);
	}

	#[test]
	fn recip_works() {
		let a = FixedU128::from_natural(2);
		assert_eq!(a.reciprocal(), Some(FixedU128::saturating_from_rational(1, 2)));

		let a = FixedU128::from_natural(2);
		assert_eq!(a.reciprocal().unwrap().checked_mul_int(&4i32), Some(2i32));

		let a = FixedU128::saturating_from_rational(100, 121);
		assert_eq!(a.reciprocal(), Some(FixedU128::saturating_from_rational(121, 100)));

		let a = FixedU128::saturating_from_rational(1, 2);
		assert_eq!(
			a.reciprocal().unwrap().checked_mul(&a),
			Some(FixedU128::from_natural(1))
		);

		let a = FixedU128::from_natural(0);
		assert_eq!(a.reciprocal(), None);
	}

	#[test]
	fn serialize_deserialize_works() {
		let two_point_five = FixedU128::saturating_from_rational(5, 2);
		let serialized = serde_json::to_string(&two_point_five).unwrap();
		assert_eq!(serialized, "\"2500000000000000000\"");
		let deserialized: FixedU128 = serde_json::from_str(&serialized).unwrap();
		assert_eq!(deserialized, two_point_five);
	}
}
