#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::too_many_arguments)]

mod mock;
mod tests;

use frame_support::pallet_prelude::*;
use orml_traits::RewardHandler;
use parity_scale_codec::{FullCodec, HasCompact};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, Saturating, UniqueSaturatedInto, Zero},
	FixedPointOperand, RuntimeDebug, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PoolInfo<Share: HasCompact, Balance: HasCompact, CurrencyId: Ord> {
	/// Total shares amount
	pub total_shares: Share,
	/// Reward infos <reward_currency, (total_reward, total_withdrawn_reward)>
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

		type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

		/// The `RewardHandler`
		type Handler: RewardHandler<Self::AccountId, Self::CurrencyId, Balance = Self::Balance, PoolId = Self::PoolId>;
	}

	type WithdrawnRewards<T> = BTreeMap<<T as Config>::CurrencyId, <T as Config>::Balance>;

	#[pallet::error]
	pub enum Error<T> {
		/// Pool does not exist
		PoolDoesNotExist,
		ShareDoesNotExist,
		CanSplitOnlyLessThanShare,
	}

	/// Record reward pool info.
	///
	/// map PoolId => PoolInfo
	#[pallet::storage]
	#[pallet::getter(fn pool_infos)]
	pub type PoolInfos<T: Config> =
		StorageMap<_, Twox64Concat, T::PoolId, PoolInfo<T::Share, T::Balance, T::CurrencyId>, ValueQuery>;

	/// Record share amount, reward currency and withdrawn reward amount for
	/// specific `AccountId` under `PoolId`.
	///
	/// double_map (PoolId, AccountId) => (Share, BTreeMap<CurrencyId, Balance>)
	#[pallet::storage]
	#[pallet::getter(fn shares_and_withdrawn_rewards)]
	pub type SharesAndWithdrawnRewards<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::PoolId,
		Twox64Concat,
		T::AccountId,
		(T::Share, WithdrawnRewards<T>),
		ValueQuery,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);
}

