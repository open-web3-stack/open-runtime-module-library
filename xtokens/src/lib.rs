#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_event, decl_module, decl_storage, traits::Get, Parameter};
use frame_system::ensure_signed;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Member},
	RuntimeDebug,
};
use sp_std::prelude::*;

use cumulus_primitives::{relay_chain::Balance as RelayChainBalance, ParaId};
use orml_utilities::with_transaction_result;
use orml_xcm_support::XcmHandler;
use xcm::v0::{Junction, MultiAsset, MultiLocation, NetworkId, Order, Xcm};

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
	/// The reserve chain of the currency. For instance, the reserve chain of
	/// DOT is Polkadot.
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

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Into<u128>;

	/// Convertor `Balance` to `RelayChainBalance`.
	type ToRelayChainBalance: Convert<Self::Balance, RelayChainBalance>;

	type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

	type RelayChainNetworkId: Get<NetworkId>;

	/// Parachain ID.
	type ParaId: Get<ParaId>;

	type XcmHandler: XcmHandler<Origin = Self::Origin, Xcm = Xcm>;
}

decl_storage! {
	trait Store for Module<T: Trait> as XTokens {}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::Balance,
		XCurrencyId = XCurrencyId,
		NetworkId = NetworkId,
	{
		/// Transferred to relay chain. [src, dest, amount]
		TransferredToRelayChain(AccountId, AccountId, Balance),

		/// Transferred to parachain. [x_currency_id, src, para_id, dest, dest_network, amount]
		TransferredToParachain(XCurrencyId, AccountId, ParaId, AccountId, NetworkId, Balance),
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		/// Transfer relay chain tokens to relay chain.
		#[weight = 10]
		pub fn transfer_to_relay_chain(origin, dest: T::AccountId, amount: T::Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin.clone())?;

				let xcm = Self::do_transfer_to_relay_chain(&dest, amount);
				T::XcmHandler::execute(origin, xcm)?;

				Self::deposit_event(Event::<T>::TransferredToRelayChain(who, dest, amount));

				Ok(())
			})?;
		}

		/// Transfer tokens to parachain.
		#[weight = 10]
		pub fn transfer_to_parachain(
			origin,
			x_currency_id: XCurrencyId,
			para_id: ParaId,
			dest: T::AccountId,
			dest_network: NetworkId,
			amount: T::Balance,
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin.clone())?;

				if para_id == T::ParaId::get() {
					return Ok(());
				}

				let xcm = Self::do_transfer_to_parachain(
					x_currency_id.clone(),
					para_id,
					&dest,
					dest_network.clone(),
					amount,
				);
				T::XcmHandler::execute(origin, xcm)?;

				Self::deposit_event(
					Event::<T>::TransferredToParachain(x_currency_id, who, para_id, dest, dest_network, amount),
				);

				Ok(())
			})?;
		}
	}
}

impl<T: Trait> Module<T> {
	fn do_transfer_to_relay_chain(dest: &T::AccountId, amount: T::Balance) -> Xcm {
		Xcm::WithdrawAsset {
			assets: vec![MultiAsset::ConcreteFungible {
				id: MultiLocation::X1(Junction::Parent),
				amount: amount.into(),
			}],
			effects: vec![Order::InitiateReserveWithdraw {
				assets: vec![MultiAsset::ConcreteFungible {
					id: MultiLocation::X1(Junction::Parent),
					amount: T::ToRelayChainBalance::convert(amount).into(),
				}],
				reserve: MultiLocation::X1(Junction::Parent),
				effects: vec![Order::DepositAsset {
					assets: vec![MultiAsset::All],
					dest: MultiLocation::X1(Junction::AccountId32 {
						network: T::RelayChainNetworkId::get(),
						id: T::AccountId32Convert::convert(dest.clone()),
					}),
				}],
			}],
		}
	}

	fn do_transfer_to_parachain(
		x_currency_id: XCurrencyId,
		para_id: ParaId,
		dest: &T::AccountId,
		dest_network: NetworkId,
		amount: T::Balance,
	) -> Xcm {
		match x_currency_id.chain_id {
			ChainId::RelayChain => Self::transfer_relay_chain_tokens_to_parachain(para_id, dest, dest_network, amount),
			ChainId::ParaChain(reserve_chain) => {
				if T::ParaId::get() == reserve_chain {
					Self::transfer_owned_tokens_to_parachain(x_currency_id, para_id, dest, dest_network, amount)
				} else {
					Self::transfer_non_owned_tokens_to_parachain(
						reserve_chain,
						x_currency_id,
						para_id,
						dest,
						dest_network,
						amount,
					)
				}
			}
		}
	}

	fn transfer_relay_chain_tokens_to_parachain(
		para_id: ParaId,
		dest: &T::AccountId,
		dest_network: NetworkId,
		amount: T::Balance,
	) -> Xcm {
		let asset_on_parachain = MultiAsset::ConcreteFungible {
			id: MultiLocation::X1(Junction::Parent),
			amount: amount.into(),
		};
		Xcm::WithdrawAsset {
			assets: vec![asset_on_parachain.clone()],
			effects: vec![Order::InitiateReserveWithdraw {
				assets: vec![MultiAsset::ConcreteFungible {
					id: MultiLocation::X1(Junction::Parent),
					amount: T::ToRelayChainBalance::convert(amount).into(),
				}],
				reserve: MultiLocation::X1(Junction::Parent),
				effects: vec![Order::DepositReserveAsset {
					assets: vec![MultiAsset::All],
					dest: MultiLocation::X1(Junction::Parachain { id: para_id.into() }),
					effects: vec![Order::DepositAsset {
						assets: vec![asset_on_parachain],
						dest: MultiLocation::X1(Junction::AccountId32 {
							network: dest_network,
							id: T::AccountId32Convert::convert(dest.clone()),
						}),
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
		dest: &T::AccountId,
		dest_network: NetworkId,
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
					dest: MultiLocation::X1(Junction::AccountId32 {
						network: dest_network,
						id: T::AccountId32Convert::convert(dest.clone()),
					}),
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
		dest: &T::AccountId,
		dest_network: NetworkId,
		amount: T::Balance,
	) -> Xcm {
		let deposit_to_dest = Order::DepositAsset {
			assets: vec![MultiAsset::All],
			dest: MultiLocation::X1(Junction::AccountId32 {
				network: dest_network,
				id: T::AccountId32Convert::convert(dest.clone()),
			}),
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
