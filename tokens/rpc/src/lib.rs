//! RPC interface for the orml-tokens pallet.
use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay},
};

pub use orml_tokens_rpc_runtime_api::TokensApi as TokensRuntimeApi;

#[rpc(client, server)]
pub trait TokensApi<BlockHash, CurrencyId, Balance> {
	#[method(name = "tokens_queryExistentialDeposit")]
	fn query_existential_deposit(&self, currency_id: CurrencyId, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;
}

/// Provides RPC methods to query existential deposit of currency.
pub struct Tokens<C, P> {
	/// Shared reference to the client.
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> Tokens<C, P> {
	/// Creates a new instance of the `Tokens` helper.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i32 {
	fn from(e: Error) -> i32 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

#[async_trait]
impl<C, Block, CurrencyId, Balance> TokensApiServer<<Block as BlockT>::Hash, CurrencyId, Balance> for Tokens<C, Block>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: TokensRuntimeApi<Block, CurrencyId, Balance>,
	Balance: Codec + MaybeDisplay + Copy + TryInto<NumberOrHex> + Send + Sync + 'static,
	CurrencyId: Codec,
{
	fn query_existential_deposit(
		&self,
		currency_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let balance = api.query_existential_deposit(&at, currency_id).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to query existential deposit.",
				Some(e.to_string()),
			))
		});

		let try_into_rpc_balance = |value: Balance| {
			value.try_into().map_err(|_| {
				JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
					ErrorCode::InvalidParams.code(),
					format!("{} doesn't fit in NumberOrHex representation", value),
					None::<()>,
				)))
			})
		};

		try_into_rpc_balance(balance?)
	}
}
