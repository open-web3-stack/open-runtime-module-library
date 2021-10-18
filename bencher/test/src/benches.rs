#![cfg(feature = "bench")]
#![allow(dead_code)]

use crate::{mock::*, pallet::*};
use frame_support::assert_ok;
use orml_bencher::{benches, Bencher};

fn set_value(b: &mut Bencher) {
	b.bench(|| {
		assert_ok!(Test::set_value(Origin::signed(1), 1));
	})
	.verify(|| {
		assert_eq!(Test::value(), Some(1 + 1));
	});
}

fn set_foo(b: &mut Bencher) {
	b.bench(|| {
		assert_ok!(Test::set_foo());
	});
}

fn remove_all_bar(b: &mut Bencher) {
	b.prepare(|| {
		Bar::<Runtime>::insert(1, 1);
	})
	.bench(|| {
		Test::remove_all_bar();
	});
}

benches!(set_value, set_foo, remove_all_bar);
