use codec::{Decode, Encode};
use sr_primitives::RuntimeDebug;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Clone, Copy)]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}
