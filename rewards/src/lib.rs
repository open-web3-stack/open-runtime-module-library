#![allow(clippy::unused_unit)]
#![cfg_attr(not(feature = "std"), no_std)]

mod migrations;
mod mock;
mod tests;

pub use migrations::migrate_to_multi_currency_reward;

use codec::{FullCodec, HasCompact, MaxEncodedLen};
use frame_support::{pallet_prelude::*, weights::Weight};
use orml_traits::RewardHandler;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Bounded, MaybeSerializeDeserialize, Member, Saturating, Zero},
	FixedPointNumber, FixedPointOperand, FixedU128, RuntimeDebug,
};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug,
};

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default, MaxEncodedLen)]
pub struct PoolInfoV0<Share: HasCompact, Balance: HasCompact> {
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

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct PoolInfo<Share: HasCompact, Balance: HasCompact, CurrencyId: Ord> {
	/// Total shares amount
	#[codec(compact)]
	pub total_shares: Share,
	/// Reward infos
	pub rewards: BTreeMap<CurrencyId, (Balance, Balance)>,
}

impl<Share, Balance, CurrencyId> Default for PoolInfo<Share, Balance, CurrencyId>
where
	Share: Default + HasCompact,
	Balance: HasCompact,
	CurrencyId: Ord,
{
	fn default() -> Self {
		Self {
			total_shares: Default::default(),
			rewards: BTreeMap::new(),
		}
	}
}

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, MaxEncodedLen)]
enum Releases {
	V0,
	V1,
}

impl Default for Releases {
	fn default() -> Self {
		Releases::V0
	}
}

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;
	use orml_traits::GetByKey;

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

		type CurrencyId: Parameter + Member + Default + Copy + MaybeSerializeDeserialize + Ord;

		/// The `RewardHandler`
		type Handler: RewardHandler<Self::AccountId, Self::CurrencyId, Balance = Self::Balance, PoolId = Self::PoolId>;

		type V0Migration: GetByKey<Self::PoolId, Self::CurrencyId>;
	}

	/// Stores reward pool info.
	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config> =
		StorageMap<_, Twox64Concat, T::PoolId, PoolInfo<T::Share, T::Balance, T::CurrencyId>, ValueQuery>;

	/// Record share amount and withdrawn reward amount for specific `AccountId`
	/// under `PoolId`.
	#[pallet::storage]
	#[pallet::getter(fn share_and_withdrawn_reward)]
	pub type ShareAndWithdrawnReward<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::PoolId,
		Twox64Concat,
		T::AccountId,
		(T::Share, BTreeMap<T::CurrencyId, T::Balance>),
		ValueQuery,
	>;

	/// The current version of the pallet.
	#[pallet::storage]
	pub(crate) type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {
	pub fn accumulate_reward(pool: &T::PoolId, reward_currency: T::CurrencyId, reward_increment: T::Balance) {
		if !reward_increment.is_zero() {
			Pools::<T>::mutate(pool, |pool_info| {
				if let Some((total_rewards, _)) = pool_info.rewards.get_mut(&reward_currency) {
					*total_rewards = total_rewards.saturating_add(reward_increment);
				} else {
					pool_info
						.rewards
						.insert(reward_currency, (reward_increment, Zero::zero()));
				}
			});
		}
	}

	pub fn add_share(who: &T::AccountId, pool: &T::PoolId, add_amount: T::Share) {
		if add_amount.is_zero() {
			return;
		}

		Pools::<T>::mutate(pool, |pool_info| {
			let total_shares = pool_info.total_shares;
			pool_info.total_shares = pool_info.total_shares.saturating_add(add_amount);

			let mut withdrawn_inflations = Vec::<(&T::CurrencyId, T::Balance)>::new();

			pool_info
				.rewards
				.iter_mut()
				.for_each(|(reward_currency, (total_rewards, total_withdrawn_rewards))| {
					let reward_inflation = if total_shares.is_zero() {
						Zero::zero()
					} else {
						let proportion = FixedU128::checked_from_rational(add_amount, total_shares)
							.unwrap_or_else(FixedU128::max_value);
						proportion.saturating_mul_int(*total_rewards)
					};
					*total_rewards = total_rewards.saturating_add(reward_inflation);
					*total_withdrawn_rewards = total_withdrawn_rewards.saturating_add(reward_inflation);

					withdrawn_inflations.push((reward_currency, reward_inflation));
				});

			ShareAndWithdrawnReward::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
				*share = share.saturating_add(add_amount);
				withdrawn_inflations
					.into_iter()
					.for_each(|(reward_currency, reward_inflation)| {
						let withdrawn = if let Some(withdrawn_rewards) = withdrawn_rewards.get(reward_currency) {
							withdrawn_rewards.saturating_add(reward_inflation)
						} else {
							reward_inflation
						};
						withdrawn_rewards.insert(*reward_currency, withdrawn);
					});
			});
		});
	}

	pub fn remove_share(who: &T::AccountId, pool: &T::PoolId, remove_amount: T::Share) {
		if remove_amount.is_zero() {
			return;
		}

		// claim rewards firstly
		Self::claim_rewards(who, pool);

		ShareAndWithdrawnReward::<T>::mutate_exists(pool, who, |share_info| {
			if let Some((mut share, mut withdrawn_rewards)) = share_info.take() {
				let remove_amount = remove_amount.min(share);

				if remove_amount.is_zero() {
					return;
				}

				Pools::<T>::mutate(pool, |pool_info| {
					let proportion = FixedU128::checked_from_rational(remove_amount, share)
						.expect("share is gte remove_amount and not zero which checked before; qed");

					pool_info.total_shares = pool_info.total_shares.saturating_sub(remove_amount);

					withdrawn_rewards
						.iter_mut()
						.for_each(|(reward_currency, withdrawn_rewards)| {
							let withdrawn_rewards_to_remove = proportion.saturating_mul_int(*withdrawn_rewards);
							if let Some((total_rewards, total_withdrawn_rewards)) =
								pool_info.rewards.get_mut(reward_currency)
							{
								*total_rewards = total_rewards.saturating_sub(withdrawn_rewards_to_remove);
								*total_withdrawn_rewards =
									total_withdrawn_rewards.saturating_sub(withdrawn_rewards_to_remove);
							}
							*withdrawn_rewards = withdrawn_rewards.saturating_sub(withdrawn_rewards_to_remove);
						});
				});

				share = share.saturating_sub(remove_amount);
				if !share.is_zero() {
					*share_info = Some((share, withdrawn_rewards));
				}
			}
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
				pool_info
					.rewards
					.iter_mut()
					.for_each(|(reward_currency, (total_rewards, total_withdrawn_rewards))| {
						let current_withdrawn_rewards =
							withdrawn_rewards.get(reward_currency).copied().unwrap_or_default();

						let reward_to_withdraw = proportion
							.saturating_mul_int(*total_rewards)
							.saturating_sub(current_withdrawn_rewards)
							.min(total_rewards.saturating_sub(*total_withdrawn_rewards));

						if reward_to_withdraw.is_zero() {
							return;
						}

						*total_withdrawn_rewards = total_withdrawn_rewards.saturating_add(reward_to_withdraw);
						withdrawn_rewards.insert(
							*reward_currency,
							current_withdrawn_rewards.saturating_add(reward_to_withdraw),
						);

						// pay reward to `who`
						T::Handler::payout(who, pool, *reward_currency, reward_to_withdraw);
					});
			});
		});
	}
}
