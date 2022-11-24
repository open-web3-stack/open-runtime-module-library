use crate::{arithmetic, Happened};
use codec::{Codec, FullCodec, MaxEncodedLen};
pub use frame_support::{
	traits::{BalanceStatus, DefensiveSaturating, LockIdentifier},
	transactional, RuntimeDebug,
};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchError, DispatchResult,
};
use sp_std::{
	cmp::{Eq, Ordering, PartialEq},
	fmt::Debug,
	result,
};

#[derive(Eq, PartialEq, RuntimeDebug)]
pub enum RestrictStatus {
	Pass,
	Reject,
}

#[derive(PartialEq, Eq, RuntimeDebug)]
pub enum ExtrinsicRestrictError {
	Rejected,
}

pub trait CheckRestrictStatus<Params> {
	fn check_restrict_status(params: &Params) -> RestrictStatus;
}

impl<Params> CheckRestrictStatus<Params> for () {
	fn check_restrict_status(_: &Params) -> RestrictStatus {
		RestrictStatus::Pass
	}
}

pub trait OnExecute<Params> {
	fn execute(params: &Params) -> DispatchResult;
}

impl<Params> OnExecute<Params> for () {
	fn execute(_: &Params) -> DispatchResult {
		Ok(())
	}
}

pub trait ConvertError {
	fn convert_error(error: ExtrinsicRestrictError) -> DispatchError;
}

impl ConvertError for () {
	fn convert_error(_: ExtrinsicRestrictError) -> DispatchError {
		DispatchError::Other("restrict error")
	}
}

pub trait ExtrinsicRestrictExecution<Params> {
	type CheckRestrictStatus: CheckRestrictStatus<Params>;

	type PrePassedExecute: OnExecute<Params>;

	type PostPassedExecute: OnExecute<Params>;

	type ErrorConvertor: ConvertError;

	fn restrict_execute(params: &Params, f: impl FnOnce() -> DispatchResult) -> DispatchResult {
		match Self::CheckRestrictStatus::check_restrict_status(params) {
			RestrictStatus::Pass => {
				Self::PrePassedExecute::execute(params)?;

				f()?;

				Self::PostPassedExecute::execute(params)?;
			}
			RestrictStatus::Reject => {
				return Err(Self::ErrorConvertor::convert_error(ExtrinsicRestrictError::Rejected));
			}
		};

		Ok(())
	}
}

impl<Params> ExtrinsicRestrictExecution<Params> for () {
	type CheckRestrictStatus = ();
	type PrePassedExecute = ();
	type PostPassedExecute = ();
	type ErrorConvertor = ();
}
