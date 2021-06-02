//! # Xtokens Module
//!
//! ## Overview
//!
//! The xtokens module provides cross-chain token transfer functionality, by
//! cross-consensus messages(XCM).
//!
//! The xtokens module provides functions for
//! - Token transfer from parachains to relay chain.
//! - Token transfer between parachains, including relay chain tokens like DOT,
//!   KSM, and parachain tokens like ACA, aUSD.
//!
//! ## Interface
//!
//! ### Dispatchable functions
//!
//! - `transfer`: Transfer local assets with given `CurrencyId` and `Amount`.
//! - `transfer_multiasset`: Transfer `MultiAsset` assets.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::from_over_into)]
#![allow(clippy::unused_unit)]
#![allow(clippy::large_enum_variant)]

use frame_support::{pallet_prelude::*, traits::Get, transactional, Parameter};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Member, Zero},
	DispatchError,
};
use sp_std::prelude::*;

use xcm::v0::prelude::*;
use xcm_executor::traits::WeightBounds;

pub use module::*;
use orml_traits::location::{Parse, Reserve};

// mod mock;
// mod tests;

enum TransferKind {
	/// Transfer self reserve asset.
	SelfReserveAsset,
	/// To reserve location.
	ToReserve,
	/// To non-reserve location.
	ToNonReserve,
}
use TransferKind::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The balance type.
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Into<u128>;

		/// Currency Id.
		type CurrencyId: Parameter + Member + Clone;

		/// Convert `T::CurrencyId` to `MultiLocation`.
		type CurrencyIdConvert: Convert<Self::CurrencyId, Option<MultiLocation>>;

		/// Convert `T::AccountId` to `MultiLocation`.
		type AccountIdToMultiLocation: Convert<Self::AccountId, MultiLocation>;

		/// Self chain location.
		#[pallet::constant]
		type SelfLocation: Get<MultiLocation>;

		/// XCM executor.
		type XcmExecutor: ExecuteXcm<Self::Call>;

		/// Means of measuring the weight consumed by an XCM message locally.
		type Weigher: WeightBounds<Self::Call>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", T::CurrencyId = "CurrencyId", T::Balance = "Balance")]
	pub enum Event<T: Config> {
		/// Transferred. \[sender, currency_id, amount, dest\]
		Transferred(T::AccountId, T::CurrencyId, T::Balance, MultiLocation),
		/// Transferred `MultiAsset`. \[sender, asset, dest\]
		TransferredMultiAsset(T::AccountId, MultiAsset, MultiLocation),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Asset has no reserve location.
		AssetHasNoReserve,
		/// Not cross-chain transfer.
		NotCrossChainTransfer,
		/// Invalid transfer destination.
		InvalidDest,
		/// Currency is not cross-chain transferable.
		NotCrossChainTransferableCurrency,
		/// The message's weight could not be determined.
		UnweighableMessage,
		//TODO: more detailed err
		/// Xcm execution failed.
		XcmExecutionFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer native currencies.
		#[transactional]
		#[pallet::weight(Pallet::<T>::weight_of_transfer(currency_id.clone(),*amount, &dest))]
		pub fn transfer(
			origin: OriginFor<T>,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			dest: MultiLocation,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if amount == Zero::zero() {
				return Ok(());
			}

			let id: MultiLocation = T::CurrencyIdConvert::convert(currency_id.clone())
				.ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;
			let asset = MultiAsset::ConcreteFungible {
				id,
				amount: amount.into(),
			};
			Self::do_transfer_multiasset(who.clone(), asset, dest.clone())?;
			Self::deposit_event(Event::<T>::Transferred(who, currency_id, amount, dest));
			Ok(())
		}

		/// Transfer `MultiAsset`.
		#[transactional]
		#[pallet::weight(Pallet::<T>::weight_of_transfer_multiasset(&asset, &dest))]
		pub fn transfer_multiasset(origin: OriginFor<T>, asset: MultiAsset, dest: MultiLocation) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if Self::is_zero_amount(&asset) {
				return Ok(());
			}

			Self::do_transfer_multiasset(who.clone(), asset.clone(), dest.clone())?;
			Self::deposit_event(Event::<T>::TransferredMultiAsset(who, asset, dest));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Transfer `MultiAsset` without depositing event.
		fn do_transfer_multiasset(who: T::AccountId, asset: MultiAsset, dest: MultiLocation) -> DispatchResult {
			let (transfer_kind, reserve, dest, recipient) = Self::transfer_kind(&asset, &dest)?;
			let mut msg = match transfer_kind {
				SelfReserveAsset => Self::transfer_self_reserve_asset(asset, dest, recipient),
				ToReserve => Self::transfer_to_reserve(asset, dest, recipient),
				ToNonReserve => Self::transfer_to_non_reserve(asset, reserve, dest, recipient),
			};

			let origin_location = T::AccountIdToMultiLocation::convert(who);
			let weight = T::Weigher::weight(&mut msg).map_err(|()| Error::<T>::UnweighableMessage)?;
			let outcome = T::XcmExecutor::execute_xcm_in_credit(origin_location, msg, weight, weight);
			match outcome {
				Outcome::Complete(_w) => Ok(()),
				//TODO: more detailed err
				Outcome::Incomplete(_w, _e) => Err(Error::<T>::XcmExecutionFailed.into()),
				//TODO: more detailed err
				Outcome::Error(_e) => Err(Error::<T>::XcmExecutionFailed.into()),
			}
		}

		fn transfer_self_reserve_asset(
			asset: MultiAsset,
			dest: MultiLocation,
			recipient: MultiLocation,
		) -> Xcm<T::Call> {
			//TODO: buy execution order
			WithdrawAsset {
				assets: vec![asset],
				effects: vec![DepositReserveAsset {
					assets: vec![MultiAsset::All],
					dest,
					effects: Self::deposit_asset(recipient),
				}],
			}
		}

		fn transfer_to_reserve(asset: MultiAsset, reserve: MultiLocation, recipient: MultiLocation) -> Xcm<T::Call> {
			//TODO: buy execution order
			WithdrawAsset {
				assets: vec![asset],
				effects: vec![InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve,
					effects: Self::deposit_asset(recipient),
				}],
			}
		}

		fn transfer_to_non_reserve(
			asset: MultiAsset,
			reserve: MultiLocation,
			dest: MultiLocation,
			recipient: MultiLocation,
		) -> Xcm<T::Call> {
			//TODO: buy execution order
			let mut reanchored_dest = dest.clone();
			if reserve == Parent.into() {
				if let MultiLocation::X2(Parent, Parachain(id)) = dest {
					reanchored_dest = Parachain(id).into();
				}
			}

			WithdrawAsset {
				assets: vec![asset],
				effects: vec![InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve,
					effects: vec![DepositReserveAsset {
						assets: vec![MultiAsset::All],
						dest: reanchored_dest,
						effects: Self::deposit_asset(recipient),
					}],
				}],
			}
		}

		fn deposit_asset(recipient: MultiLocation) -> Vec<Order<()>> {
			vec![DepositAsset {
				assets: vec![MultiAsset::All],
				dest: recipient,
			}]
		}

		fn is_zero_amount(asset: &MultiAsset) -> bool {
			if let MultiAsset::ConcreteFungible { id: _, amount } = asset {
				if *amount == Zero::zero() {
					return true;
				}
			}

			if let MultiAsset::AbstractFungible { id: _, amount } = asset {
				if *amount == Zero::zero() {
					return true;
				}
			}

			false
		}

		/// Ensure has the `dest` has chain part and recipient part.
		fn ensure_valid_dest(
			dest: &MultiLocation,
		) -> sp_std::result::Result<(MultiLocation, MultiLocation), DispatchError> {
			if let (Some(dest), Some(recipient)) = (dest.chain_part(), dest.non_chain_part()) {
				Ok((dest, recipient))
			} else {
				Err(Error::<T>::InvalidDest.into())
			}
		}

		/// Get the transfer kind.
		///
		/// Returns `Err` if `asset` and `dest` combination doesn't make sense,
		/// else returns a tuple of:
		/// - `transfer_kind`.
		/// - asset's `reserve` parachain or relay chain location,
		/// - `dest` parachain or relay chain location.
		/// - `recipient` location.
		fn transfer_kind(
			asset: &MultiAsset,
			dest: &MultiLocation,
		) -> sp_std::result::Result<(TransferKind, MultiLocation, MultiLocation, MultiLocation), DispatchError> {
			let (dest, recipient) = Self::ensure_valid_dest(dest)?;

			let self_location = T::SelfLocation::get();
			ensure!(dest != self_location, Error::<T>::NotCrossChainTransfer);

			let reserve = asset.reserve().ok_or(Error::<T>::AssetHasNoReserve)?;
			let transfer_kind = if reserve == self_location {
				SelfReserveAsset
			} else if reserve == dest {
				ToReserve
			} else {
				ToNonReserve
			};
			Ok((transfer_kind, dest, reserve, recipient))
		}
	}

	// weights
	impl<T: Config> Pallet<T> {
		/// Returns weight of `transfer_multiasset` call.
		fn weight_of_transfer_multiasset(asset: &MultiAsset, dest: &MultiLocation) -> Weight {
			if let Ok((transfer_kind, dest, _, reserve)) = Self::transfer_kind(asset, dest) {
				let mut msg = match transfer_kind {
					SelfReserveAsset => WithdrawAsset {
						assets: sp_std::vec![asset.clone()],
						effects: sp_std::vec![DepositReserveAsset {
							assets: sp_std::vec![All],
							dest,
							effects: sp_std::vec![],
						}],
					},
					ToReserve | ToNonReserve => {
						WithdrawAsset {
							assets: sp_std::vec![asset.clone()],
							effects: sp_std::vec![InitiateReserveWithdraw {
								assets: sp_std::vec![All],
								// `dest` is always (equal to) `reserve` in both cases
								reserve,
								effects: sp_std::vec![],
							}],
						}
					}
				};
				T::Weigher::weight(&mut msg).map_or(Weight::max_value(), |w| 100_000_000 + w)
			} else {
				0
			}
		}

		/// Returns weight of `transfer` call.
		fn weight_of_transfer(currency_id: T::CurrencyId, amount: T::Balance, dest: &MultiLocation) -> Weight {
			if let Some(id) = T::CurrencyIdConvert::convert(currency_id) {
				let asset = MultiAsset::ConcreteFungible {
					id,
					amount: amount.into(),
				};
				Self::weight_of_transfer_multiasset(&asset, &dest)
			} else {
				0
			}
		}
	}
}
