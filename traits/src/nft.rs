use codec::FullCodec;
use frame_support::traits::tokens::nonfungibles::{Create, Inspect};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchResult,
};
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
	/// The balance of account.
	type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

	// Returns the next class ID to be created.
	fn next_class_id() -> Self::ClassId;
	// Sets the property of the given class.
	fn set_class_properties(class: &Self::ClassId, properties: ClassProperties) -> DispatchResult;
	// Pays for the fee to mint tokens of a particular class.
	fn pay_mint_fee(payer: &AccountId, class: &Self::ClassId, quantity: u32) -> DispatchResult;
	// Gets the base cost of minting one token for a class with default attributes.
	fn base_mint_fee() -> Self::Balance;
	// Gets the base cost of creating a new Class with default attributes.
	fn base_create_class_fee() -> Self::Balance;
}
