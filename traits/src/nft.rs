use codec::FullCodec;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize};
use sp_std::fmt::Debug;
use frame_support::traits::tokens::nonfungibles::Inspect;

/// Trait to complement the Inspect trait
#[allow(clippy::upper_case_acronyms)]
pub trait InspectExtended<AccountId>: Inspect<AccountId> {
	/// The balance of account.
	type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The number of NFTs assigned to `who`.
	fn balance(who: &AccountId) -> Self::Balance;

	/// Get the next token ID to be minted for a Class
	fn next_token_id(class: Self::ClassId) -> Self::InstanceId;
}
