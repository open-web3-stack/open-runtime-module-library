#![cfg(test)]

use super::mock::*;
use super::weights::ModuleWeights;
use frame_support::dispatch::PostDispatchInfo;

#[test]
fn set_value() {
	let weight = ModuleWeights::<Runtime>::set_value() + ModuleWeights::<Runtime>::set_foo();
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Test::set_value(RuntimeOrigin::signed(1), 1),
			Ok(PostDispatchInfo {
				actual_weight: Some(weight),
				..Default::default()
			})
		);
	});
}
