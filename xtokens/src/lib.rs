#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, traits::Get, Parameter};
use frame_system::ensure_signed;
use orml_traits::MultiCurrency;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, CheckedSub, Convert, MaybeSerializeDeserialize, Member, Saturating},
	DispatchResult, RuntimeDebug,
};
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
use polkadot_parachain::primitives::AccountIdConversion;

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
	chain_id: ChainId,
	/// The identity of the currency.
	currency_id: Vec<u8>,
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum XCMPTokenMessage<AccountId, Balance> {
	/// Token transfer. [x_currency_id, para_id, dest, amount]
	Transfer(XCurrencyId, ParaId, AccountId, Balance),
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The balance type.
	type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize;

	/// Convertor to convert between `T::Balance` and relay chain balance.
	type BalanceConvertor: Convert<RelayChainBalance, Self::Balance> + Convert<Self::Balance, RelayChainBalance>;

	/// The currency ID type
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + Into<Vec<u8>> + TryFrom<Vec<u8>>;

	/// Currency Id of relay chain.
	type RelayChainCurrencyId: Get<Self::CurrencyId>;

	/// The `MultiCurrency` impl for tokens.
	type Currency: MultiCurrency<Self::AccountId, CurrencyId = Self::CurrencyId, Balance = Self::Balance>;

	/// Parachain ID.
	type ParaId: Get<ParaId>;

	/// The sender of XCMP message.
	type XCMPMessageSender: XCMPMessageSender<XCMPTokenMessage<Self::AccountId, Self::Balance>>;

	/// The sender of upward message(to relay chain).
	type UpwardMessageSender: UpwardMessageSender<Self::UpwardMessage>;

	/// The upward message type used by parachain runtime.
	type UpwardMessage: codec::Codec + BalancesMessage<Self::AccountId, RelayChainBalance>;
}

