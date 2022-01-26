use codec::FullCodec;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchResult, DispatchError,
};
use sp_std::{fmt::Debug, vec::*};

/// Abstraction over a non-fungible token system.
#[allow(clippy::upper_case_acronyms)]
pub trait NFT<AccountId> {
	/// The NFT class identifier.
	type ClassId: Default + Copy;

	/// The NFT token identifier.
	type TokenId: Default + Copy;

	/// The balance of account.
	type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The number of NFTs assigned to `who`.
	fn balance(who: &AccountId) -> Self::Balance;

	/// The owner of the given token ID. Returns `None` if the token does not
	/// exist.
	fn owner(token: (Self::ClassId, Self::TokenId)) -> Option<AccountId>;

	/// Transfer the given token ID from one account to another.
	fn transfer(from: &AccountId, to: &AccountId, token: (Self::ClassId, Self::TokenId)) -> DispatchResult;
}

// This trait provides interface to manage NFTs
#[allow(clippy::upper_case_acronyms)]
pub trait ManageNFT<AccountId, CID, Attributes> {
	/// The NFT class identifier.
	type ClassId: Default + Copy;

	/// The NFT token identifier.
	type TokenId: Default + Copy;

	/// The balance of account.
	type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// To mint new NFT tokens.
	fn mint(
		who: AccountId,
		to: AccountId,
		class_id: Self::ClassId,
		metadata: CID,
		attributes: Attributes,
		quantity: u32,
	) -> Result<Vec<Self::TokenId>, DispatchError>;

	/// To burn a NFT token.
	fn burn(who: AccountId, token: (Self::ClassId, Self::TokenId), remark: Option<Vec<u8>>) -> DispatchResult;
}
