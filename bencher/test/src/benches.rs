#![cfg(feature = "wasm-bench")]
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

fn clear_bar(b: &mut Bencher) {
	Bar::<Runtime>::insert(1, 1);
	b.bench(|| {
		Test::clear_bar();
	});
}

fn clear_bar_with_limit(b: &mut Bencher) {
	Bar::<Runtime>::insert(1, 1);
	b.bench(|| {
		Test::clear_bar_with_limit();
	});
}

fn set_foo_with_whitelist(b: &mut Bencher) {
	b.whitelist(Bar::<Runtime>::hashed_key_for(1), true, true);
	b.whitelist(Bar::<Runtime>::hashed_key_for(2), true, false);
	b.whitelist(Foo::<Runtime>::hashed_key().to_vec(), true, true);
	b.whitelist(Value::<Runtime>::hashed_key().to_vec(), true, true);
	b.bench(|| {
		let _ = Test::set_foo();
	});
}

benches!(
	set_foo_with_whitelist,
	set_value,
	set_foo,
	clear_bar,
	clear_bar_with_limit
);
