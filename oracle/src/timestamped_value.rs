use codec::{Decode, Encode};

#[derive(Encode, Decode, Default, Debug, Eq, PartialEq, Clone)]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}
