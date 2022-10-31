#![cfg(feature = "bench")]
#![allow(dead_code)]

use crate::{mock::*, pallet::*};
use frame_support::assert_ok;
use orml_bencher::{benches, Bencher};

fn set_value(b: &mut Bencher) {
	let result = b.bench(|| Test::set_value(RuntimeOrigin::signed(1), 1));
	assert_ok!(result);
	assert_eq!(Test::value(), Some(1 + 1));
}

fn set_foo(b: &mut Bencher) {
	b.bench(|| {
		let _ = Test::set_foo();
	});
}

fn remove_all_bar(b: &mut Bencher) {
	Bar::<Runtime>::insert(1, 1);
	b.bench(|| {
		Test::remove_all_bar();
	});
}

fn remove_all_bar_with_limit(b: &mut Bencher) {
	b.count_clear_prefix();
	Bar::<Runtime>::insert(1, 1);
	b.bench(|| {
		Test::remove_all_bar_with_limit();
	});
}

fn whitelist(b: &mut Bencher) {
	b.whitelist(Bar::<Runtime>::hashed_key_for(1), true, true);
	b.whitelist(Bar::<Runtime>::hashed_key_for(2), true, false);
	b.whitelist(Foo::<Runtime>::hashed_key().to_vec(), true, true);
	b.whitelist(Value::<Runtime>::hashed_key().to_vec(), true, true);
	b.bench(|| {
		let _ = Test::set_foo();
	});
}

benches!(
	whitelist,
	set_value,
	set_foo,
	remove_all_bar,
	remove_all_bar_with_limit
);
