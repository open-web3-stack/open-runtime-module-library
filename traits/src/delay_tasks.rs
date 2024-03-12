use sp_runtime::DispatchResult;

pub trait DelayedTask {
	fn pre_delay(&self) -> DispatchResult;
	fn pre_delayed_execute(&self) -> DispatchResult;
	fn delayed_execute(&self) -> DispatchResult;
	fn on_cancel(&self) -> DispatchResult;
}

pub trait DelayTasksManager<Task, BlockNumber> {
	fn add_delay_task(task: Task, delay_blocks: BlockNumber) -> DispatchResult;
}

#[macro_export]
macro_rules! define_combined_delayed_task {
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

		impl DelayedTask for $combined_name {
			fn pre_delay(&self) -> DispatchResult {
				match self {
					$(
						$combined_name::$task(t) => t.pre_delay(),
					)*
				}
			}
			fn pre_delayed_execute(&self) -> DispatchResult {
				match self {
					$(
						$combined_name::$task(t) => t.pre_delayed_execute(),
					)*
				}
			}
			fn delayed_execute(&self) -> DispatchResult {
				match self {
					$(
						$combined_name::$task(t) => t.delayed_execute(),
					)*
				}
			}
			fn on_cancel(&self) -> DispatchResult {
				match self {
					$(
						$combined_name::$task(t) => t.on_cancel(),
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
