use frame_support::traits::tokens::nonfungibles::Inspect;
use parity_scale_codec::FullCodec;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize};
use sp_std::fmt::Debug;

/// Trait to complement the Inspect trait
pub trait InspectExtended<AccountId>: Inspect<AccountId> {
	/// The balance of account.
	type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The number of NFTs assigned to `who`.
	fn balance(who: &AccountId) -> Self::Balance;

	/// Get the next token ID to be minted for a Class
	fn next_token_id(class: Self::CollectionId) -> Self::ItemId;
}
