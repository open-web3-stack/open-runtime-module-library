use frame_support::transactional;
use impl_trait_for_tuples::impl_for_tuples;
use sp_runtime::DispatchResult;

pub trait MergeAccount<AccountId> {
	fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult;
}

#[impl_for_tuples(5)]
impl<AccountId> MergeAccount<AccountId> for Tuple {
	#[transactional]
	fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult {
		for_tuples!( #( Tuple::merge_account(source, dest)?; )* );
		Ok(())
	}
}
