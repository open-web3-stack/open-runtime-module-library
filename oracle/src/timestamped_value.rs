use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use rstd::fmt::{Display, Formatter, Result};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}

impl<Value: Display, Moment: Display> Display for TimestampedValue<Value, Moment> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut Formatter) -> Result {
		write!(f, "TimestampedValue {{ value: {}, timestamp: {} }}", self.value, self.timestamp)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut Formatter) -> Result {
		Ok(())
	}
}
