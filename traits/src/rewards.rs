use codec::FullCodec;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize};
use sp_std::fmt::Debug;

/// Hooks to manage reward pool
pub trait RewardHandler<AccountId, BlockNumber> {
	/// The share type of pool
	type Share: AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Debug;

	/// The reward balance type
	type Balance: AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Debug;

	/// The reward pool ID type
	type PoolId: Copy + FullCodec;

	/// Accumulate rewards
	fn accumulate_reward(now: BlockNumber, callback: impl Fn(Self::PoolId, Self::Balance)) -> Self::Balance;

	/// Payout the reward to `who`
	fn payout(who: &AccountId, pool: Self::PoolId, amount: Self::Balance);
}
