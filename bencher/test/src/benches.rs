#![cfg(feature = "bench")]
#![allow(dead_code)]

use crate::{pallet_test::*, mock::*};
use frame_support::{assert_ok, StorageMap};
use orml_bencher::{benches, Bencher};

fn set_value(b: &mut Bencher) {
	b.bench(|| {
		assert_ok!(TestPallet::set_value(Origin::signed(1), 1));
	})
	.verify(|| {
		assert_eq!(TestPallet::value(), Some(1 + 1));
	});
}

fn set_foo(b: &mut Bencher) {
	b.bench(|| {
		assert_ok!(TestPallet::set_foo());
	});
}

fn remove_all_bar(b: &mut Bencher) {
	b.prepare(|| {
		crate::pallet_test::Bar::<DefaultInstance>::insert(1, 1);
	})
	.bench(|| {
		TestPallet::remove_all_bar();
	});
}

benches!(set_value, set_foo, remove_all_bar);
