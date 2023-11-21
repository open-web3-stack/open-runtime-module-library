//!This pallet allows users to create secure reversible payments that keep
//! funds locked in a merchant's account until the off-chain goods are confirmed
//! to be received. Each payment gets assigned its own *judge* that can help
//! resolve any disputes between the two parties.

//! ## Terminology
//!
//! - Created: A payment has been created and the amount arrived to its
//!   destination but it's locked.
//! - NeedsReview: The payment has bee disputed and is awaiting settlement by a
//!   judge.
//! - IncentivePercentage: A small share of the payment amount is held in escrow
//!   until a payment is completed/cancelled. The Incentive Percentage
//!   represents this value.
//! - Resolver Account: A resolver account is assigned to every payment created,
//!   this account has the privilege to cancel/release a payment that has been
//!   disputed.
//! - Remark: The pallet allows to create payments by optionally providing some
//!   extra(limited) amount of bytes, this is referred to as Remark. This can be
//!   used by a marketplace to separate/tag payments.
//! - CancelBufferBlockLength: This is the time window where the recipient can
//!   dispute a cancellation request from the payment creator.

//! Extrinsics
//!
//! - `pay` - Create an payment for the given currencyid/amount
//! - `pay_with_remark` - Create a payment with a remark, can be used to tag
//!   payments
//! - `release` - Release the payment amount to recipent
//! - `cancel` - Allows the recipient to cancel the payment and release the
//!   payment amount to creator
//! - `resolve_release_payment` - Allows assigned judge to release a payment
//! - `resolve_cancel_payment` - Allows assigned judge to cancel a payment
//! - `request_refund` - Allows the creator of the payment to trigger cancel
//!   with a buffer time.
//! - `claim_refund` - Allows the creator to claim payment refund after buffer
//!   time
//! - `dispute_refund` - Allows the recipient to dispute the payment request of
//!   sender
//! - `request_payment` - Create a payment that can be completed by the sender
//!   using the `accept_and_pay` extrinsic.
//! - `accept_and_pay` - Allows the sender to fulfill a payment request created
//!   by a recipient

//! Types
//!
//! The `PaymentDetail` struct stores information about the payment/escrow. A
//! "payment" in virto network is similar to an escrow, it is used to guarantee
//! proof of funds and can be released once an agreed upon condition has reached
//! between the payment creator and recipient. The payment lifecycle is tracked
//! using the state field.

