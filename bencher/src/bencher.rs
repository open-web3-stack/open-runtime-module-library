use codec::{Decode, Encode};
use sp_std::prelude::Vec;

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct Bencher {
	pub method: Vec<u8>,
	pub elapses: Vec<u128>,
	pub keys: Vec<u8>,
	pub warnings: Vec<u8>,
}

impl Bencher {
	pub fn with_name(name: &str) -> Self {
		Self {
			method: name.as_bytes().to_vec(),
			..Default::default()
		}
	}
}

#[inline]
fn black_box<T>(dummy: T) -> T {
	let ret = unsafe { sp_std::ptr::read_volatile(&dummy) };
	sp_std::mem::forget(dummy);
	ret
}

#[allow(unused_variables, clippy::let_and_return)]
impl Bencher {
	pub fn whitelist(&mut self, key: Vec<u8>, read: bool, write: bool) {
		#[cfg(not(feature = "std"))]
		crate::bench::whitelist(key, read, write);
	}

	pub fn before_run(&self) {
		#[cfg(not(feature = "std"))]
		{
			crate::bench::commit_db();
			crate::bench::wipe_db();
		}
	}

	pub fn bench<T, F>(&mut self, mut inner: F) -> T
	where
		F: FnMut() -> T,
	{
		#[cfg(not(feature = "std"))]
		{
			crate::bench::commit_db();
			crate::bench::reset_read_write_count();
			crate::bench::start_timer();
		}

		let ret = black_box(inner());

		#[cfg(not(feature = "std"))]
		{
			let elapsed = crate::bench::end_timer().saturating_sub(crate::bench::redundant_time());
			self.elapses.push(elapsed);

			crate::bench::commit_db();

			// changed keys
			self.keys = crate::bench::read_written_keys();
			self.warnings = crate::bench::warnings();
		}

		ret
	}
}
