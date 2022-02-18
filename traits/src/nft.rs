use codec::FullCodec;
use frame_support::traits::tokens::nonfungibles::{Create, Inspect};
use sp_runtime::{DispatchResult, traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize}};
use sp_std::fmt::Debug;

/// Trait to complement the Inspect trait
pub trait InspectExtended<AccountId>: Inspect<AccountId> {
	/// The balance of account.
	type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	/// The number of NFTs assigned to `who`.
	fn balance(who: &AccountId) -> Self::Balance;

	/// Get the next token ID to be minted for a Class
	fn next_token_id(class: Self::ClassId) -> Self::InstanceId;
}

// Supplement trait to the nonfungibles::Create trait
pub trait CreateExtended<AccountId, ClassProperties>: Create<AccountId> {
	fn next_class_id() -> Self::ClassId;
	fn set_class_properties(class: &Self::ClassId, properties: ClassProperties) -> DispatchResult;
}