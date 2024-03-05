use frame_support::Parameter;
use parity_scale_codec::Encode;
use sp_runtime::{traits::Member, RuntimeDebug};

#[derive(PartialEq, Eq, RuntimeDebug)]
pub enum RateLimiterError {
	ExceedLimit,
}

/// Rate Limiter
pub trait RateLimiter {
	/// The type for the rate limiter.
	type RateLimiterId: Parameter + Member + Copy;

	/// Check whether the rate limiter of can be bypassed according to the
	/// `key`.
	fn is_whitelist(limiter_id: Self::RateLimiterId, key: impl Encode) -> bool;

	/// Check whether the `value` can be consumed under the limit of
	/// `limit_key`.
	fn can_consume(
		limiter_id: Self::RateLimiterId,
		limit_key: impl Encode,
		value: u128,
	) -> Result<(), RateLimiterError>;

	/// The handler function to consume quota.
	fn consume(limiter_id: Self::RateLimiterId, limit_key: impl Encode, value: u128);

	/// Try consume quota.
	fn try_consume(
		limiter_id: Self::RateLimiterId,
		limit_key: impl Encode + Clone,
		value: u128,
		whitelist_check: Option<impl Encode>,
	) -> Result<(), RateLimiterError> {
		let need_consume = match whitelist_check {
			Some(whitelist_key) => !Self::is_whitelist(limiter_id, whitelist_key),
			None => true,
		};

		if need_consume {
			Self::can_consume(limiter_id, limit_key.clone(), value)?;
			Self::consume(limiter_id, limit_key, value);
		}

		Ok(())
	}
}

impl RateLimiter for () {
	type RateLimiterId = ();

	fn is_whitelist(_: Self::RateLimiterId, _: impl Encode) -> bool {
		true
	}

	fn can_consume(_: Self::RateLimiterId, _: impl Encode, _: u128) -> Result<(), RateLimiterError> {
		Ok(())
	}

	fn consume(_: Self::RateLimiterId, _: impl Encode, _: u128) {}
}
