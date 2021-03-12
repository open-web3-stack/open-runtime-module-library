#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::from_over_into)]
#![allow(clippy::unused_unit)]
#![allow(clippy::large_enum_variant)]

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use codec::{Decode, Encode};
	use frame_support::{pallet_prelude::*, traits::Get, transactional, Parameter};
	use frame_system::{ensure_signed, pallet_prelude::*};
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Member},
		RuntimeDebug,
	};
	use sp_std::prelude::*;

	use cumulus_primitives_core::{relay_chain::Balance as RelayChainBalance, ParaId};
	use xcm::v0::{Error as XcmError, ExecuteXcm, Junction, MultiAsset, MultiLocation, NetworkId, Order, Xcm};
	use xcm_executor::traits::LocationConversion;

	#[derive(Encode, Decode, Eq, PartialEq, Clone, Copy, RuntimeDebug)]
	/// Identity of chain.
	pub enum ChainId {
		/// The relay chain.
		RelayChain,
		/// A parachain.
		ParaChain(ParaId),
	}

	#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug)]
	/// Identity of cross chain currency.
	pub struct XCurrencyId {
		/// The reserve chain of the currency. For instance, the reserve chain
		/// of DOT is Polkadot.
		pub chain_id: ChainId,
		/// The identity of the currency.
		pub currency_id: Vec<u8>,
	}

	#[cfg(test)]
	impl XCurrencyId {
		pub fn new(chain_id: ChainId, currency_id: Vec<u8>) -> Self {
			XCurrencyId { chain_id, currency_id }
		}
	}

	impl Into<MultiLocation> for XCurrencyId {
		fn into(self) -> MultiLocation {
			MultiLocation::X1(Junction::GeneralKey(self.currency_id))
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Into<u128>;

		/// Convertor `Balance` to `RelayChainBalance`.
		type ToRelayChainBalance: Convert<Self::Balance, RelayChainBalance>;

		type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

		type RelayChainNetworkId: Get<NetworkId>;

		/// Parachain ID.
		type ParaId: Get<ParaId>;

		type AccountIdConverter: LocationConversion<Self::AccountId>;

		type XcmExecutor: ExecuteXcm;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Transferred to relay chain. \[src, dest, amount\]
		TransferredToRelayChain(T::AccountId, T::AccountId, T::Balance),

		/// Transfer to relay chain failed. \[src, dest, amount, error\]
		TransferToRelayChainFailed(T::AccountId, T::AccountId, T::Balance, XcmError),

		/// Transferred to parachain. \[x_currency_id, src, para_id, dest,
		/// dest_network, amount\]
		TransferredToParachain(XCurrencyId, T::AccountId, ParaId, MultiLocation, T::Balance),

		/// Transfer to parachain failed. \[x_currency_id, src, para_id, dest,
		/// dest_network, amount, error\]
		TransferToParachainFailed(XCurrencyId, T::AccountId, ParaId, MultiLocation, T::Balance, XcmError),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Bad location.
		BadLocation,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer relay chain tokens to relay chain.
		#[pallet::weight(10)]
		#[transactional]
		pub fn transfer_to_relay_chain(
			origin: OriginFor<T>,
			dest: T::AccountId,
			amount: T::Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let xcm = Xcm::WithdrawAsset {
				assets: vec![MultiAsset::ConcreteFungible {
					id: MultiLocation::X1(Junction::Parent),
					amount: T::ToRelayChainBalance::convert(amount),
				}],
				effects: vec![Order::InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve: MultiLocation::X1(Junction::Parent),
					effects: vec![Order::DepositAsset {
						assets: vec![MultiAsset::All],
						dest: MultiLocation::X1(Junction::AccountId32 {
							network: T::RelayChainNetworkId::get(),
							id: T::AccountId32Convert::convert(dest.clone()),
						}),
					}],
				}],
			};

			let xcm_origin =
				T::AccountIdConverter::try_into_location(who.clone()).map_err(|_| Error::<T>::BadLocation)?;
			// TODO: revert state on xcm execution failure.
			match T::XcmExecutor::execute_xcm(xcm_origin, xcm) {
				Ok(_) => Self::deposit_event(Event::<T>::TransferredToRelayChain(who, dest, amount)),
				Err(err) => Self::deposit_event(Event::<T>::TransferToRelayChainFailed(who, dest, amount, err)),
			}

			Ok(().into())
		}

		/// Transfer tokens to parachain.
		#[pallet::weight(10)]
		#[transactional]
		pub fn transfer_to_parachain(
			origin: OriginFor<T>,
			x_currency_id: XCurrencyId,
			para_id: ParaId,
			dest: MultiLocation,
			amount: T::Balance,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			if para_id == T::ParaId::get() {
				return Ok(().into());
			}

			let xcm = match x_currency_id.chain_id {
				ChainId::RelayChain => Self::transfer_relay_chain_tokens_to_parachain(para_id, dest.clone(), amount),
				ChainId::ParaChain(reserve_chain) => {
					if T::ParaId::get() == reserve_chain {
						Self::transfer_owned_tokens_to_parachain(x_currency_id.clone(), para_id, dest.clone(), amount)
					} else {
						Self::transfer_non_owned_tokens_to_parachain(
							reserve_chain,
							x_currency_id.clone(),
							para_id,
							dest.clone(),
							amount,
						)
					}
				}
			};

			let xcm_origin =
				T::AccountIdConverter::try_into_location(who.clone()).map_err(|_| Error::<T>::BadLocation)?;
			// TODO: revert state on xcm execution failure.
			match T::XcmExecutor::execute_xcm(xcm_origin, xcm) {
				Ok(_) => Self::deposit_event(Event::<T>::TransferredToParachain(
					x_currency_id,
					who,
					para_id,
					dest,
					amount,
				)),
				Err(err) => Self::deposit_event(Event::<T>::TransferToParachainFailed(
					x_currency_id,
					who,
					para_id,
					dest,
					amount,
					err,
				)),
			}

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		fn transfer_relay_chain_tokens_to_parachain(para_id: ParaId, dest: MultiLocation, amount: T::Balance) -> Xcm {
			Xcm::WithdrawAsset {
				assets: vec![MultiAsset::ConcreteFungible {
					id: MultiLocation::X1(Junction::Parent),
					amount: T::ToRelayChainBalance::convert(amount),
				}],
				effects: vec![Order::InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve: MultiLocation::X1(Junction::Parent),
					effects: vec![Order::DepositReserveAsset {
						assets: vec![MultiAsset::All],
						// Reserve asset deposit dest is children parachain(of parent).
						dest: MultiLocation::X1(Junction::Parachain { id: para_id.into() }),
						effects: vec![Order::DepositAsset {
							assets: vec![MultiAsset::All],
							dest,
						}],
					}],
				}],
			}
		}

		/// Transfer parachain tokens "owned" by self parachain to another
		/// parachain.
		///
		/// NOTE - `para_id` must not be self parachain.
		fn transfer_owned_tokens_to_parachain(
			x_currency_id: XCurrencyId,
			para_id: ParaId,
			dest: MultiLocation,
			amount: T::Balance,
		) -> Xcm {
			Xcm::WithdrawAsset {
				assets: vec![MultiAsset::ConcreteFungible {
					id: x_currency_id.into(),
					amount: amount.into(),
				}],
				effects: vec![Order::DepositReserveAsset {
					assets: vec![MultiAsset::All],
					dest: MultiLocation::X2(Junction::Parent, Junction::Parachain { id: para_id.into() }),
					effects: vec![Order::DepositAsset {
						assets: vec![MultiAsset::All],
						dest,
					}],
				}],
			}
		}

		/// Transfer parachain tokens not "owned" by self chain to another
		/// parachain.
		fn transfer_non_owned_tokens_to_parachain(
			reserve_chain: ParaId,
			x_currency_id: XCurrencyId,
			para_id: ParaId,
			dest: MultiLocation,
			amount: T::Balance,
		) -> Xcm {
			let deposit_to_dest = Order::DepositAsset {
				assets: vec![MultiAsset::All],
				dest,
			};
			// If transfer to reserve chain, deposit to `dest` on reserve chain,
			// else deposit reserve asset.
			let reserve_chain_order = if para_id == reserve_chain {
				deposit_to_dest
			} else {
				Order::DepositReserveAsset {
					assets: vec![MultiAsset::All],
					dest: MultiLocation::X2(Junction::Parent, Junction::Parachain { id: para_id.into() }),
					effects: vec![deposit_to_dest],
				}
			};

			Xcm::WithdrawAsset {
				assets: vec![MultiAsset::ConcreteFungible {
					id: x_currency_id.into(),
					amount: amount.into(),
				}],
				effects: vec![Order::InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve: MultiLocation::X2(
						Junction::Parent,
						Junction::Parachain {
							id: reserve_chain.into(),
						},
					),
					effects: vec![reserve_chain_order],
				}],
			}
		}
	}
}
