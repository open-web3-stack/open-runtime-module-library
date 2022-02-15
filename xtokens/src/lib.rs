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

use frame_support::{log, pallet_prelude::*, require_transactional, traits::Get, transactional, Parameter};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Member, Zero},
	DispatchError,
};
use sp_std::{prelude::*, result::Result};

use xcm::prelude::*;
use xcm_executor::traits::{InvertLocation, WeightBounds};

pub use module::*;
use orml_traits::{
	location::{Parse, Reserve},
	XcmTransfer,
};

mod mock;
mod tests;

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

		/// Base XCM weight.
		///
		/// The actually weight for an XCM message is `T::BaseXcmWeight +
		/// T::Weigher::weight(&msg)`.
		#[pallet::constant]
		type BaseXcmWeight: Get<Weight>;

		/// Means of inverting a location.
		type LocationInverter: InvertLocation;

		/// The maximum number of distinct assets allowed to be transferred in a
		/// single helper extrinsic.
		type MaxAssetsForTransfer: Get<usize>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Transferred.
		Transferred {
			sender: T::AccountId,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			dest: MultiLocation,
		},
		/// Transferred with fee.
		TransferredWithFee {
			sender: T::AccountId,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			fee: T::Balance,
			dest: MultiLocation,
		},
		/// Transferred `MultiAsset`.
		TransferredMultiAsset {
			sender: T::AccountId,
			asset: MultiAsset,
			dest: MultiLocation,
		},
		/// Transferred `MultiAsset` with fee.
		TransferredMultiAssetWithFee {
			sender: T::AccountId,
			asset: MultiAsset,
			fee: MultiAsset,
			dest: MultiLocation,
		},
		/// Transferred `MultiAsset` with fee.
		TransferredMultiCurrencies {
			sender: T::AccountId,
			currencies: Vec<(T::CurrencyId, T::Balance)>,
			dest: MultiLocation,
		},
		/// Transferred `MultiAsset` with fee.
		TransferredMultiAssets {
			sender: T::AccountId,
			assets: MultiAssets,
			dest: MultiLocation,
		},
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
		// TODO: expand into XcmExecutionFailed(XcmError) after https://github.com/paritytech/substrate/pull/10242 done
		/// XCM execution failed.
		XcmExecutionFailed,
		/// Could not re-anchor the assets to declare the fees for the
		/// destination chain.
		CannotReanchor,
		/// Could not get ancestry of asset reserve location.
		InvalidAncestry,
		/// Not fungible asset.
		NotFungible,
		/// The destination `MultiLocation` provided cannot be inverted.
		DestinationNotInvertible,
		/// The version of the `Versioned` value used is not able to be
		/// interpreted.
		BadVersion,
		/// We tried sending distinct asset and fee but they have different
		/// reserve chains
		DistinctReserveForAssetAndFee,
		/// The fee amount was zero when the fee specification extrinsic is
		/// being used.
		FeeCannotBeZero,
		/// The number of assets to be sent is over the maximum
		TooManyAssetsBeingSent,
		/// The specified index does not exist in a MultiAssets struct
		AssetIndexNonExistent,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer native currencies.
		///
		/// `dest_weight` is the weight for XCM execution on the dest chain, and
		/// it would be charged from the transferred assets. If set below
		/// requirements, the execution may fail and assets wouldn't be
		/// received.
		///
		/// It's a no-op if any error on local XCM execution or message sending.
		/// Note sending assets out per se doesn't guarantee they would be
		/// received. Receiving depends on if the XCM message could be delivered
		/// by the network, and if the receiving chain would handle
		/// messages correctly.
		#[pallet::weight(Pallet::<T>::weight_of_transfer(currency_id.clone(), *amount, dest))]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			dest: Box<VersionedMultiLocation>,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let dest: MultiLocation = (*dest).try_into().map_err(|()| Error::<T>::BadVersion)?;
			Self::do_transfer(who, currency_id, amount, dest, dest_weight)
		}

		/// Transfer `MultiAsset`.
		///
		/// `dest_weight` is the weight for XCM execution on the dest chain, and
		/// it would be charged from the transferred assets. If set below
		/// requirements, the execution may fail and assets wouldn't be
		/// received.
		///
		/// It's a no-op if any error on local XCM execution or message sending.
		/// Note sending assets out per se doesn't guarantee they would be
		/// received. Receiving depends on if the XCM message could be delivered
		/// by the network, and if the receiving chain would handle
		/// messages correctly.
		#[pallet::weight(Pallet::<T>::weight_of_transfer_multiasset(asset, dest))]
		#[transactional]
		pub fn transfer_multiasset(
			origin: OriginFor<T>,
			asset: Box<VersionedMultiAsset>,
			dest: Box<VersionedMultiLocation>,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let asset: MultiAsset = (*asset).try_into().map_err(|()| Error::<T>::BadVersion)?;
			let dest: MultiLocation = (*dest).try_into().map_err(|()| Error::<T>::BadVersion)?;
			Self::do_transfer_multiasset(who, asset, dest, dest_weight)
		}

		/// Transfer native currencies specifying the fee and amount as
		/// separate.
		///
		/// `dest_weight` is the weight for XCM execution on the dest chain, and
		/// it would be charged from the transferred assets. If set below
		/// requirements, the execution may fail and assets wouldn't be
		/// received.
		///
		/// `fee` is the amount to be spent to pay for execution in destination
		/// chain. Both fee and amount will be subtracted form the callers
		/// balance.
		///
		/// If `fee` is not high enough to cover for the execution costs in the
		/// destination chain, then the assets will be trapped in the
		/// destination chain
		///
		/// It's a no-op if any error on local XCM execution or message sending.
		/// Note sending assets out per se doesn't guarantee they would be
		/// received. Receiving depends on if the XCM message could be delivered
		/// by the network, and if the receiving chain would handle
		/// messages correctly.
		#[pallet::weight(Pallet::<T>::weight_of_transfer(currency_id.clone(), *amount, dest))]
		#[transactional]
		pub fn transfer_with_fee(
			origin: OriginFor<T>,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			fee: T::Balance,
			dest: Box<VersionedMultiLocation>,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let dest: MultiLocation = (*dest).try_into().map_err(|()| Error::<T>::BadVersion)?;
			// Zero fee is an error
			if fee.is_zero() {
				return Err(Error::<T>::FeeCannotBeZero.into());
			}

			Self::do_transfer_with_fee(who, currency_id, amount, fee, dest, dest_weight)
		}

		/// Transfer `MultiAsset` specifying the fee and amount as separate.
		///
		/// `dest_weight` is the weight for XCM execution on the dest chain, and
		/// it would be charged from the transferred assets. If set below
		/// requirements, the execution may fail and assets wouldn't be
		/// received.
		///
		/// `fee` is the multiasset to be spent to pay for execution in
		/// destination chain. Both fee and amount will be subtracted form the
		/// callers balance For now we only accept fee and asset having the same
		/// `MultiLocation` id.
		///
		/// If `fee` is not high enough to cover for the execution costs in the
		/// destination chain, then the assets will be trapped in the
		/// destination chain
		///
		/// It's a no-op if any error on local XCM execution or message sending.
		/// Note sending assets out per se doesn't guarantee they would be
		/// received. Receiving depends on if the XCM message could be delivered
		/// by the network, and if the receiving chain would handle
		/// messages correctly.
		#[pallet::weight(Pallet::<T>::weight_of_transfer_multiasset(asset, dest))]
		#[transactional]
		pub fn transfer_multiasset_with_fee(
			origin: OriginFor<T>,
			asset: Box<VersionedMultiAsset>,
			fee: Box<VersionedMultiAsset>,
			dest: Box<VersionedMultiLocation>,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let asset: MultiAsset = (*asset).try_into().map_err(|()| Error::<T>::BadVersion)?;
			let fee: MultiAsset = (*fee).try_into().map_err(|()| Error::<T>::BadVersion)?;
			let dest: MultiLocation = (*dest).try_into().map_err(|()| Error::<T>::BadVersion)?;
			// Zero fee is an error
			if fungible_amount(&fee).is_zero() {
				return Err(Error::<T>::FeeCannotBeZero.into());
			}

			Self::do_transfer_multiasset_with_fee(who, asset, fee, dest, dest_weight)
		}

		/// Transfer several currencies specifying the item to be used as fee
		///
		/// `dest_weight` is the weight for XCM execution on the dest chain, and
		/// it would be charged from the transferred assets. If set below
		/// requirements, the execution may fail and assets wouldn't be
		/// received.
		///
		/// `fee_item` is index of the currencies tuple that we want to use for
		/// payment
		///
		/// It's a no-op if any error on local XCM execution or message sending.
		/// Note sending assets out per se doesn't guarantee they would be
		/// received. Receiving depends on if the XCM message could be delivered
		/// by the network, and if the receiving chain would handle
		/// messages correctly.
		#[pallet::weight(Pallet::<T>::weight_of_transfer_multicurrencies(currencies, fee_item, dest))]
		#[transactional]
		pub fn transfer_multicurrencies(
			origin: OriginFor<T>,
			currencies: Vec<(T::CurrencyId, T::Balance)>,
			fee_item: u32,
			dest: Box<VersionedMultiLocation>,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let dest: MultiLocation = (*dest).try_into().map_err(|()| Error::<T>::BadVersion)?;

			Self::do_transfer_multicurrencies(who, currencies, fee_item, dest, dest_weight)
		}

		/// Transfer several `MultiAsset` specifying the item to be used as fee
		///
		/// `dest_weight` is the weight for XCM execution on the dest chain, and
		/// it would be charged from the transferred assets. If set below
		/// requirements, the execution may fail and assets wouldn't be
		/// received.
		///
		/// `fee_item` is index of the MultiAssets that we want to use for
		/// payment
		///
		/// It's a no-op if any error on local XCM execution or message sending.
		/// Note sending assets out per se doesn't guarantee they would be
		/// received. Receiving depends on if the XCM message could be delivered
		/// by the network, and if the receiving chain would handle
		/// messages correctly.
		#[pallet::weight(Pallet::<T>::weight_of_transfer_multiassets(assets, fee_item, dest))]
		#[transactional]
		pub fn transfer_multiassets(
			origin: OriginFor<T>,
			assets: Box<VersionedMultiAssets>,
			fee_item: u32,
			dest: Box<VersionedMultiLocation>,
			dest_weight: Weight,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let assets: MultiAssets = (*assets).try_into().map_err(|()| Error::<T>::BadVersion)?;
			let dest: MultiLocation = (*dest).try_into().map_err(|()| Error::<T>::BadVersion)?;

			// We first grab the fee
			let fee: &MultiAsset = assets.get(fee_item as usize).ok_or(Error::<T>::AssetIndexNonExistent)?;

			Self::do_transfer_multiassets(who, assets.clone(), fee.clone(), dest, dest_weight, true)
		}
	}

	impl<T: Config> Pallet<T> {
		fn do_transfer(
			who: T::AccountId,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			let location: MultiLocation = T::CurrencyIdConvert::convert(currency_id.clone())
				.ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			let asset: MultiAsset = (location, amount.into()).into();
			Self::do_transfer_multiassets(
				who.clone(),
				vec![asset.clone()].into(),
				asset,
				dest.clone(),
				dest_weight,
				false,
			)?;

			Self::deposit_event(Event::<T>::Transferred {
				sender: who,
				currency_id,
				amount,
				dest,
			});
			Ok(())
		}

		fn do_transfer_with_fee(
			who: T::AccountId,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			fee: T::Balance,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			let location: MultiLocation = T::CurrencyIdConvert::convert(currency_id.clone())
				.ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			let asset = (location.clone(), amount.into()).into();
			let fee_asset: MultiAsset = (location, fee.into()).into();

			// Push contains saturated addition, so we should be able to use it safely
			let mut assets = MultiAssets::new();
			assets.push(asset);
			assets.push(fee_asset.clone());

			Self::do_transfer_multiassets(who.clone(), assets, fee_asset, dest.clone(), dest_weight, false)?;

			Self::deposit_event(Event::<T>::TransferredWithFee {
				sender: who,
				currency_id,
				fee,
				amount,
				dest,
			});
			Ok(())
		}

		fn do_transfer_multiasset(
			who: T::AccountId,
			asset: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			if !asset.is_fungible(None) {
				return Err(Error::<T>::NotFungible.into());
			}

			if fungible_amount(&asset).is_zero() {
				return Ok(());
			}

			Self::do_transfer_multiassets(
				who.clone(),
				vec![asset.clone()].into(),
				asset.clone(),
				dest.clone(),
				dest_weight,
				false,
			)?;

			Self::deposit_event(Event::<T>::TransferredMultiAsset {
				sender: who,
				asset,
				dest,
			});

			Ok(())
		}

		fn do_transfer_multiasset_with_fee(
			who: T::AccountId,
			asset: MultiAsset,
			fee: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			if !asset.is_fungible(None) || !fee.is_fungible(None) {
				return Err(Error::<T>::NotFungible.into());
			}

			if fungible_amount(&asset).is_zero() {
				return Ok(());
			}

			// Push contains saturated addition, so we should be able to use it safely
			let mut assets = MultiAssets::new();
			assets.push(asset.clone());
			assets.push(fee.clone());

			Self::do_transfer_multiassets(who.clone(), assets, fee.clone(), dest.clone(), dest_weight, false)?;

			Self::deposit_event(Event::<T>::TransferredMultiAssetWithFee {
				sender: who,
				asset,
				fee,
				dest,
			});

			Ok(())
		}

		fn do_transfer_multicurrencies(
			who: T::AccountId,
			currencies: Vec<(T::CurrencyId, T::Balance)>,
			fee_item: u32,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			let mut assets = MultiAssets::new();

			// Lets grab the fee amount and location first
			let (fee_currency_id, fee_amount) = currencies
				.get(fee_item as usize)
				.ok_or(Error::<T>::AssetIndexNonExistent)?;

			for (currency_id, amount) in &currencies {
				let location: MultiLocation = T::CurrencyIdConvert::convert(currency_id.clone())
					.ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;
				// Push contains saturated addition, so we should be able to use it safely
				assets.push((location, (*amount).into()).into())
			}

			// We construct the fee now, since getting it from assets wont work as assets
			// sorts it
			let fee_location: MultiLocation = T::CurrencyIdConvert::convert(fee_currency_id.clone())
				.ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;

			let fee: MultiAsset = (fee_location, (*fee_amount).into()).into();

			Self::do_transfer_multiassets(who.clone(), assets, fee, dest.clone(), dest_weight, false)?;

			Self::deposit_event(Event::<T>::TransferredMultiCurrencies {
				sender: who,
				currencies,
				dest,
			});
			Ok(())
		}

		fn do_transfer_multiassets(
			who: T::AccountId,
			assets: MultiAssets,
			fee: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
			deposit_event: bool,
		) -> DispatchResult {
			ensure!(
				assets.len() <= T::MaxAssetsForTransfer::get(),
				Error::<T>::TooManyAssetsBeingSent
			);

			// We check that all assets are valid and share the same reserve
			for i in 0..assets.len() {
				let asset = assets.get(i).ok_or(Error::<T>::AssetIndexNonExistent)?;
				if !asset.is_fungible(None) {
					return Err(Error::<T>::NotFungible.into());
				}
				if fungible_amount(asset).is_zero() {
					return Ok(());
				}
				ensure!(
					fee.reserve() == asset.reserve(),
					Error::<T>::DistinctReserveForAssetAndFee
				);
			}

			let (transfer_kind, dest, reserve, recipient) = Self::transfer_kind(&fee, &dest)?;
			let mut msg = match transfer_kind {
				SelfReserveAsset => {
					Self::transfer_self_reserve_asset(assets.clone(), fee, dest.clone(), recipient, dest_weight)?
				}
				ToReserve => Self::transfer_to_reserve(assets.clone(), fee, dest.clone(), recipient, dest_weight)?,
				ToNonReserve => {
					Self::transfer_to_non_reserve(assets.clone(), fee, reserve, dest.clone(), recipient, dest_weight)?
				}
			};

			let origin_location = T::AccountIdToMultiLocation::convert(who.clone());
			let weight = T::Weigher::weight(&mut msg).map_err(|()| Error::<T>::UnweighableMessage)?;
			T::XcmExecutor::execute_xcm_in_credit(origin_location, msg, weight, weight)
				.ensure_complete()
				.map_err(|error| {
					log::error!("Failed execute transfer message with {:?}", error);
					Error::<T>::XcmExecutionFailed
				})?;

			if deposit_event {
				Self::deposit_event(Event::<T>::TransferredMultiAssets {
					sender: who,
					assets,
					dest,
				});
			}

			Ok(())
		}

		fn transfer_self_reserve_asset(
			assets: MultiAssets,
			fee: MultiAsset,
			dest: MultiLocation,
			recipient: MultiLocation,
			dest_weight: Weight,
		) -> Result<Xcm<T::Call>, DispatchError> {
			Ok(Xcm(vec![
				WithdrawAsset(assets.clone()),
				DepositReserveAsset {
					assets: All.into(),
					max_assets: assets.len() as u32,
					dest: dest.clone(),
					xcm: Xcm(vec![
						Self::buy_execution(fee, &dest, dest_weight)?,
						Self::deposit_asset(recipient, assets.len() as u32),
					]),
				},
			]))
		}

		fn transfer_to_reserve(
			assets: MultiAssets,
			fee: MultiAsset,
			reserve: MultiLocation,
			recipient: MultiLocation,
			dest_weight: Weight,
		) -> Result<Xcm<T::Call>, DispatchError> {
			Ok(Xcm(vec![
				WithdrawAsset(assets.clone()),
				InitiateReserveWithdraw {
					assets: All.into(),
					reserve: reserve.clone(),
					xcm: Xcm(vec![
						Self::buy_execution(fee, &reserve, dest_weight)?,
						Self::deposit_asset(recipient, assets.len() as u32),
					]),
				},
			]))
		}

		fn transfer_to_non_reserve(
			assets: MultiAssets,
			fee: MultiAsset,
			reserve: MultiLocation,
			dest: MultiLocation,
			recipient: MultiLocation,
			dest_weight: Weight,
		) -> Result<Xcm<T::Call>, DispatchError> {
			let mut reanchored_dest = dest.clone();
			if reserve == MultiLocation::parent() {
				match dest {
					MultiLocation {
						parents,
						interior: X1(Parachain(id)),
					} if parents == 1 => {
						reanchored_dest = Parachain(id).into();
					}
					_ => {}
				}
			}

			Ok(Xcm(vec![
				WithdrawAsset(assets.clone()),
				InitiateReserveWithdraw {
					assets: All.into(),
					reserve: reserve.clone(),
					xcm: Xcm(vec![
						Self::buy_execution(half(&fee), &reserve, dest_weight)?,
						DepositReserveAsset {
							assets: All.into(),
							max_assets: assets.len() as u32,
							dest: reanchored_dest,
							xcm: Xcm(vec![
								Self::buy_execution(half(&fee), &dest, dest_weight)?,
								Self::deposit_asset(recipient, assets.len() as u32),
							]),
						},
					]),
				},
			]))
		}

		fn deposit_asset(recipient: MultiLocation, max_assets: u32) -> Instruction<()> {
			DepositAsset {
				assets: All.into(),
				max_assets,
				beneficiary: recipient,
			}
		}

		fn buy_execution(
			asset: MultiAsset,
			at: &MultiLocation,
			weight: Weight,
		) -> Result<Instruction<()>, DispatchError> {
			let ancestry = T::LocationInverter::ancestry();
			let fees = asset
				.reanchored(at, &ancestry)
				.map_err(|_| Error::<T>::CannotReanchor)?;
			Ok(BuyExecution {
				fees,
				weight_limit: WeightLimit::Limited(weight),
			})
		}

		/// Ensure has the `dest` has chain part and recipient part.
		fn ensure_valid_dest(dest: &MultiLocation) -> Result<(MultiLocation, MultiLocation), DispatchError> {
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
		) -> Result<(TransferKind, MultiLocation, MultiLocation, MultiLocation), DispatchError> {
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
		fn weight_of_transfer_multiasset(asset: &VersionedMultiAsset, dest: &VersionedMultiLocation) -> Weight {
			let asset = asset.clone().try_into();
			let dest = dest.clone().try_into();
			if let (Ok(asset), Ok(dest)) = (asset, dest) {
				if let Ok((transfer_kind, dest, _, reserve)) = Self::transfer_kind(&asset, &dest) {
					let mut msg = match transfer_kind {
						SelfReserveAsset => Xcm(vec![
							WithdrawAsset(MultiAssets::from(asset.clone())),
							DepositReserveAsset {
								assets: All.into(),
								max_assets: 1,
								dest,
								xcm: Xcm(vec![]),
							},
						]),
						ToReserve | ToNonReserve => Xcm(vec![
							WithdrawAsset(MultiAssets::from(asset.clone())),
							InitiateReserveWithdraw {
								assets: All.into(),
								// `dest` is always (equal to) `reserve` in both cases
								reserve,
								xcm: Xcm(vec![]),
							},
						]),
					};
					return T::Weigher::weight(&mut msg)
						.map_or(Weight::max_value(), |w| T::BaseXcmWeight::get().saturating_add(w));
				}
			}
			0
		}

		/// Returns weight of `transfer` call.
		fn weight_of_transfer(currency_id: T::CurrencyId, amount: T::Balance, dest: &VersionedMultiLocation) -> Weight {
			if let Some(location) = T::CurrencyIdConvert::convert(currency_id) {
				let asset = (location, amount.into()).into();
				Self::weight_of_transfer_multiasset(&asset, dest)
			} else {
				0
			}
		}

		/// Returns weight of `transfer` call.
		fn weight_of_transfer_multicurrencies(
			currencies: &[(T::CurrencyId, T::Balance)],
			fee_item: &u32,
			dest: &VersionedMultiLocation,
		) -> Weight {
			let mut assets: Vec<MultiAsset> = Vec::new();
			for (currency_id, amount) in currencies {
				if let Some(location) = T::CurrencyIdConvert::convert(currency_id.clone()) {
					let asset: MultiAsset = (location.clone(), (*amount).into()).into();
					assets.push(asset);
				} else {
					return 0;
				}
			}

			Self::weight_of_transfer_multiassets(&VersionedMultiAssets::from(MultiAssets::from(assets)), fee_item, dest)
		}

		/// Returns weight of `transfer_multiassets` call.
		fn weight_of_transfer_multiassets(
			assets: &VersionedMultiAssets,
			fee_item: &u32,
			dest: &VersionedMultiLocation,
		) -> Weight {
			let assets: Result<MultiAssets, ()> = assets.clone().try_into();

			let dest = dest.clone().try_into();
			if let (Ok(assets), Ok(dest)) = (assets, dest) {
				if let Some(fee) = assets.get(*fee_item as usize) {
					if let Ok((transfer_kind, dest, _, reserve)) = Self::transfer_kind(fee, &dest) {
						let mut msg = match transfer_kind {
							SelfReserveAsset => Xcm(vec![
								WithdrawAsset(assets.clone()),
								DepositReserveAsset {
									assets: All.into(),
									max_assets: assets.len() as u32,
									dest,
									xcm: Xcm(vec![]),
								},
							]),
							ToReserve | ToNonReserve => Xcm(vec![
								WithdrawAsset(assets),
								InitiateReserveWithdraw {
									assets: All.into(),
									// `dest` is always (equal to) `reserve` in both cases
									reserve,
									xcm: Xcm(vec![]),
								},
							]),
						};
						return T::Weigher::weight(&mut msg)
							.map_or(Weight::max_value(), |w| T::BaseXcmWeight::get().saturating_add(w));
					}
				}
			}
			0
		}
	}

	impl<T: Config> XcmTransfer<T::AccountId, T::Balance, T::CurrencyId> for Pallet<T> {
		#[require_transactional]
		fn transfer(
			who: T::AccountId,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			Self::do_transfer(who, currency_id, amount, dest, dest_weight)
		}

		#[require_transactional]
		fn transfer_multi_asset(
			who: T::AccountId,
			asset: MultiAsset,
			dest: MultiLocation,
			dest_weight: Weight,
		) -> DispatchResult {
			Self::do_transfer_multiasset(who, asset, dest, dest_weight)
		}
	}
}

/// Returns amount if `asset` is fungible, or zero.
fn fungible_amount(asset: &MultiAsset) -> u128 {
	if let Fungible(amount) = &asset.fun {
		*amount
	} else {
		Zero::zero()
	}
}

fn half(asset: &MultiAsset) -> MultiAsset {
	let half_amount = fungible_amount(asset)
		.checked_div(2)
		.expect("div 2 can't overflow; qed");
	MultiAsset {
		fun: Fungible(half_amount),
		id: asset.id.clone(),
	}
}