impl<T: Config> Pallet<T> {
	pub fn accumulate_reward(
		pool: &T::PoolId,
		reward_currency: T::CurrencyId,
		reward_increment: T::Balance,
	) -> DispatchResult {
		if reward_increment.is_zero() {
			return Ok(());
		}
		PoolInfos::<T>::mutate_exists(pool, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;

			pool_info
				.rewards
				.entry(reward_currency)
				.and_modify(|(total_reward, _)| {
					*total_reward = total_reward.saturating_add(reward_increment);
				})
				.or_insert((reward_increment, Zero::zero()));

			Ok(())
		})
	}

	pub fn add_share(who: &T::AccountId, pool: &T::PoolId, add_amount: T::Share) {
		if add_amount.is_zero() {
			return;
		}

		PoolInfos::<T>::mutate(pool, |pool_info| {
			let initial_total_shares = pool_info.total_shares;
			pool_info.total_shares = pool_info.total_shares.saturating_add(add_amount);

			let mut withdrawn_inflation = Vec::<(T::CurrencyId, T::Balance)>::new();

			pool_info
				.rewards
				.iter_mut()
				.for_each(|(reward_currency, (total_reward, total_withdrawn_reward))| {
					let reward_inflation = if initial_total_shares.is_zero() {
						Zero::zero()
					} else {
						U256::from(add_amount.to_owned().saturated_into::<u128>())
							.saturating_mul(total_reward.to_owned().saturated_into::<u128>().into())
							.checked_div(initial_total_shares.to_owned().saturated_into::<u128>().into())
							.unwrap_or_default()
							.as_u128()
							.saturated_into()
					};
					*total_reward = total_reward.saturating_add(reward_inflation);
					*total_withdrawn_reward = total_withdrawn_reward.saturating_add(reward_inflation);

					withdrawn_inflation.push((*reward_currency, reward_inflation));
				});

			SharesAndWithdrawnRewards::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
				*share = share.saturating_add(add_amount);
				// update withdrawn inflation for each reward currency
				withdrawn_inflation
					.into_iter()
					.for_each(|(reward_currency, reward_inflation)| {
						withdrawn_rewards
							.entry(reward_currency)
							.and_modify(|withdrawn_reward| {
								*withdrawn_reward = withdrawn_reward.saturating_add(reward_inflation);
							})
							.or_insert(reward_inflation);
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

		SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |share_info| {
			if let Some((mut share, mut withdrawn_rewards)) = share_info.take() {
				let remove_amount = remove_amount.min(share);

				if remove_amount.is_zero() {
					return;
				}

				PoolInfos::<T>::mutate_exists(pool, |maybe_pool_info| {
					if let Some(mut pool_info) = maybe_pool_info.take() {
						let removing_share = U256::from(remove_amount.saturated_into::<u128>());

						pool_info.total_shares = pool_info.total_shares.saturating_sub(remove_amount);

						// update withdrawn rewards for each reward currency
						withdrawn_rewards
							.iter_mut()
							.for_each(|(reward_currency, withdrawn_reward)| {
								let withdrawn_reward_to_remove: T::Balance = removing_share
									.saturating_mul(withdrawn_reward.to_owned().saturated_into::<u128>().into())
									.checked_div(share.saturated_into::<u128>().into())
									.unwrap_or_default()
									.as_u128()
									.saturated_into();

								if let Some((total_reward, total_withdrawn_reward)) =
									pool_info.rewards.get_mut(reward_currency)
								{
									*total_reward = total_reward.saturating_sub(withdrawn_reward_to_remove);
									*total_withdrawn_reward =
										total_withdrawn_reward.saturating_sub(withdrawn_reward_to_remove);

									// remove if all reward is withdrawn
									if total_reward.is_zero() {
										pool_info.rewards.remove(reward_currency);
									}
								}
								*withdrawn_reward = withdrawn_reward.saturating_sub(withdrawn_reward_to_remove);
							});

						if !pool_info.total_shares.is_zero() {
							*maybe_pool_info = Some(pool_info);
						}
					}
				});

				share = share.saturating_sub(remove_amount);
				if !share.is_zero() {
					*share_info = Some((share, withdrawn_rewards));
				}
			}
		});
	}

	pub fn set_share(who: &T::AccountId, pool: &T::PoolId, new_share: T::Share) {
		let (share, _) = Self::shares_and_withdrawn_rewards(pool, who);

		if new_share > share {
			Self::add_share(who, pool, new_share.saturating_sub(share));
		} else {
			Self::remove_share(who, pool, share.saturating_sub(new_share));
		}
	}

	pub fn claim_rewards(who: &T::AccountId, pool: &T::PoolId) {
		SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |maybe_share_withdrawn| {
			if let Some((share, withdrawn_rewards)) = maybe_share_withdrawn {
				if share.is_zero() {
					return;
				}

				PoolInfos::<T>::mutate_exists(pool, |maybe_pool_info| {
					if let Some(pool_info) = maybe_pool_info {
						let total_shares = U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
						pool_info.rewards.iter_mut().for_each(
							|(reward_currency, (total_reward, total_withdrawn_reward))| {
								Self::claim_one(
									withdrawn_rewards,
									*reward_currency,
									share.to_owned(),
									total_reward.to_owned(),
									total_shares,
									total_withdrawn_reward,
									who,
									pool,
								);
							},
						);
					}
				});
			}
		});
	}

	pub fn claim_reward(who: &T::AccountId, pool: &T::PoolId, reward_currency: T::CurrencyId) {
		SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |maybe_share_withdrawn| {
			if let Some((share, withdrawn_rewards)) = maybe_share_withdrawn {
				if share.is_zero() {
					return;
				}

				PoolInfos::<T>::mutate(pool, |pool_info| {
					let total_shares = U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
					if let Some((total_reward, total_withdrawn_reward)) = pool_info.rewards.get_mut(&reward_currency) {
						Self::claim_one(
							withdrawn_rewards,
							reward_currency,
							share.to_owned(),
							total_reward.to_owned(),
							total_shares,
							total_withdrawn_reward,
							who,
							pool,
						);
					}
				});
			}
		});
	}

	/// Splits share into two parts.
	///
	/// `move_share` - amount of share to remove and put into `other` share
	/// `other` - new account who will own new share
	///
	/// Similar too claim and add 2 shares later, but does not requires pool
	/// inflation and is more efficient.
	pub fn transfer_share_and_rewards(
		who: &T::AccountId,
		pool: &T::PoolId,
		move_share: T::Share,
		other: &T::AccountId,
	) -> DispatchResult {
		SharesAndWithdrawnRewards::<T>::mutate(pool, other, |increased_share| {
			let (increased_share, increased_rewards) = increased_share;
			SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |share| {
				let (share, rewards) = share.as_mut().ok_or(Error::<T>::ShareDoesNotExist)?;
				ensure!(move_share < *share, Error::<T>::CanSplitOnlyLessThanShare);
				for (reward_currency, balance) in rewards {
					// u128 * u128 is always less than u256
					// move_share / share always less then 1 and share > 0
					// so final results is computable and is always less or equal than u128
					let move_balance = U256::from(balance.to_owned().saturated_into::<u128>())
						* U256::from(move_share.to_owned().saturated_into::<u128>())
						/ U256::from(share.to_owned().saturated_into::<u128>());
					let move_balance: Option<u128> = move_balance.try_into().ok();
					if let Some(move_balance) = move_balance {
						let move_balance: T::Balance = move_balance.unique_saturated_into();
						*balance = balance.saturating_sub(move_balance);
						increased_rewards
							.entry(*reward_currency)
							.and_modify(|increased_reward| {
								*increased_reward = increased_reward.saturating_add(move_balance);
							})
							.or_insert(move_balance);
					}
				}
				*share = share.saturating_sub(move_share);
				*increased_share = increased_share.saturating_add(move_share);
				Ok(())
			})
		})
	}

	#[allow(clippy::too_many_arguments)] // just we need to have all these to do the stuff
	fn claim_one(
		withdrawn_rewards: &mut BTreeMap<T::CurrencyId, T::Balance>,
		reward_currency: T::CurrencyId,
		share: T::Share,
		total_reward: T::Balance,
		total_shares: U256,
		total_withdrawn_reward: &mut T::Balance,
		who: &T::AccountId,
		pool: &T::PoolId,
	) {
		let withdrawn_reward = withdrawn_rewards.get(&reward_currency).copied().unwrap_or_default();
		let reward_to_withdraw = Self::reward_to_withdraw(
			share,
			total_reward,
			total_shares,
			withdrawn_reward,
			total_withdrawn_reward.to_owned(),
		);
		if !reward_to_withdraw.is_zero() {
			*total_withdrawn_reward = total_withdrawn_reward.saturating_add(reward_to_withdraw);
			withdrawn_rewards.insert(reward_currency, withdrawn_reward.saturating_add(reward_to_withdraw));

			// pay reward to `who`
			T::Handler::payout(who, pool, reward_currency, reward_to_withdraw);
		}
	}

	fn reward_to_withdraw(
		share: T::Share,
		total_reward: T::Balance,
		total_shares: U256,
		withdrawn_reward: T::Balance,
		total_withdrawn_reward: T::Balance,
	) -> T::Balance {
		let total_reward_proportion: T::Balance = U256::from(share.saturated_into::<u128>())
			.saturating_mul(U256::from(total_reward.saturated_into::<u128>()))
			.checked_div(total_shares)
			.unwrap_or_default()
			.as_u128()
			.unique_saturated_into();
		total_reward_proportion
			.saturating_sub(withdrawn_reward)
			.min(total_reward.saturating_sub(total_withdrawn_reward))
	}
}
