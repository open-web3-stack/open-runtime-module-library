use super::*;
use frame_support::{
	log,
	traits::{Get, OnRuntimeUpgrade},
};

/// Reset WithdrawnRewards for Storage SharesAndWithdrawnRewards at specific
/// PoolId
pub struct ResetSharesAndWithdrawnRewards<T, GetPoolId>(
	sp_std::marker::PhantomData<T>,
	sp_std::marker::PhantomData<GetPoolId>,
);
impl<T: Config, GetPoolId: Get<<T as Config>::PoolId>> OnRuntimeUpgrade
	for ResetSharesAndWithdrawnRewards<T, GetPoolId>
{
	fn on_runtime_upgrade() -> Weight {
		let pool_id = GetPoolId::get();
		log::info!(
			target: "rewards",
			"ResetSharesAndWithdrawnRewards::on_runtime_upgrade execute, will reset Storage SharesAndWithdrawnRewards for Pool {:?}",
			pool_id
		);

		// reset WithdrawnRewards to default
		for (who, (_, _)) in SharesAndWithdrawnRewards::<T>::iter_prefix(&pool_id) {
			SharesAndWithdrawnRewards::<T>::mutate(&pool_id, &who, |(_, withdrawn_rewards)| {
				*withdrawn_rewards = WithdrawnRewards::<T>::new();
			});
		}

		0
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let pool_id = GetPoolId::get();

		for (_, (_, withdrawn_rewards)) in SharesAndWithdrawnRewards::<T>::iter_prefix(&pool_id) {
			assert_eq!(withdrawn_rewards, WithdrawnRewards::<T>::new());
		}

		log::info!(
			target: "rewards",
			"ResetSharesAndWithdrawnRewards for Pool {:?} done!",
			pool_id,
		);

		Ok(())
	}
}
