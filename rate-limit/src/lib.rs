//! # Rate Limit
//! A module to provide rate limit for arbitrary type Key and integer type Value
//!
//! - [`Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! This module is a utility to provide rate limiter for arbitrary type Key and
//! integer type Value, which can config limit rule to produce quota and consume
//! quota, and expose quota consuming checking and whitelist that can bypass
//! checks.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{pallet_prelude::*, traits::UnixTime, transactional, BoundedVec};
use frame_system::pallet_prelude::*;
use orml_traits::{RateLimiter, RateLimiterError};
use orml_utilities::OrderedSet;
use parity_scale_codec::MaxEncodedLen;
use scale_info::TypeInfo;
use sp_runtime::traits::{BlockNumberProvider, SaturatedConversion, Zero};
use sp_std::{prelude::*, vec::Vec};

pub use module::*;
pub use weights::WeightInfo;

mod mock;
mod tests;
pub mod weights;

#[frame_support::pallet]
pub mod module {
	use super::*;

	/// Period type.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum Period {
		Blocks(u64),
		Seconds(u64),
	}

	/// Limit rules type.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum RateLimitRule {
		/// Each period to reset remainer quota to `quota` amount.
		/// `can_consume` check return true when the remainer quota gte the
		/// consume amount.
		PerPeriod { period: Period, quota: u128 },
		/// Each period to increase `quota_increment` amount to remainer quota
		/// and keep remainer quota lte `max_quota`.
		/// `can_consume` check return true when the remainer quota gte the
		/// consume amount.
		TokenBucket {
			period: Period,
			quota_increment: u128,
			max_quota: u128,
		},
		/// can_consume check return true always.
		Unlimited,
		/// can_consume check return false always.
		NotAllowed,
	}

	/// The maximum length of KeyFilter inner key.
	pub const MAX_FILTER_KEY_LENGTH: u32 = 256;

	/// Match rules to fitler key is in bypass whitelist.
	#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum KeyFilter {
		/// If the encoded key is equal to the vec, the key is in whitelist.
		Match(BoundedVec<u8, ConstU32<MAX_FILTER_KEY_LENGTH>>),
		/// If the encoded key starts with the vec, the key is in whitelist.
		StartsWith(BoundedVec<u8, ConstU32<MAX_FILTER_KEY_LENGTH>>),
		/// If the encoded key ends with the vec, the key is in whitelist.
		EndsWith(BoundedVec<u8, ConstU32<MAX_FILTER_KEY_LENGTH>>),
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin represented Governance.
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type RateLimiterId: Parameter + Member + Copy + TypeInfo;

		/// The maximum number of KeyFilter configured to a RateLimiterId.
		#[pallet::constant]
		type MaxWhitelistFilterCount: Get<u32>;

		/// Time used for calculate quota.
		type UnixTime: UnixTime;

		// The block number provider
		type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Invalid rate limit rule.
		InvalidRateLimitRule,
		/// The KeyFilter has been existed already.
		FilterExisted,
		/// The KeyFilter doesn't exist.
		FilterNotExisted,
		/// Exceed the allowed maximum number of KeyFilter configured to a
		/// RateLimiterId.
		MaxFilterExceeded,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The rate limit rule has updated.
		RateLimitRuleUpdated {
			rate_limiter_id: T::RateLimiterId,
			encoded_key: Vec<u8>,
			update: Option<RateLimitRule>,
		},
		/// The whitelist of bypass rate limit has been added new KeyFilter.
		WhitelistFilterAdded { rate_limiter_id: T::RateLimiterId },
		/// The whitelist of bypass rate limit has been removed a KeyFilter.
		WhitelistFilterRemoved { rate_limiter_id: T::RateLimiterId },
		/// The whitelist of bypass rate limit has been reset.
		WhitelistFilterReset { rate_limiter_id: T::RateLimiterId },
	}

	/// The rate limit rule for specific RateLimiterId and encoded key.
	///
	/// RateLimitRules: double_map RateLimiterId, EncodedKey => RateLimitRule
	#[pallet::storage]
	#[pallet::getter(fn rate_limit_rules)]
	pub type RateLimitRules<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::RateLimiterId, Blake2_128Concat, Vec<u8>, RateLimitRule, OptionQuery>;

	/// The quota for specific RateLimiterId and encoded key.
	///
	/// RateLimitQuota: double_map RateLimiterId, EncodedKey =>
	/// (LastUpdatedBlockOrTime, RemainerQuota)
	#[pallet::storage]
	#[pallet::getter(fn rate_limit_quota)]
	pub type RateLimitQuota<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::RateLimiterId, Blake2_128Concat, Vec<u8>, (u64, u128), ValueQuery>;

	/// The rules to filter if key is in whitelist for specific RateLimiterId.
	///
	/// LimitWhitelist: map RateLimiterId => Vec<KeyFilter>
	#[pallet::storage]
	#[pallet::getter(fn limit_whitelist)]
	pub type LimitWhitelist<T: Config> =
		StorageMap<_, Twox64Concat, T::RateLimiterId, OrderedSet<KeyFilter, T::MaxWhitelistFilterCount>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Config the rate limit rule.
		///
		/// Requires `GovernanceOrigin`
		///
		/// Parameters:
		/// - `rate_limiter_id`: rate limiter id.
		/// - `encoded key`: the encoded key to limit.
		/// - `update`: the RateLimitRule to config, None will remove current
		///   config.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::update_rate_limit_rule())]
		#[transactional]
		pub fn update_rate_limit_rule(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			encoded_key: Vec<u8>,
			update: Option<RateLimitRule>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			RateLimitRules::<T>::try_mutate_exists(rate_limiter_id, &encoded_key, |maybe_limit| -> DispatchResult {
				*maybe_limit = update.clone();

				if let Some(rule) = maybe_limit {
					match rule {
						RateLimitRule::PerPeriod { period, quota } => {
							match period {
								Period::Blocks(blocks_count) => {
									ensure!(!blocks_count.is_zero(), Error::<T>::InvalidRateLimitRule);
								}
								Period::Seconds(secs_count) => {
									ensure!(!secs_count.is_zero(), Error::<T>::InvalidRateLimitRule);
								}
							}

							ensure!(!quota.is_zero(), Error::<T>::InvalidRateLimitRule);
						}
						RateLimitRule::TokenBucket {
							period,
							quota_increment,
							max_quota,
						} => {
							match period {
								Period::Blocks(blocks_count) => {
									ensure!(!blocks_count.is_zero(), Error::<T>::InvalidRateLimitRule);
								}
								Period::Seconds(secs_count) => {
									ensure!(!secs_count.is_zero(), Error::<T>::InvalidRateLimitRule);
								}
							}

							ensure!(
								!quota_increment.is_zero() && !max_quota.is_zero(),
								Error::<T>::InvalidRateLimitRule
							);
						}
						RateLimitRule::Unlimited => {}
						RateLimitRule::NotAllowed => {}
					}
				}

				// always reset RateLimitQuota.
				RateLimitQuota::<T>::remove(rate_limiter_id, &encoded_key);

				Self::deposit_event(Event::RateLimitRuleUpdated {
					rate_limiter_id,
					encoded_key: encoded_key.clone(),
					update,
				});

				Ok(())
			})
		}

		/// Add whitelist filter rule.
		///
		/// Requires `GovernanceOrigin`
		///
		/// Parameters:
		/// - `rate_limiter_id`: rate limiter id.
		/// - `key_filter`: filter rule to add.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::add_whitelist())]
		#[transactional]
		pub fn add_whitelist(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			key_filter: KeyFilter,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			LimitWhitelist::<T>::try_mutate(rate_limiter_id, |whitelist| -> DispatchResult {
				ensure!(!whitelist.contains(&key_filter), Error::<T>::FilterExisted);
				let inserted = whitelist.insert(key_filter);
				ensure!(inserted, Error::<T>::MaxFilterExceeded);

				Self::deposit_event(Event::WhitelistFilterAdded { rate_limiter_id });
				Ok(())
			})
		}

		/// Remove whitelist filter rule.
		///
		/// Requires `GovernanceOrigin`
		///
		/// Parameters:
		/// - `rate_limiter_id`: rate limiter id.
		/// - `key_filter`: filter rule to remove.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::remove_whitelist())]
		#[transactional]
		pub fn remove_whitelist(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			key_filter: KeyFilter,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			LimitWhitelist::<T>::try_mutate(rate_limiter_id, |whitelist| -> DispatchResult {
				ensure!(whitelist.contains(&key_filter), Error::<T>::FilterNotExisted);
				whitelist.remove(&key_filter);

				Self::deposit_event(Event::WhitelistFilterRemoved { rate_limiter_id });
				Ok(())
			})
		}

		/// Resett whitelist filter rule.
		///
		/// Requires `GovernanceOrigin`
		///
		/// Parameters:
		/// - `rate_limiter_id`: rate limiter id.
		/// - `new_list`: the filter rule list to reset.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::reset_whitelist())]
		#[transactional]
		pub fn reset_whitelist(
			origin: OriginFor<T>,
			rate_limiter_id: T::RateLimiterId,
			new_list: Vec<KeyFilter>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let whitelist: BoundedVec<KeyFilter, T::MaxWhitelistFilterCount> =
				BoundedVec::try_from(new_list).map_err(|_| Error::<T>::MaxFilterExceeded)?;
			let ordered_set: OrderedSet<KeyFilter, T::MaxWhitelistFilterCount> = whitelist.into();
			LimitWhitelist::<T>::insert(rate_limiter_id, ordered_set);

			Self::deposit_event(Event::WhitelistFilterReset { rate_limiter_id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Access the RateLimitQuota, if RateLimitRule will produce new quota,
		/// update RateLimitQuota and then return remainer_quota
		pub fn access_remainer_quota_after_update(
			rate_limit_rule: RateLimitRule,
			limiter_id: &T::RateLimiterId,
			encoded_key: &Vec<u8>,
		) -> u128 {
			RateLimitQuota::<T>::mutate(limiter_id, encoded_key, |(last_updated, remainer_quota)| -> u128 {
				match rate_limit_rule {
					RateLimitRule::PerPeriod { period, quota } => {
						let (now, count): (u64, u64) = match period {
							Period::Blocks(blocks_count) => (
								T::BlockNumberProvider::current_block_number().saturated_into(),
								blocks_count,
							),
							Period::Seconds(secs_count) => (T::UnixTime::now().as_secs(), secs_count),
						};

						let interval: u64 = now.saturating_sub(*last_updated);
						if interval >= count {
							*last_updated = now;
							*remainer_quota = quota;
						}
					}

					RateLimitRule::TokenBucket {
						period,
						quota_increment,
						max_quota,
					} => {
						let (now, count): (u64, u64) = match period {
							Period::Blocks(blocks_count) => (
								T::BlockNumberProvider::current_block_number().saturated_into(),
								blocks_count,
							),
							Period::Seconds(secs_count) => (T::UnixTime::now().as_secs(), secs_count),
						};

						let interval: u64 = now.saturating_sub(*last_updated);
						if !count.is_zero() && interval >= count {
							let inc_times: u128 = interval
								.checked_div(count)
								.expect("already ensure count is not zero; qed")
								.saturated_into();

							*last_updated = now;
							*remainer_quota = quota_increment
								.saturating_mul(inc_times)
								.saturating_add(*remainer_quota)
								.min(max_quota);
						}
					}

					RateLimitRule::Unlimited | RateLimitRule::NotAllowed => {}
				}

				*remainer_quota
			})
		}
	}

	impl<T: Config> RateLimiter for Pallet<T> {
		type RateLimiterId = T::RateLimiterId;

		fn is_whitelist(limiter_id: Self::RateLimiterId, key: impl Encode) -> bool {
			let encode_key: Vec<u8> = key.encode();

			for key_filter in LimitWhitelist::<T>::get(limiter_id).0 {
				match key_filter {
					KeyFilter::Match(bounded_vec) => {
						if encode_key == bounded_vec.into_inner() {
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

		fn can_consume(limiter_id: Self::RateLimiterId, key: impl Encode, value: u128) -> Result<(), RateLimiterError> {
			let encoded_key: Vec<u8> = key.encode();

			let allowed = match RateLimitRules::<T>::get(limiter_id, &encoded_key) {
				Some(rate_limit_rule @ RateLimitRule::PerPeriod { .. })
				| Some(rate_limit_rule @ RateLimitRule::TokenBucket { .. }) => {
					let remainer_quota =
						Self::access_remainer_quota_after_update(rate_limit_rule, &limiter_id, &encoded_key);

					value <= remainer_quota
				}
				Some(RateLimitRule::Unlimited) => true,
				Some(RateLimitRule::NotAllowed) => {
					// always return false, even if the value is zero.
					false
				}
				None => {
					// if doesn't rate limit rule, always return true.
					true
				}
			};

			ensure!(allowed, RateLimiterError::ExceedLimit);

			Ok(())
		}

		fn consume(limiter_id: Self::RateLimiterId, key: impl Encode, value: u128) {
			let encoded_key: Vec<u8> = key.encode();

			match RateLimitRules::<T>::get(limiter_id, &encoded_key) {
				Some(RateLimitRule::PerPeriod { .. }) | Some(RateLimitRule::TokenBucket { .. }) => {
					// consume remainer quota in these situation.
					RateLimitQuota::<T>::mutate(limiter_id, &encoded_key, |(_, remainer_quota)| {
						*remainer_quota = (*remainer_quota).saturating_sub(value);
					});
				}
				_ => {}
			};
		}
	}
}
