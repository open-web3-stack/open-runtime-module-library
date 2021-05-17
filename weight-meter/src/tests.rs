use crate::mock::*;
use frame_support::weights::PostDispatchInfo;

#[test]
fn used_weight_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::expect_100(Origin::signed(100)).unwrap();
		// Check used weight is correct
		assert_eq!(Some(100), result.actual_weight);
		// Check that the method ran correctly
		assert_eq!(Some(100), TestModule::something());

		let result: PostDispatchInfo = TestModule::expect_500(Origin::signed(100)).unwrap();
		assert_eq!(Some(500), result.actual_weight);
		assert_eq!(Some(600), TestModule::something());
	});
}

#[test]
fn used_weight_branch_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::expect_100_or_200(Origin::signed(100), false).unwrap();
		// Check used weight is correct
		assert_eq!(Some(100), result.actual_weight);
		// Check that the method ran correctly
		assert_eq!(Some(100), TestModule::something());

		let result: PostDispatchInfo = TestModule::expect_100_or_200(Origin::signed(100), true).unwrap();
		// Check used weight is correct
		assert_eq!(Some(200), result.actual_weight);
		// Check that the method ran correctly
		assert_eq!(Some(300), TestModule::something());
	});
}

#[test]
fn used_weight_nested_calls_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::nested_inner_methods(Origin::signed(100)).unwrap();
		// Check used weight is correct
		assert_eq!(Some(300), result.actual_weight);
	});
}

#[test]
fn exceed_max_weight_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::expect_max_weight(Origin::signed(100)).unwrap();
		// Check used weight is correct
		assert_eq!(Some(u64::MAX), result.actual_weight);
	});
}

#[test]
fn nested_module_calls_works() {
	new_test_ext().execute_with(|| {
		let result = TestModule::nested_extrinsic(Origin::signed(0)).unwrap();
		assert_eq!(result.actual_weight, Some(700));
	});
}
