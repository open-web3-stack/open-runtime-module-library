use parity_scale_codec::FullCodec;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize};
use sp_std::fmt::Debug;

/// Hooks to manage reward pool
pub trait RewardHandler<AccountId, CurrencyId> {
	/// The reward balance type
	type Balance: AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Debug;

	/// The reward pool ID type
	type PoolId: FullCodec;

	/// Payout the reward to `who`
	fn payout(who: &AccountId, pool: &Self::PoolId, currency_id: CurrencyId, amount: Self::Balance);
}
