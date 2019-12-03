use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Clone, Copy)]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}
