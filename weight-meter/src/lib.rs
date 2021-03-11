// TODO: research if there's a better way
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate lazy_static;

use sp_std::sync::Mutex;

pub use weight_meter_procedural::*;

#[derive(Default)]
struct State {
	pub used_weight: u64,
	pub nested: usize,
}

lazy_static! {
	static ref STATE: Mutex<State> = Mutex::new(State::default());
}

pub fn using(m: u64) {
	let mut s = STATE.lock().unwrap();
	s.used_weight = s.used_weight.saturating_add(m);
}

pub fn used_weight() -> u64 {
	STATE.lock().unwrap().used_weight
}

pub fn start_with(m: u64) {
	let mut s = STATE.lock().unwrap();
	if s.nested == 0 {
		s.used_weight = m;
	}
	s.nested = s.nested.saturating_add(1);
}

pub fn end() {
	let mut s = STATE.lock().unwrap();
	s.nested = s.nested.saturating_sub(1);
}