//! The `PaymentState` enum tracks the possible states that a payment can be in.
//! When a payment is 'completed' or 'cancelled' it is removed from storage and
//! hence not tracked by a state.
#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod types;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	pub use crate::{
		types::{DisputeResolver, FeeHandler, PaymentDetail, PaymentHandler, PaymentState, ScheduledTask, Task},
		weights::WeightInfo,
	};
	use frame_support::{
		dispatch::DispatchResultWithPostInfo, fail, pallet_prelude::*, require_transactional,
		storage::bounded_btree_map::BoundedBTreeMap, traits::tokens::BalanceStatus,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{MultiCurrency, MultiReservableCurrency};
	use sp_runtime::{
		traits::{CheckedAdd, Saturating},
		Percent,
	};
	use sp_std::vec::Vec;

	pub type BalanceOf<T> = <<T as Config>::Asset as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type AssetIdOf<T> = <<T as Config>::Asset as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;
	pub type BoundedDataOf<T> = BoundedVec<u8, <T as Config>::MaxRemarkLength>;
	/// type of ScheduledTask used by the pallet
	pub type ScheduledTaskOf<T> = ScheduledTask<BlockNumberFor<T>>;
	/// list of ScheduledTasks, stored as a BoundedBTreeMap
	pub type ScheduledTaskList<T> = BoundedBTreeMap<
		(
			<T as frame_system::Config>::AccountId,
			<T as frame_system::Config>::AccountId,
		),
		ScheduledTaskOf<T>,
		<T as Config>::MaxRemarkLength,
	>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's
		/// definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// the type of assets this pallet can hold in payment
		type Asset: MultiReservableCurrency<Self::AccountId>;
		/// Dispute resolution account
		type DisputeResolver: DisputeResolver<Self::AccountId>;
		/// Fee handler trait
		type FeeHandler: FeeHandler<Self>;
		/// Incentive percentage - amount withheld from sender
		#[pallet::constant]
		type IncentivePercentage: Get<Percent>;
		/// Maximum permitted size of `Remark`
		#[pallet::constant]
		type MaxRemarkLength: Get<u32>;
		/// Buffer period - number of blocks to wait before user can claim
		/// canceled payment
		#[pallet::constant]
		type CancelBufferBlockLength: Get<BlockNumberFor<Self>>;
		/// Buffer period - number of blocks to wait before user can claim
		/// canceled payment
		#[pallet::constant]
		type MaxScheduledTaskListLength: Get<u32>;
		//// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn payment)]
	/// Payments created by a user, this method of storageDoubleMap is chosen
	/// since there is no usecase for listing payments by provider/currency. The
	/// payment will only be referenced by the creator in any transaction of
	/// interest. The storage map keys are the creator and the recipient, this
	/// also ensures that for any (sender,recipient) combo, only a single
	/// payment is active. The history of payment is not stored.
	pub(super) type Payment<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // payment creator
		Blake2_128Concat,
		T::AccountId, // payment recipient
		PaymentDetail<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn tasks)]
	/// Store the list of tasks to be executed in the on_idle function
	pub(super) type ScheduledTasks<T: Config> = StorageValue<_, ScheduledTaskList<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new payment has been created
		PaymentCreated {
			from: T::AccountId,
			asset: AssetIdOf<T>,
			amount: BalanceOf<T>,
			remark: Option<BoundedDataOf<T>>,
		},
		/// Payment amount released to the recipient
		PaymentReleased { from: T::AccountId, to: T::AccountId },
		/// Payment has been cancelled by the creator
		PaymentCancelled { from: T::AccountId, to: T::AccountId },
		/// A payment that NeedsReview has been resolved by Judge
		PaymentResolved {
			from: T::AccountId,
			to: T::AccountId,
			recipient_share: Percent,
		},
		/// the payment creator has created a refund request
		PaymentCreatorRequestedRefund {
			from: T::AccountId,
			to: T::AccountId,
			expiry: BlockNumberFor<T>,
		},
		/// the refund request from creator was disputed by recipient
		PaymentRefundDisputed { from: T::AccountId, to: T::AccountId },
		/// Payment request was created by recipient
		PaymentRequestCreated { from: T::AccountId, to: T::AccountId },
		/// Payment request was completed by sender
		PaymentRequestCompleted { from: T::AccountId, to: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The selected payment does not exist
		InvalidPayment,
		/// The selected payment cannot be released
		PaymentAlreadyReleased,
		/// The selected payment already exists and is in process
		PaymentAlreadyInProcess,
		/// Action permitted only for whitelisted users
		InvalidAction,
		/// Payment is in review state and cannot be modified
		PaymentNeedsReview,
		/// Unexpeted math error
		MathError,
		/// Payment request has not been created
		RefundNotRequested,
		/// Dispute period has not passed
		DisputePeriodNotPassed,
		/// The automatic cancelation queue cannot accept
		RefundQueueFull,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Hook that execute when there is leftover space in a block
		/// This function will look for any pending scheduled tasks that can
		/// be executed and will process them.
		fn on_idle(now: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			const MAX_TASKS_TO_PROCESS: usize = 5;
			// used to read the task list
			let mut used_weight = T::WeightInfo::remove_task();
			let cancel_weight = T::WeightInfo::cancel();

			// calculate count of tasks that can be processed with remaining weight
			let possible_task_count: usize = remaining_weight
				.saturating_sub(used_weight)
				.saturating_div(cancel_weight.ref_time())
				.ref_time()
				.try_into()
				.unwrap_or(MAX_TASKS_TO_PROCESS);

			ScheduledTasks::<T>::mutate(|tasks| {
				let mut task_list: Vec<_> = tasks
					.clone()
					.into_iter()
					.take(possible_task_count)
					// leave out tasks in the future
					.filter(|(_, ScheduledTask { when, task })| when <= &now && matches!(task, Task::Cancel))
					.collect();

				// order by oldest task to process
				task_list.sort_by(|(_, t), (_, x)| x.when.cmp(&t.when));

				while !task_list.is_empty() && used_weight.all_lte(remaining_weight) {
					if let Some((account_pair, _)) = task_list.pop() {
						used_weight = used_weight.saturating_add(cancel_weight);
						// remove the task form the tasks storage
						tasks.remove(&account_pair);

						// process the cancel payment
						if <Self as PaymentHandler<T>>::settle_payment(
							&account_pair.0,
							&account_pair.1,
							Percent::from_percent(0),
						)
						.is_err()
						{
							// log the payment refund failure
							log::warn!(
								target: "runtime::payments",
								"Warning: Unable to process payment refund!"
							);
						} else {
							// emit the cancel event if the refund was successful
							Self::deposit_event(Event::PaymentCancelled {
								from: account_pair.0,
								to: account_pair.1,
							});
						}
					}
				}
			});
			used_weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// This allows any user to create a new payment, that releases only to
		/// specified recipient The only action is to store the details of this
		/// payment in storage and reserve the specified amount. User also has
		/// the option to add a remark, this remark can then be used to run
		/// custom logic and trigger alternate payment flows. the specified
		/// amount.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::pay(T::MaxRemarkLength::get()))]
		pub fn pay(
			origin: OriginFor<T>,
			recipient: T::AccountId,
			asset: AssetIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>,
			remark: Option<BoundedDataOf<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			// create PaymentDetail and add to storage
			let payment_detail = <Self as PaymentHandler<T>>::create_payment(
				&who,
				&recipient,
				asset,
				amount,
				PaymentState::Created,
				T::IncentivePercentage::get(),
				remark.as_ref().map(|x| x.as_slice()),
			)?;
			// reserve funds for payment
			<Self as PaymentHandler<T>>::reserve_payment_amount(&who, &recipient, payment_detail)?;
			// emit paymentcreated event
			Self::deposit_event(Event::PaymentCreated {
				from: who,
				asset,
				amount,
				remark,
			});
			Ok(().into())
		}

		/// Release any created payment, this will transfer the reserved amount
		/// from the creator of the payment to the assigned recipient
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::release())]
		pub fn release(origin: OriginFor<T>, to: T::AccountId) -> DispatchResultWithPostInfo {
			let from = ensure_signed(origin)?;

			// ensure the payment is in Created state
			let payment = Payment::<T>::get(&from, &to).ok_or(Error::<T>::InvalidPayment)?;
			ensure!(payment.state == PaymentState::Created, Error::<T>::InvalidAction);

			// release is a settle_payment with 100% recipient_share
			<Self as PaymentHandler<T>>::settle_payment(&from, &to, Percent::from_percent(100))?;

			Self::deposit_event(Event::PaymentReleased { from, to });
			Ok(().into())
		}

		/// Cancel a payment in created state, this will release the reserved
		/// back to creator of the payment. This extrinsic can only be called by
		/// the recipient of the payment
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::cancel())]
		pub fn cancel(origin: OriginFor<T>, creator: T::AccountId) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			if let Some(payment) = Payment::<T>::get(&creator, &who) {
				match payment.state {
					// call settle payment with recipient_share=0, this refunds the sender
					PaymentState::Created => {
						<Self as PaymentHandler<T>>::settle_payment(&creator, &who, Percent::from_percent(0))?;
						Self::deposit_event(Event::PaymentCancelled { from: creator, to: who });
					}
					// if the payment is in state PaymentRequested, remove from storage
					PaymentState::PaymentRequested => Payment::<T>::remove(&creator, &who),
					_ => fail!(Error::<T>::InvalidAction),
				}
			}
			Ok(().into())
		}

		/// This extrinsic is used to resolve disputes between the creator and
		/// recipient of the payment.
		/// This extrinsic allows the assigned judge to
		/// cancel/release/partial_release the payment.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::resolve_payment())]
		pub fn resolve_payment(
			origin: OriginFor<T>,
			from: T::AccountId,
			recipient: T::AccountId,
			recipient_share: Percent,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let account_pair = (from, recipient);
			// ensure the caller is the assigned resolver
			if let Some(payment) = Payment::<T>::get(&account_pair.0, &account_pair.1) {
				ensure!(who == payment.resolver_account, Error::<T>::InvalidAction);
				ensure!(
					payment.state != PaymentState::PaymentRequested,
					Error::<T>::InvalidAction
				);
				if matches!(payment.state, PaymentState::RefundRequested { .. }) {
					ScheduledTasks::<T>::mutate(|tasks| {
						tasks.remove(&account_pair);
					})
				}
			}
			// try to update the payment to new state
			<Self as PaymentHandler<T>>::settle_payment(&account_pair.0, &account_pair.1, recipient_share)?;
			Self::deposit_event(Event::PaymentResolved {
				from: account_pair.0,
				to: account_pair.1,
				recipient_share,
			});
			Ok(().into())
		}

		/// Allow the creator of a payment to initiate a refund that will return
		/// the funds after a configured amount of time that the reveiver has to
		/// react and oppose the request
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::request_refund())]
		pub fn request_refund(origin: OriginFor<T>, recipient: T::AccountId) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Payment::<T>::try_mutate(who.clone(), recipient.clone(), |maybe_payment| -> DispatchResult {
				// ensure the payment exists
				let payment = maybe_payment.as_mut().ok_or(Error::<T>::InvalidPayment)?;
				// refunds only possible for payments in created state
				ensure!(payment.state == PaymentState::Created, Error::<T>::InvalidAction);

				// set the payment to requested refund
				let current_block = frame_system::Pallet::<T>::block_number();
				let cancel_block = current_block
					.checked_add(&T::CancelBufferBlockLength::get())
					.ok_or(Error::<T>::MathError)?;

				ScheduledTasks::<T>::try_mutate(|task_list| -> DispatchResult {
					task_list
						.try_insert(
							(who.clone(), recipient.clone()),
							ScheduledTask {
								task: Task::Cancel,
								when: cancel_block,
							},
						)
						.map_err(|_| Error::<T>::RefundQueueFull)?;
					Ok(())
				})?;

				payment.state = PaymentState::RefundRequested { cancel_block };

				Self::deposit_event(Event::PaymentCreatorRequestedRefund {
					from: who,
					to: recipient,
					expiry: cancel_block,
				});

				Ok(())
			})?;

			Ok(().into())
		}

		/// Allow payment recipient to dispute the refund request from the
		/// payment creator This does not cancel the request, instead sends the
		/// payment to a NeedsReview state The assigned resolver account can
		/// then change the state of the payment after review.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::dispute_refund())]
		pub fn dispute_refund(origin: OriginFor<T>, creator: T::AccountId) -> DispatchResultWithPostInfo {
			use PaymentState::*;
			let who = ensure_signed(origin)?;

			Payment::<T>::try_mutate(
				creator.clone(),
				who.clone(), // should be called by the payment recipient
				|maybe_payment| -> DispatchResult {
					// ensure the payment exists
					let payment = maybe_payment.as_mut().ok_or(Error::<T>::InvalidPayment)?;
					// ensure the payment is in Requested Refund state
					match payment.state {
						RefundRequested { cancel_block } => {
							ensure!(
								cancel_block > frame_system::Pallet::<T>::block_number(),
								Error::<T>::InvalidAction
							);

							payment.state = PaymentState::NeedsReview;

							// remove the payment from scheduled tasks
							ScheduledTasks::<T>::try_mutate(|task_list| -> DispatchResult {
								task_list
									.remove(&(creator.clone(), who.clone()))
									.ok_or(Error::<T>::InvalidAction)?;
								Ok(())
							})?;

							Self::deposit_event(Event::PaymentRefundDisputed { from: creator, to: who });
						}
						_ => fail!(Error::<T>::InvalidAction),
					}

					Ok(())
				},
			)?;

			Ok(().into())
		}

		// Creates a new payment with the given details. This can be called by the
		// recipient of the payment to create a payment and then completed by the sender
		// using the `accept_and_pay` extrinsic.  The payment will be in
		// PaymentRequested State and can only be modified by the `accept_and_pay`
		// extrinsic.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::request_payment())]
		pub fn request_payment(
			origin: OriginFor<T>,
			from: T::AccountId,
			asset: AssetIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let to = ensure_signed(origin)?;

			// create PaymentDetail and add to storage
			<Self as PaymentHandler<T>>::create_payment(
				&from,
				&to,
				asset,
				amount,
				PaymentState::PaymentRequested,
				Percent::from_percent(0),
				None,
			)?;

			Self::deposit_event(Event::PaymentRequestCreated { from, to });

			Ok(().into())
		}

		// This extrinsic allows the sender to fulfill a payment request created by a
		// recipient. The amount will be transferred to the recipient and payment
		// removed from storage
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::accept_and_pay())]
		pub fn accept_and_pay(origin: OriginFor<T>, to: T::AccountId) -> DispatchResultWithPostInfo {
			let from = ensure_signed(origin)?;

			let payment = Payment::<T>::get(&from, &to).ok_or(Error::<T>::InvalidPayment)?;

			ensure!(
				payment.state == PaymentState::PaymentRequested,
				Error::<T>::InvalidAction
			);

			// reserve all the fees from the sender
			<Self as PaymentHandler<T>>::reserve_payment_amount(&from, &to, payment)?;

			// release the payment and delete the payment from storage
			<Self as PaymentHandler<T>>::settle_payment(&from, &to, Percent::from_percent(100))?;

			Self::deposit_event(Event::PaymentRequestCompleted { from, to });

			Ok(().into())
		}
	}

	impl<T: Config> PaymentHandler<T> for Pallet<T> {
		/// The function will create a new payment. The fee and incentive
		/// amounts will be calculated and the `PaymentDetail` will be added to
		/// storage.
		#[require_transactional]
		fn create_payment(
			from: &T::AccountId,
			recipient: &T::AccountId,
			asset: AssetIdOf<T>,
			amount: BalanceOf<T>,
			payment_state: PaymentState<T>,
			incentive_percentage: Percent,
			remark: Option<&[u8]>,
		) -> Result<PaymentDetail<T>, sp_runtime::DispatchError> {
			Payment::<T>::try_mutate(
				from,
				recipient,
				|maybe_payment| -> Result<PaymentDetail<T>, sp_runtime::DispatchError> {
					// only payment requests can be overwritten
					if let Some(payment) = maybe_payment {
						ensure!(
							payment.state == PaymentState::PaymentRequested,
							Error::<T>::PaymentAlreadyInProcess
						);
					}

					// Calculate incentive amount - this is to insentivise the user to release
					// the funds once a transaction has been completed
					let incentive_amount = incentive_percentage.mul_floor(amount);

					let mut new_payment = PaymentDetail {
						asset,
						amount,
						incentive_amount,
						state: payment_state,
						resolver_account: T::DisputeResolver::get_resolver_account(),
						fee_detail: None,
					};

					// Calculate fee amount - this will be implemented based on the custom
					// implementation of the fee provider
					let (fee_recipient, fee_percent) = T::FeeHandler::apply_fees(from, recipient, &new_payment, remark);
					let fee_amount = fee_percent.mul_floor(amount);
					new_payment.fee_detail = Some((fee_recipient, fee_amount));

					*maybe_payment = Some(new_payment.clone());

					Ok(new_payment)
				},
			)
		}

		/// The function will reserve the fees+transfer amount from the `from`
		/// account. After reserving the payment.amount will be transferred to
		/// the recipient but will stay in Reserve state.
		#[require_transactional]
		fn reserve_payment_amount(from: &T::AccountId, to: &T::AccountId, payment: PaymentDetail<T>) -> DispatchResult {
			let fee_amount = payment.fee_detail.map(|(_, f)| f).unwrap_or_else(|| 0u32.into());

			let total_fee_amount = payment.incentive_amount.saturating_add(fee_amount);
			let total_amount = total_fee_amount.saturating_add(payment.amount);

			// reserve the total amount from payment creator
			T::Asset::reserve(payment.asset, from, total_amount)?;
			// transfer payment amount to recipient -- keeping reserve status
			T::Asset::repatriate_reserved(payment.asset, from, to, payment.amount, BalanceStatus::Reserved)?;
			Ok(())
		}

		/// This function allows the caller to settle the payment by specifying
		/// a recipient_share this will unreserve the fee+incentive to sender
		/// and unreserve transferred amount to recipient if the settlement is a
		/// release (ie recipient_share=100), the fee is transferred to
		/// fee_recipient For cancelling a payment, recipient_share = 0
		/// For releasing a payment, recipient_share = 100
		/// In other cases, the custom recipient_share can be specified
		fn settle_payment(from: &T::AccountId, to: &T::AccountId, recipient_share: Percent) -> DispatchResult {
			Payment::<T>::try_mutate(from, to, |maybe_payment| -> DispatchResult {
				let payment = maybe_payment.take().ok_or(Error::<T>::InvalidPayment)?;

				// unreserve the incentive amount and fees from the owner account
				match payment.fee_detail {
					Some((fee_recipient, fee_amount)) => {
						T::Asset::unreserve(payment.asset, from, payment.incentive_amount.saturating_add(fee_amount));
						// transfer fee to marketplace if operation is not cancel
						if recipient_share != Percent::zero() {
							T::Asset::transfer(
								payment.asset,
								from,           // fee is paid by payment creator
								&fee_recipient, // account of fee recipient
								fee_amount,     // amount of fee
							)?;
						}
					}
					None => {
						T::Asset::unreserve(payment.asset, from, payment.incentive_amount);
					}
				};

				// Unreserve the transfer amount
				T::Asset::unreserve(payment.asset, to, payment.amount);

				let amount_to_recipient = recipient_share.mul_floor(payment.amount);
				let amount_to_sender = payment.amount.saturating_sub(amount_to_recipient);
				// send share to recipient
				T::Asset::transfer(payment.asset, to, from, amount_to_sender)?;

				Ok(())
			})?;
			Ok(())
		}

		fn get_payment_details(from: &T::AccountId, to: &T::AccountId) -> Option<PaymentDetail<T>> {
			Payment::<T>::get(from, to)
		}
	}
}
