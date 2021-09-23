#![cfg(feature = "bench")]

#![allow(dead_code)]
#![allow(unused_imports)]

use frame_support::assert_ok;
use orml_bencher::{Bencher, bench};
use orml_bencher_test::mock::{TestPallet, Origin, AllPalletsWithSystem, Block};

fn set_value(b: &mut Bencher) {
    b.bench(|| {
        assert_ok!(TestPallet::set_value(Origin::signed(1), 1));
    }).verify(|| {
        assert_eq!(TestPallet::value(), Some(1 + 1));
    });
}

fn set_foo(b: &mut Bencher) {
    b.bench(|| {
        assert_ok!(TestPallet::set_foo());
    });
}

bench!(AllPalletsWithSystem, Block, set_value, set_foo);