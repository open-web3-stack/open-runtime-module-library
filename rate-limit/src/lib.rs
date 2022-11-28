//! # Oracle
//! A module to allow oracle operators to feed external data.
//!
//! - [`Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! This module exposes capabilities for oracle operators to feed external
//! offchain data. The raw values can be combined to provide an aggregated
//! value.
//!
//! The data is valid only if feeded by an authorized operator.
//! `pallet_membership` in FRAME can be used to as source of `T::Members`.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, traits::UnixTime, transactional, BoundedVec};
use frame_system::pallet_prelude::*;
use orml_traits::{RateLimiter, RateLimiterError};
use scale_info::TypeInfo;
use sp_runtime::traits::{MaybeSerializeDeserialize, SaturatedConversion, Zero};
use sp_std::{prelude::*, vec::Vec};

//pub use module::*;
// pub use weights::WeightInfo;

// mod mock;
// mod tests;
// pub mod weights;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum RateLimit {
		PerBlocks {
			blocks_count: u64,
			quota: u128,
		},
		PerSeconds {
			secs_count: u64,
			quota: u128,
		},
		TokenBucket {
			blocks_count: u64,
			quota_increment: u128,
			max_quota: u128,
		},
		Unlimited,
		NotAllowed,
	}

	impl Default for RateLimit {
		fn default() -> Self {
			RateLimit::Unlimited
		}
	}

	#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum KeyFilter {
		Match(Vec<u8>),
		StartsWith(Vec<u8>),
		EndsWith(Vec<u8>),
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin represented Governance
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type RateLimiterId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + TypeInfo;

		#[pallet::constant]
		type MaxWhitelistFilterCount: Get<u32>;

		type UnixTime: UnixTime;

		// /// Weight information for the extrinsics in this module.
		// type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		FilterExisted,
		FilterNotExisted,
		MaxFilterExceeded,
		DecodeKeyFailed,
		InvalidRateLimit,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		LimiRateUpdated {
			rate_limiter_id: T::RateLimiterId,
			key: Vec<u8>,
			update: Option<RateLimit>,
		},
		WhitelistFilterAdded {
			rate_limiter_id: T::RateLimiterId,
		},
		WhitelistFilterRemoved {
			rate_limiter_id: T::RateLimiterId,
		},
		WhitelistFilterReset {
			rate_limiter_id: T::RateLimiterId,
		},
	}

	#[pallet::storage]
	#[pallet::getter(fn rate_limits)]
	pub type RateLimits<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::RateLimiterId, Twox64Concat, Vec<u8>, RateLimit, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn rate_limit_quota)]
	pub type RateLimitQuota<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::RateLimiterId, Twox64Concat, Vec<u8>, (u64, u128), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bypass_limit_whitelist)]
	pub type BypassLimitWhitelist<T: Config> =
		StorageMap<_, Twox64Concat, T::RateLimiterId, BoundedVec<KeyFilter, T::MaxWhitelistFilterCount>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		#[transactional]
		pub fn update_rate_limit(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			key: Vec<u8>,
			update: Option<RateLimit>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			RateLimits::<T>::try_mutate_exists(&rate_limiter_id, key.clone(), |maybe_limit| -> DispatchResult {
				*maybe_limit = update.clone();

				if let Some(rate_limit) = maybe_limit {
					match rate_limit {
						RateLimit::PerBlocks { blocks_count, quota } => {
							ensure!(
								!blocks_count.is_zero() && !quota.is_zero(),
								Error::<T>::InvalidRateLimit
							);
						}
						RateLimit::PerSeconds { secs_count, quota } => {
							ensure!(!secs_count.is_zero() && !quota.is_zero(), Error::<T>::InvalidRateLimit);
						}
						RateLimit::TokenBucket {
							blocks_count,
							quota_increment,
							max_quota,
						} => {
							ensure!(
								!blocks_count.is_zero() && !quota_increment.is_zero() && !max_quota.is_zero(),
								Error::<T>::InvalidRateLimit
							);
						}
						_ => {}
					}
				}

				// always reset RateLimitQuota.
				RateLimitQuota::<T>::remove(&rate_limiter_id, &key);

				Self::deposit_event(Event::LimiRateUpdated {
					rate_limiter_id,
					key,
					update,
				});

				Ok(())
			})
		}

		#[pallet::weight(10000)]
		#[transactional]
		pub fn set_rate_limit_quota(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			key: Vec<u8>,
			last_update: u64,
			amount: u128,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			RateLimitQuota::<T>::insert(rate_limiter_id, key, (last_update, amount));

			Ok(())
		}

		#[pallet::weight(10000)]
		#[transactional]
		pub fn add_whitelist(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			key_filter: KeyFilter,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			BypassLimitWhitelist::<T>::try_mutate(rate_limiter_id, |whitelist| -> DispatchResult {
				let location = whitelist
					.binary_search(&key_filter)
					.err()
					.ok_or(Error::<T>::FilterExisted)?;
				whitelist
					.try_insert(location, key_filter)
					.map_err(|_| Error::<T>::MaxFilterExceeded)?;

				Self::deposit_event(Event::WhitelistFilterAdded { rate_limiter_id });
				Ok(())
			})
		}

		#[pallet::weight(10000)]
		#[transactional]
		pub fn remove_whitelist(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			key_filter: KeyFilter,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			BypassLimitWhitelist::<T>::try_mutate(rate_limiter_id, |whitelist| -> DispatchResult {
				let location = whitelist
					.binary_search(&key_filter)
					.ok()
					.ok_or(Error::<T>::FilterExisted)?;
				whitelist.remove(location);

				Self::deposit_event(Event::WhitelistFilterRemoved { rate_limiter_id });
				Ok(())
			})
		}

		#[pallet::weight(10000)]
		#[transactional]
		pub fn reset_whitelist(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			new_list: Vec<KeyFilter>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let mut whitelist: BoundedVec<KeyFilter, T::MaxWhitelistFilterCount> =
				BoundedVec::try_from(new_list).map_err(|_| Error::<T>::MaxFilterExceeded)?;
			whitelist.sort();
			BypassLimitWhitelist::<T>::insert(rate_limiter_id, whitelist);

			Self::deposit_event(Event::WhitelistFilterReset { rate_limiter_id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_remainer_quota_after_update(
			rate_limit: RateLimit,
			limiter_id: &T::RateLimiterId,
			encoded_key: &Vec<u8>,
		) -> u128 {
			RateLimitQuota::<T>::mutate(limiter_id, encoded_key, |(last_updated, remainer_quota)| -> u128 {
				match rate_limit {
					RateLimit::PerBlocks { blocks_count, quota } => {
						let now: u64 = frame_system::Pallet::<T>::block_number().saturated_into();
						let interval: u64 = now.saturating_sub(*last_updated);
						if interval >= blocks_count {
							*last_updated = now;
							*remainer_quota = quota;
						}
					}

					RateLimit::PerSeconds { secs_count, quota } => {
						let now: u64 = T::UnixTime::now().as_secs();
						let interval: u64 = now.saturating_sub(*last_updated);
						if interval >= secs_count {
							*last_updated = now;
							*remainer_quota = quota;
						}
					}

					RateLimit::TokenBucket {
						blocks_count,
						quota_increment,
						max_quota,
					} => {
						let now: u64 = frame_system::Pallet::<T>::block_number().saturated_into();
						let interval: u64 = now.saturating_sub(*last_updated);
						if !blocks_count.is_zero() && interval >= blocks_count {
							let inc_times: u128 = interval
								.checked_div(blocks_count)
								.expect("already ensure blocks_count is not zero; qed")
								.saturated_into();

							*last_updated = now;
							*remainer_quota = quota_increment
								.saturating_mul(inc_times)
								.saturating_add(*remainer_quota)
								.min(max_quota);
						}
					}

					_ => {}
				}

				*remainer_quota
			})
		}
	}

	impl<T: Config> RateLimiter for Pallet<T> {
		type RateLimiterId = T::RateLimiterId;

		fn bypass_limit(limiter_id: Self::RateLimiterId, key: impl Encode) -> bool {
			let encode_key: Vec<u8> = key.encode();

			for key_filter in BypassLimitWhitelist::<T>::get(limiter_id) {
				match key_filter {
					KeyFilter::Match(vec) => {
						if encode_key == vec {
							return true;
						}
					}
					KeyFilter::StartsWith(prefix) => {
						if encode_key.starts_with(&prefix) {
							return true;
						}
					}
					KeyFilter::EndsWith(postfix) => {
						if encode_key.ends_with(&postfix) {
							return true;
						}
					}
				}
			}

			false
		}

		fn is_allowed(limiter_id: Self::RateLimiterId, key: impl Encode, value: u128) -> Result<(), RateLimiterError> {
			let encoded_key: Vec<u8> = key.encode();

			let allowed = match RateLimits::<T>::get(&limiter_id, &encoded_key) {
				Some(rate_limit @ RateLimit::PerBlocks { .. })
				| Some(rate_limit @ RateLimit::PerSeconds { .. })
				| Some(rate_limit @ RateLimit::TokenBucket { .. }) => {
					let remainer_quota = Self::get_remainer_quota_after_update(rate_limit, &limiter_id, &encoded_key);
					value <= remainer_quota
				}
				Some(RateLimit::Unlimited) => false,
				Some(RateLimit::NotAllowed) => true,
				None => {
					// if not defined limit for key, allow it.
					true
				}
			};

			ensure!(allowed, RateLimiterError::ExceedLimit);
			Ok(())
		}

		fn record(limiter_id: Self::RateLimiterId, key: impl Encode, value: u128) {
			let encoded_key: Vec<u8> = key.encode();

			match RateLimits::<T>::get(&limiter_id, &encoded_key) {
				Some(RateLimit::PerBlocks { .. })
				| Some(RateLimit::PerSeconds { .. })
				| Some(RateLimit::TokenBucket { .. }) => {
					// consume remainer quota only in these situation.
					RateLimitQuota::<T>::mutate(&limiter_id, &encoded_key, |(_, remainer_quota)| {
						*remainer_quota = (*remainer_quota).saturating_sub(value);
					});
				}
				_ => {}
			};
		}
	}
}
