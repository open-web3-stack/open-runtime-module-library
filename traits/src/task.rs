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

// pub trait DelayedTask {
// 	fn pre_delay(&self) -> DispatchResult;
// 	fn pre_delayed_execute(&self) -> DispatchResult;
// 	fn delayed_execute(&self) -> DispatchResult;
// 	fn on_cancel(&self) -> DispatchResult;
// }

// pub trait DelayTasksManager<Task, BlockNumber> {
// 	fn add_delay_task(task: Task, delay_blocks: BlockNumber) -> DispatchResult;
// }

// #[macro_export]
// macro_rules! define_combined_task {
// 	(
// 		$(#[$meta:meta])*
// 		$vis:vis enum $combined_name:ident {
// 			$(
// 				$task:ident ( $vtask:ident $(<$($generic:tt),*>)? )
// 			),+ $(,)?
// 		}
// 	) => {
// 		$(#[$meta])*
// 		$vis enum $combined_name {
// 			$(
// 				$task($vtask $(<$($generic),*>)?),
// 			)*
// 		}

// 		impl DispatchableTask for $combined_name {
// 			fn dispatch(self, weight: Weight) -> TaskResult {
// 				match self {
// 					$(
// 						$combined_name::$task(t) => t.dispatch(weight),
// 					)*
// 				}
// 			}
// 		}

//         $(
//             impl From<$vtask $(<$($generic),*>)?> for $combined_name {
//                 fn from(t: $vtask $(<$($generic),*>)?) -> Self{
//                     $combined_name::$task(t)
//                 }
//             }
//         )*
// 	};
// }

// #[macro_export]
// macro_rules! define_combined_delayed_task {
// 	(
// 		$(#[$meta:meta])*
// 		$vis:vis enum $combined_name:ident {
// 			$(
// 				$task:ident ( $vtask:ident $(<$($generic:tt),*>)? )
// 			),+ $(,)?
// 		}
// 	) => {
// 		$(#[$meta])*
// 		$vis enum $combined_name {
// 			$(
// 				$task($vtask $(<$($generic),*>)?),
// 			)*
// 		}

// 		impl DelayedTask for $combined_name {
// 			fn pre_delay(&self) -> DispatchResult {
// 				match self {
// 					$(
// 						$combined_name::$task(t) => t.pre_delay(),
// 					)*
// 				}
// 			}
// 			fn pre_delayed_execute(&self) -> DispatchResult {
// 				match self {
// 					$(
// 						$combined_name::$task(t) => t.pre_delayed_execute(),
// 					)*
// 				}
// 			}
// 			fn delayed_execute(&self) -> DispatchResult {
// 				match self {
// 					$(
// 						$combined_name::$task(t) => t.delayed_execute(),
// 					)*
// 				}
// 			}
// 			fn on_cancel(&self) -> DispatchResult {
// 				match self {
// 					$(
// 						$combined_name::$task(t) => t.on_cancel(),
// 					)*
// 				}
// 			}
// 		}

//         $(
//             impl From<$vtask $(<$($generic),*>)?> for $combined_name {
//                 fn from(t: $vtask $(<$($generic),*>)?) -> Self{
//                     $combined_name::$task(t)
//                 }
//             }
//         )*
// 	};
// }
