#![allow(clippy::unused_unit)]
#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{FullCodec, HasCompact};
use frame_support::{pallet_prelude::*, traits::MaxEncodedLen};
use orml_traits::RewardHandler;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, Saturating, Zero},
	FixedPointNumber, FixedPointOperand, FixedU128, RuntimeDebug,
};
use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug,
};

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default, MaxEncodedLen)]
pub struct PoolInfo<Share: HasCompact, Balance: HasCompact> {
	/// Total shares amount
	#[codec(compact)]
	pub total_shares: Share,
	/// Total rewards amount
	#[codec(compact)]
	pub total_rewards: Balance,
	/// Total withdrawn rewards amount
	#[codec(compact)]
	pub total_withdrawn_rewards: Balance,
}

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The share type of pool.
		type Share: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ FixedPointOperand;

		/// The reward balance type.
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ FixedPointOperand;

		/// The reward pool ID type.
		type PoolId: Parameter + Member + Clone + FullCodec;

		/// The `RewardHandler`
		type Handler: RewardHandler<Self::AccountId, Balance = Self::Balance, PoolId = Self::PoolId>;
	}

	/// Stores reward pool info.
	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> = StorageMap<_, Twox64Concat, T::PoolId, PoolInfo<T::Share, T::Balance>, ValueQuery>;

	/// Record share amount and withdrawn reward amount for specific `AccountId`
	/// under `PoolId`.
	#[pallet::storage]
	#[pallet::getter(fn share_and_withdrawn_reward)]
	pub type ShareAndWithdrawnReward<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::PoolId, Twox64Concat, T::AccountId, (T::Share, T::Balance), ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
	pub fn accumulate_reward(pool: &T::PoolId, reward_increment: T::Balance) {
		if !reward_increment.is_zero() {
			Pools::<T>::mutate(pool, |pool_info| {
				pool_info.total_rewards = pool_info.total_rewards.saturating_add(reward_increment)
			});
		}
	}

	pub fn add_share(who: &T::AccountId, pool: &T::PoolId, add_amount: T::Share) {
		if add_amount.is_zero() {
			return;
		}

		Pools::<T>::mutate(pool, |pool_info| {
			let proportion = FixedU128::checked_from_rational(add_amount, pool_info.total_shares).unwrap_or_default();
			let reward_inflation = proportion.saturating_mul_int(pool_info.total_rewards);

			pool_info.total_shares = pool_info.total_shares.saturating_add(add_amount);
			pool_info.total_rewards = pool_info.total_rewards.saturating_add(reward_inflation);
			pool_info.total_withdrawn_rewards = pool_info.total_withdrawn_rewards.saturating_add(reward_inflation);

			ShareAndWithdrawnReward::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
				*share = share.saturating_add(add_amount);
				*withdrawn_rewards = withdrawn_rewards.saturating_add(reward_inflation);
			});
		});
	}

	pub fn remove_share(who: &T::AccountId, pool: &T::PoolId, remove_amount: T::Share) {
		if remove_amount.is_zero() {
			return;
		}

		// claim rewards firstly
		Self::claim_rewards(who, pool);

		ShareAndWithdrawnReward::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
			let remove_amount = remove_amount.min(*share);

			if remove_amount.is_zero() {
				return;
			}

			Pools::<T>::mutate(pool, |pool_info| {
				let proportion = FixedU128::checked_from_rational(remove_amount, *share).unwrap_or_default();
				let withdrawn_rewards_to_remove = proportion.saturating_mul_int(*withdrawn_rewards);

				pool_info.total_shares = pool_info.total_shares.saturating_sub(remove_amount);
				pool_info.total_rewards = pool_info.total_rewards.saturating_sub(withdrawn_rewards_to_remove);
				pool_info.total_withdrawn_rewards = pool_info
					.total_withdrawn_rewards
					.saturating_sub(withdrawn_rewards_to_remove);

				*withdrawn_rewards = withdrawn_rewards.saturating_sub(withdrawn_rewards_to_remove);
			});

			*share = share.saturating_sub(remove_amount);
		});
	}

	pub fn set_share(who: &T::AccountId, pool: &T::PoolId, new_share: T::Share) {
		let (share, _) = Self::share_and_withdrawn_reward(pool, who);

		if new_share > share {
			Self::add_share(who, pool, new_share.saturating_sub(share));
		} else {
			Self::remove_share(who, pool, share.saturating_sub(new_share));
		}
	}

	pub fn claim_rewards(who: &T::AccountId, pool: &T::PoolId) {
		ShareAndWithdrawnReward::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
			if share.is_zero() {
				return;
			}

			Pools::<T>::mutate(pool, |pool_info| {
				let proportion = FixedU128::checked_from_rational(*share, pool_info.total_shares).unwrap_or_default();
				let reward_to_withdraw = proportion
					.saturating_mul_int(pool_info.total_rewards)
					.saturating_sub(*withdrawn_rewards)
					.min(
						pool_info
							.total_rewards
							.saturating_sub(pool_info.total_withdrawn_rewards),
					);

				if reward_to_withdraw.is_zero() {
					return;
				}

				pool_info.total_withdrawn_rewards =
					pool_info.total_withdrawn_rewards.saturating_add(reward_to_withdraw);
				*withdrawn_rewards = withdrawn_rewards.saturating_add(reward_to_withdraw);

				// pay reward to `who`
				T::Handler::payout(who, pool, reward_to_withdraw);
			});
		});
	}
}
