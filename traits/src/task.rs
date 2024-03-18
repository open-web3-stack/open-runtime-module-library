use frame_support::weights::Weight;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::DispatchResult;
use sp_runtime::RuntimeDebug;

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TaskResult {
	pub result: DispatchResult,
	pub used_weight: Weight,
	pub finished: bool,
}

/// Dispatchable tasks
pub trait DispatchableTask {
	fn dispatch(self, weight: Weight) -> TaskResult;
}

#[cfg(feature = "std")]
impl DispatchableTask for () {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		unimplemented!()
	}
}

#[macro_export]
macro_rules! define_combined_task {
	(
		$(#[$meta:meta])*
		$vis:vis enum $combined_name:ident {
			$(
				$task:ident ( $vtask:ident $(<$($generic:tt),*>)? )
			),+ $(,)?
		}
	) => {
		$(#[$meta])*
		$vis enum $combined_name {
			$(
				$task($vtask $(<$($generic),*>)?),
			)*
		}

		impl DispatchableTask for $combined_name {
			fn dispatch(self, weight: Weight) -> TaskResult {
				match self {
					$(
						$combined_name::$task(t) => t.dispatch(weight),
					)*
				}
			}
		}

        $(
            impl From<$vtask $(<$($generic),*>)?> for $combined_name {
                fn from(t: $vtask $(<$($generic),*>)?) -> Self{
                    $combined_name::$task(t)
                }
            }
        )*
	};
}

pub trait DelayTasksManager<Task, BlockNumber> {
	fn add_delay_task(task: Task, delay_blocks: BlockNumber) -> DispatchResult;
}

pub trait DelayTaskHooks<Task> {
	fn pre_delay(task: &Task) -> DispatchResult;
	fn pre_delayed_execute(task: &Task) -> DispatchResult;
	fn on_cancel(task: &Task) -> DispatchResult;
}

impl<Task> DelayTaskHooks<Task> for () {
	fn pre_delay(_: &Task) -> DispatchResult {
		Ok(())
	}
	fn pre_delayed_execute(_: &Task) -> DispatchResult {
		Ok(())
	}
	fn on_cancel(_: &Task) -> DispatchResult {
		Ok(())
	}
}
