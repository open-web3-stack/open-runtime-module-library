//! RPC interface for the orml-tokens pallet.
use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay},
};

pub use self::gen_client::Client as TokensClient;
pub use orml_tokens_rpc_runtime_api::TokensApi as TokensRuntimeApi;

#[rpc]
pub trait TokensApi<BlockHash, CurrencyId, Balance> {
	#[rpc(name = "tokens_queryExistentialDeposit")]
	fn query_existential_deposit(&self, currency_id: CurrencyId, at: Option<BlockHash>) -> Result<NumberOrHex>;
}

/// A struct that implements the [`TokensApi`].
pub struct Tokens<C, P> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> Tokens<C, P> {
	/// Create new `Tokens` with the given reference to the client.
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

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

impl<C, Block, CurrencyId, Balance> TokensApi<<Block as BlockT>::Hash, CurrencyId, Balance> for Tokens<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: TokensRuntimeApi<Block, CurrencyId, Balance>,
	Balance: Codec + MaybeDisplay + Copy + TryInto<NumberOrHex> + std::fmt::Debug,
	CurrencyId: Codec,
{
	fn query_existential_deposit(
		&self,
		currency_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<NumberOrHex> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));

		let balance = api.query_existential_deposit(&at, currency_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to query existential_deposit.".into(),
			data: Some(format!("{:?}", e).into()),
		});

		let try_into_rpc_balance = |value: Balance| {
			value.try_into().map_err(|_| RpcError {
				code: ErrorCode::InvalidParams,
				message: format!("{} doesn't fit in NumberOrHex representation", value),
				data: None,
			})
		};

		try_into_rpc_balance(balance?)
	}
}
