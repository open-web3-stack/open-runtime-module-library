//! RPC interface for the orml-tokens pallet.

pub use self::gen_client::Client as TokensClient;
use codec::{Codec};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use orml_tokens_rpc_runtime_api::TokensApi as TokensRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, MaybeDisplay, MaybeSerializeDeserialize, Member},
};
use frame_support::pallet_prelude::*;

use std::sync::Arc;

#[rpc]
pub trait TokensRpcApi<BlockHash, CurrencyId, Balance> {
	#[rpc(name = "existential_deposit")]
	fn query_existential_deposit(&self, currency_id: CurrencyId, at: Option<BlockHash>) -> Result<Balance>;
}

/// A struct that implements the [`TokensRpcApi`].
pub struct Tokens<C, P> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<P>,
}

impl<C, P> Tokens<C, P> {
	/// Create new `Tokens` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

/// Error type of this RPC api.
pub enum Error {
	/// The transaction was not decodable.
	DecodeError,
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
			Error::DecodeError => 2,
		}
	}
}

impl<C, Block, CurrencyId, Balance> TokensRpcApi<<Block as BlockT>::Hash, CurrencyId, Balance>
	for Tokens<C, Block>
where
	Block: BlockT,
	C: 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: TokensRuntimeApi<Block, CurrencyId, Balance>,
	Balance: Codec + MaybeDisplay + Copy + TryInto<NumberOrHex>,
	CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord
{
	fn query_existential_deposit(
		&self,
		currency_id: CurrencyId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Balance> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));

		api.query_existential_deposit(&at, currency_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to query dispatch info.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
