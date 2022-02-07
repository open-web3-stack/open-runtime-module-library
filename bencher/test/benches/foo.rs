use orml_bencher_test::mock::{AllPalletsWithSystem, Block};
orml_bencher::run_benches!(AllPalletsWithSystem, Block);
