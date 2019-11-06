use rstd::prelude::Vec;

pub trait OperatorProvider<AccountId> {
	// Make sure `who` has permission to feed data
	fn can_feed_data(who: &AccountId) -> bool;

	// return a list of operators
	fn operators() -> Vec<AccountId>;
}
