use codec::Encode;
use frame_support::{Parameter, RuntimeDebug};
use sp_runtime::traits::Member;

#[derive(PartialEq, Eq, RuntimeDebug)]
pub enum RateLimiterError {
	NotDefined,
	ExceedLimit,
}

/// Rate Limiter
pub trait RateLimiter {
	/// The type for the rate limiter.
	type RateLimiterId: Parameter + Member + Copy;

	/// Check whether the rate limiter of can be bypassed according to the
	/// `key`.
	fn is_whitelist(limiter_id: Self::RateLimiterId, key: impl Encode) -> bool;

	/// Check whether the `value` can be passed the limit of `limit_key`.
	fn is_allowed(limiter_id: Self::RateLimiterId, limit_key: impl Encode, value: u128)
		-> Result<(), RateLimiterError>;

	/// The handler function after allowed.
	fn record(limiter_id: Self::RateLimiterId, limit_key: impl Encode, value: u128);
}

impl RateLimiter for () {
	type RateLimiterId = ();

	fn is_whitelist(_: Self::RateLimiterId, _: impl Encode) -> bool {
		true
	}

	fn is_allowed(_: Self::RateLimiterId, _: impl Encode, _: u128) -> Result<(), RateLimiterError> {
		Ok(())
	}

	fn record(_: Self::RateLimiterId, _: impl Encode, _: u128) {}
}
