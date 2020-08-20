#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, traits::Get, Parameter};
use frame_system::ensure_signed;
use orml_traits::MultiCurrency;
use sp_runtime::traits::{AtLeast32Bit, CheckedSub, Convert, MaybeSerializeDeserialize, Member};
use sp_std::{
	convert::{TryFrom, TryInto},
	prelude::*,
};

use cumulus_primitives::{
	relay_chain::{Balance as RelayChainBalance, DownwardMessage},
	xcmp::{XCMPMessageHandler, XCMPMessageSender},
	DownwardMessageHandler, ParaId, UpwardMessageOrigin, UpwardMessageSender,
};

use cumulus_upward_message::BalancesMessage;

#[derive(Encode, Decode)]
pub enum XCMPMessage<AccountId, Balance> {
	/// Transfer tokens to the given account from the Parachain account.
	Transfer(Vec<u8>, Balance, AccountId),
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The balance type
	type Balance: Parameter + Member + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The currency ID type
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + Into<Vec<u8>> + TryFrom<Vec<u8>>;

	type RelayChainCurrencyId: Get<Self::CurrencyId>;

	type FromRelayChainBalance: Convert<RelayChainBalance, Self::Balance>;
	type ToRelayChainBalance: Convert<Self::Balance, RelayChainBalance>;

	type Currency: MultiCurrency<Self::AccountId, CurrencyId = Self::CurrencyId, Balance = Self::Balance>;

	type XCMPMessageSender: XCMPMessageSender<XCMPMessage<Self::AccountId, Self::Balance>>;

	/// The sender of upward messages.
	type UpwardMessageSender: UpwardMessageSender<Self::UpwardMessage>;

	/// The upward message type used by the Parachain runtime.
	type UpwardMessage: codec::Codec + BalancesMessage<Self::AccountId, RelayChainBalance>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Tokens {
		UnknownBalances: double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) Vec<u8> => T::Balance;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::Balance
	{
		/// Transfer some assets to another parachain. [para_id, asset_id, dest, amount]
		TransferToParachain(ParaId, Vec<u8>, AccountId, Balance),
		/// Received soem assets from another parachain. [para_id, asset_id, dest, amount]
		ReceivedTransferFromParachain(ParaId, Vec<u8>, AccountId, Balance),
		/// Transfer relay chain token to relay chain. [dest, amount]
		TransferToRelayChain(AccountId, Balance),
		/// Received relay chain token from relay chain. [dest, amount]
		ReceivedTransferFromRelayChain(AccountId, Balance),
	}
);

decl_error! {
	/// Error for token module.
	pub enum Error for Module<T: Trait> {
		TooLow
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 10]
		fn transfer_to_parachain(origin, currency_id: T::CurrencyId, para_id: ParaId, dest: T::AccountId, amount: T::Balance) {
			let who = ensure_signed(origin)?;

			T::Currency::withdraw(currency_id, &who, amount)?;

			let asset_id: Vec<u8> = currency_id.into();

			T::XCMPMessageSender::send_xcmp_message(para_id, &XCMPMessage::Transfer(asset_id.clone(), amount, dest.clone())).expect("should not fail");

			Self::deposit_event(Event::<T>::TransferToParachain(para_id, asset_id, dest, amount));
		}

		#[weight = 10]
		fn transfer_to_relay_chain(origin, dest: T::AccountId, amount: T::Balance) {
			let who = ensure_signed(origin)?;

			T::Currency::withdraw(T::RelayChainCurrencyId::get(), &who, amount)?;

			let msg = T::UpwardMessage::transfer(dest.clone(), T::ToRelayChainBalance::convert(amount));
			T::UpwardMessageSender::send_upward_message(&msg, UpwardMessageOrigin::Signed).expect("should not fail");

			Self::deposit_event(Event::<T>::TransferToRelayChain(dest, amount));
		}

		#[weight = 10]
		fn transfer_unknown_asset_to_parachain(origin, id: Vec<u8>, para_id: ParaId, dest: T::AccountId, amount: T::Balance) {
			let who = ensure_signed(origin)?;

			UnknownBalances::<T>::try_mutate(who, &id, |total| total.checked_sub(&amount).ok_or(Error::<T>::TooLow))?;

			T::XCMPMessageSender::send_xcmp_message(para_id, &XCMPMessage::Transfer(id.clone(), amount, dest.clone())).expect("should not fail");

			Self::deposit_event(Event::<T>::TransferToParachain(para_id, id, dest, amount));
		}
	}
}

/// This is a hack to convert from one generic type to another where we are sure
/// that both are the same type/use the same encoding.
fn convert_hack<O: Decode>(input: &impl Encode) -> O {
	input.using_encoded(|e| Decode::decode(&mut &e[..]).expect("Must be compatible; qed"))
}

impl<T: Trait> DownwardMessageHandler for Module<T> {
	fn handle_downward_message(msg: &DownwardMessage) {
		match msg {
			DownwardMessage::TransferInto(dest, amount, _) => {
				let dest: T::AccountId = convert_hack(dest);
				let amount = T::FromRelayChainBalance::convert(*amount);

				let _ = T::Currency::deposit(T::RelayChainCurrencyId::get(), &dest, amount.clone());

				Self::deposit_event(Event::<T>::ReceivedTransferFromRelayChain(dest.clone(), amount));
			}
			_ => {}
		}
	}
}

impl<T: Trait> XCMPMessageHandler<XCMPMessage<T::AccountId, T::Balance>> for Module<T> {
	fn handle_xcmp_message(src: ParaId, msg: &XCMPMessage<T::AccountId, T::Balance>) {
		match msg {
			XCMPMessage::Transfer(asset_id, amount, dest) => {
				match asset_id.clone().try_into() {
					Ok(currency_id) => {
						T::Currency::deposit(currency_id, dest, *amount).expect("should not fail");
					}
					_ => UnknownBalances::<T>::mutate(dest, asset_id, |total| *total += *amount),
				}

				Self::deposit_event(Event::<T>::ReceivedTransferFromParachain(
					src,
					asset_id.clone(),
					dest.clone(),
					*amount,
				));
			}
		}
	}
}
