use codec::{Decode, Encode};
use sr_primitives::RuntimeDebug;

#[derive(Encode, Decode, Default, RuntimeDebug, Eq, PartialEq, Clone)]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}