decl_storage! {
	trait Store for Module<T: Trait> as XTokens {
		/// Balances of currencies not known to self parachain.
		UnknownBalances: double_map hasher(blake2_128_concat) T::AccountId, hasher(blake2_128_concat) Vec<u8> => T::Balance;
	}
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

decl_error! {
	/// Error for xtokens module.
	pub enum Error for Module<T: Trait> {
		/// Insufficient balance to transfer.
		InsufficientBalance,
		/// Invalid currency ID.
		InvalidCurrencyId,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Transfer relay chain tokens to relay chain.
		#[weight = 10]
		pub fn transfer_to_relay_chain(origin, dest: T::AccountId, amount: T::Balance) {
			let who = ensure_signed(origin)?;
			Self::do_transfer_to_relay_chain(&who, &dest, amount)?;
			Self::deposit_event(Event::<T>::TransferredToRelayChain(who, dest, amount));
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
			let who = ensure_signed(origin)?;
			Self::do_transfer_to_parachain(x_currency_id.clone(), &who, para_id, &dest, amount)?;
			Self::deposit_event(Event::<T>::TransferredToParachain(x_currency_id, who, para_id, dest, amount));
		}
	}
}

impl<T: Trait> Module<T> {
	fn do_transfer_to_relay_chain(who: &T::AccountId, dest: &T::AccountId, amount: T::Balance) -> DispatchResult {
		T::Currency::withdraw(T::RelayChainCurrencyId::get(), who, amount)?;
		let msg = T::UpwardMessage::transfer(dest.clone(), T::BalanceConvertor::convert(amount));
		T::UpwardMessageSender::send_upward_message(&msg, UpwardMessageOrigin::Signed).expect("Should not fail; qed");
		Ok(())
	}

	fn do_transfer_to_parachain(
		x_currency_id: XCurrencyId,
		src: &T::AccountId,
		para_id: ParaId,
		dest: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		match x_currency_id.chain_id {
			ChainId::RelayChain => {
				Self::transfer_relay_chain_tokens_to_parachain(x_currency_id, src, para_id, dest, amount)
			}
			ChainId::ParaChain(token_owner) => {
				if T::ParaId::get() == token_owner {
					Self::transfer_owned_tokens_to_parachain(x_currency_id, src, para_id, dest, amount)
				} else {
					Self::transfer_non_owned_tokens_to_parachain(token_owner, x_currency_id, src, para_id, dest, amount)
				}
			}
		}
	}

	/// Transfer relay chain tokens to another parachain.
	///
	/// 1. Withdraw `src` balance.
	/// 2. Transfer in relay chain: from self parachain account to `para_id`
	/// account. 3. Notify `para_id` the transfer.
	fn transfer_relay_chain_tokens_to_parachain(
		x_currency_id: XCurrencyId,
		src: &T::AccountId,
		para_id: ParaId,
		dest: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		let para_account = para_id.into_account();

		T::Currency::withdraw(T::RelayChainCurrencyId::get(), src, amount)?;

		let msg = T::UpwardMessage::transfer(para_account, T::BalanceConvertor::convert(amount));
		T::UpwardMessageSender::send_upward_message(&msg, UpwardMessageOrigin::Signed).expect("Should not fail; qed");

		T::XCMPMessageSender::send_xcmp_message(
			para_id,
			&XCMPTokenMessage::Transfer(x_currency_id, para_id, dest.clone(), amount),
		)
		.expect("Should not fail; qed");

		Ok(())
	}

	/// Transfer parachain tokens "owned" by self parachain to another
	/// parachain.
	///
	/// 1. Transfer from `src` to `para_id` account.
	/// 2. Notify `para_id` the transfer.
	fn transfer_owned_tokens_to_parachain(
		x_currency_id: XCurrencyId,
		src: &T::AccountId,
		para_id: ParaId,
		dest: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		let para_account = para_id.into_account();
		let currency_id: T::CurrencyId = x_currency_id
			.currency_id
			.clone()
			.try_into()
			.map_err(|_| Error::<T>::InvalidCurrencyId)?;
		T::Currency::transfer(currency_id, src, &para_account, amount)?;

		T::XCMPMessageSender::send_xcmp_message(
			para_id,
			&XCMPTokenMessage::Transfer(x_currency_id, para_id, dest.clone(), amount),
		)
		.expect("Should not fail; qed");

		Ok(())
	}

	/// Transfer parachain tokens not "owned" by self chain to another
	/// parachain.
	///
	/// 1. Withdraw from `src`.
	/// 2. Notify token owner parachain the transfer. (Token owner chain would
	/// further notify `para_id`)
	fn transfer_non_owned_tokens_to_parachain(
		token_owner: ParaId,
		x_currency_id: XCurrencyId,
		src: &T::AccountId,
		para_id: ParaId,
		dest: &T::AccountId,
		amount: T::Balance,
	) -> DispatchResult {
		if let Ok(currency_id) = x_currency_id.currency_id.clone().try_into() {
			// Known currency, withdraw from src.
			T::Currency::withdraw(currency_id, src, amount)?;
		} else {
			// Unknown currency, update balance.
			UnknownBalances::<T>::try_mutate(src, &x_currency_id.currency_id, |total| {
				total.checked_sub(&amount).ok_or(Error::<T>::InsufficientBalance)
			})?;
		}

		T::XCMPMessageSender::send_xcmp_message(
			token_owner,
			&XCMPTokenMessage::Transfer(x_currency_id, para_id, dest.clone(), amount),
		)
		.expect("Should not fail; qed");

		Ok(())
	}
}

/// This is a hack to convert from one generic type to another where we are sure
/// that both are the same type/use the same encoding.
fn convert_hack<O: Decode>(input: &impl Encode) -> O {
	input.using_encoded(|e| Decode::decode(&mut &e[..]).expect("Must be compatible; qed"))
}

impl<T: Trait> DownwardMessageHandler for Module<T> {
	fn handle_downward_message(msg: &DownwardMessage) {
		if let DownwardMessage::TransferInto(dest, amount, _) = msg {
			let dest: T::AccountId = convert_hack(dest);
			let amount: T::Balance = <T::BalanceConvertor as Convert<RelayChainBalance, T::Balance>>::convert(*amount);
			// Should not fail, but if it does, there is nothing can be done.
			let _ = T::Currency::deposit(T::RelayChainCurrencyId::get(), &dest, amount);

			Self::deposit_event(Event::<T>::ReceivedTransferFromRelayChain(dest, amount));
		}
	}
}

impl<T: Trait> XCMPMessageHandler<XCMPTokenMessage<T::AccountId, T::Balance>> for Module<T> {
	fn handle_xcmp_message(src: ParaId, msg: &XCMPTokenMessage<T::AccountId, T::Balance>) {
		match msg {
			XCMPTokenMessage::Transfer(x_currency_id, para_id, dest, amount) => {
				match x_currency_id.chain_id {
					ChainId::RelayChain => {
						// Relay chain tokens. Should not fail, but if it does, there is nothing we
						// could do.
						let _ = T::Currency::deposit(T::RelayChainCurrencyId::get(), &dest, *amount);
					}
					ChainId::ParaChain(token_owner) => {
						if T::ParaId::get() == token_owner {
							// Handle owned tokens:
							// 1. transfer between para accounts
							// 2. notify the `para_id`
							let src_para_account = src.into_account();
							// Should not fail, but if it does, there is nothing can be done.
							let _ = Self::transfer_owned_tokens_to_parachain(
								x_currency_id.clone(),
								&src_para_account,
								*para_id,
								dest,
								*amount,
							);
						} else if let Ok(currency_id) = x_currency_id.currency_id.clone().try_into() {
							// Handle known tokens.
							// Should not fail, but if it does, there is nothing can be done.
							let _ = T::Currency::deposit(currency_id, dest, *amount);
						} else {
							// Handle unknown tokens.
							UnknownBalances::<T>::mutate(dest, x_currency_id.currency_id.clone(), |total| {
								*total = total.saturating_add(*amount)
							});
						}
					}
				}

				Self::deposit_event(Event::<T>::ReceivedTransferFromParachain(
					x_currency_id.clone(),
					src,
					dest.clone(),
					*amount,
				));
			}
		}
	}
}
