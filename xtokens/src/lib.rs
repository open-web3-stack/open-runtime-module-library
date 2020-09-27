#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_event, decl_module, decl_storage, traits::Get, Parameter};
use frame_system::ensure_signed;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Member},
	DispatchResult, RuntimeDebug,
};
use sp_std::prelude::*;

use xcm::v0::{Xcm, Junction, MultiAsset, MultiLocation, NetworkId, Order};
use cumulus_primitives::{ParaId, relay_chain::Balance as RelayChainBalance};
use orml_utilities::with_transaction_result;
use orml_xmulticurrency::XcmHandler;

mod mock;
mod tests;

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
	/// The owner chain of the currency. For instance, the owner chain of DOT is
	/// Polkadot.
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

#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug)]
pub enum XCMPTokenMessage<AccountId, Balance> {
	/// Token transfer. [x_currency_id, para_id, dest, amount]
	Transfer(XCurrencyId, ParaId, AccountId, Balance),
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Into<u128>;

	/// Convertor `RelayChainBalance` to `Balance`.
	type FromRelayChainBalance: Convert<RelayChainBalance, Self::Balance>;

	/// Convertor `Balance` to `RelayChainBalance`.
	type ToRelayChainBalance: Convert<Self::Balance, RelayChainBalance>;

	type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

	type RelayChainCurrencyKey: Get<Vec<u8>>;

	type RelayChainNetworkId: Get<NetworkId>;

	/// Parachain ID.
	type ParaId: Get<ParaId>;

	type XcmHandler: XcmHandler<Origin=Self::Origin, Xcm=Xcm>;
}

decl_storage! {
	trait Store for Module<T: Trait> as XTokens {}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::Balance,
		XCurrencyId = XCurrencyId,
	{
		/// Transferred to relay chain. [src, dest, amount]
		TransferredToRelayChain(AccountId, AccountId, Balance),

		/// Received transfer from relay chain. [dest, amount]
		ReceivedTransferFromRelayChain(AccountId, Balance),

		/// Transferred to parachain. [x_currency_id, src, para_id, dest, amount]
		TransferredToParachain(XCurrencyId, AccountId, ParaId, AccountId, Balance),

		/// Received transfer from parachain. [x_currency_id, para_id, dest, amount]
		ReceivedTransferFromParachain(XCurrencyId, ParaId, AccountId, Balance),
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
				Self::do_transfer_to_relay_chain(origin, &dest, amount)?;
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
			amount: T::Balance,
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;

				if para_id == T::ParaId::get() {
					return Ok(());
				}

				Self::do_transfer_to_parachain(x_currency_id.clone(), &who, para_id, &dest, amount)?;
				Self::deposit_event(Event::<T>::TransferredToParachain(x_currency_id, who, para_id, dest, amount));

				Ok(())
			})?;
		}
	}
}

impl <T: Trait> Module<T> {
	fn do_transfer_to_relay_chain(origin: T::Origin, dest: &T::AccountId, amount: T::Balance) -> DispatchResult {
		let relay_chain_token = MultiAsset::ConcreteFungible {
			id: MultiLocation::X1(Junction::Parent),
			amount: amount.into(),
		};
		// deposit to `dest` on relay chain
		let deposit_asset = Order::DepositReserveAsset {
			assets: vec![relay_chain_token.clone()],
			dest: MultiLocation::X1(Junction::AccountId32 {
				network: T::RelayChainNetworkId::get(),
				id: T::AccountId32Convert::convert(dest.clone()),
			}),
			effects: vec![],
		};
		// withdraw from reserve account on relay chain
		let initiate_reserved_withdraw = Order::InitiateReserveWithdraw {
			assets: vec![relay_chain_token],
			reserve: MultiLocation::X1(Junction::Parent),
			effects: vec![deposit_asset],
		};

		let local_relay_chain_token = MultiAsset::ConcreteFungible {
			id: MultiLocation::X1(Junction::GeneralKey(T::RelayChainCurrencyKey::get())),
			amount: amount.into(),
		};
		// withdraw from `origin` on home chain
		let xcm = Xcm::WithdrawAsset {
			assets: vec![local_relay_chain_token],
			effects: vec![initiate_reserved_withdraw],
		};

		T::XcmHandler::execute(origin, xcm)
	}

	fn do_transfer_to_parachain(
		x_currency_id: XCurrencyId,
		src: &T::AccountId,
		para_id: ParaId,
		dest: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		Ok(())
	}
}
