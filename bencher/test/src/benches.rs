#![cfg(feature = "bench")]
#![allow(dead_code)]

use crate::mock::*;
use frame_support::assert_ok;
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

benches!(set_value, set_foo);
