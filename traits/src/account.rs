use impl_trait_for_tuples::impl_for_tuples;
use orml_utilities::with_transaction_result;
use sp_runtime::DispatchResult;

pub trait MergeAccount<AccountId> {
	fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult;
}

#[impl_for_tuples(5)]
impl<AccountId> MergeAccount<AccountId> for Tuple {
	fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult {
		with_transaction_result(|| {
			for_tuples!( #( {
                Tuple::merge_account(source, dest)?;
            } )* );
			Ok(())
		})
	}
}
