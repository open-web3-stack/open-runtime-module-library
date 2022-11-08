use crate::mock::*;
use frame_support::{dispatch::PostDispatchInfo, weights::Weight};

#[test]
fn used_weight_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::expect_100(RuntimeOrigin::signed(100)).unwrap();
		// Check used weight is correct
		assert_eq!(Some(Weight::from_ref_time(100)), result.actual_weight);
		// Check that the method ran correctly
		assert_eq!(Some(100), TestModule::something());

		let result: PostDispatchInfo = TestModule::expect_500(RuntimeOrigin::signed(100)).unwrap();
		assert_eq!(Some(Weight::from_ref_time(500)), result.actual_weight);
		assert_eq!(Some(600), TestModule::something());
	});
}

#[test]
fn used_weight_branch_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::expect_100_or_200(RuntimeOrigin::signed(100), false).unwrap();
		// Check used weight is correct
		assert_eq!(Some(Weight::from_ref_time(100)), result.actual_weight);
		// Check that the method ran correctly
		assert_eq!(Some(100), TestModule::something());

		let result: PostDispatchInfo = TestModule::expect_100_or_200(RuntimeOrigin::signed(100), true).unwrap();
		// Check used weight is correct
		assert_eq!(Some(Weight::from_ref_time(200)), result.actual_weight);
		// Check that the method ran correctly
		assert_eq!(Some(300), TestModule::something());
	});
}

#[test]
fn used_weight_nested_calls_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::nested_inner_methods(RuntimeOrigin::signed(100)).unwrap();
		// Check used weight is correct
		assert_eq!(Some(Weight::from_ref_time(300)), result.actual_weight);
	});
}

#[test]
fn exceed_max_weight_works() {
	new_test_ext().execute_with(|| {
		let result: PostDispatchInfo = TestModule::expect_max_weight(RuntimeOrigin::signed(100)).unwrap();
		// Check used weight is correct
		assert_eq!(Some(Weight::from_ref_time(u64::MAX)), result.actual_weight);
	});
}

#[test]
fn nested_module_calls_works() {
	new_test_ext().execute_with(|| {
		let result = TestModule::nested_extrinsic(RuntimeOrigin::signed(0)).unwrap();
		assert_eq!(result.actual_weight, Some(Weight::from_ref_time(700)));
	});
}
