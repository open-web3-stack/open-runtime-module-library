use std::sync::Arc;

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use orml_oracle_rpc_runtime_api::OracleApi as OracleRuntimeApi;

#[rpc(client, server)]
pub trait OracleApi<BlockHash, ProviderId, Key, Value> {
	#[method(name = "oracle_getValue")]
	fn get_value(&self, provider_id: ProviderId, key: Key, at: Option<BlockHash>) -> RpcResult<Option<Value>>;
	#[method(name = "oracle_getAllValues")]
	fn get_all_values(&self, provider_id: ProviderId, at: Option<BlockHash>) -> RpcResult<Vec<(Key, Option<Value>)>>;
}

/// Provides RPC methods to query oracle value.
pub struct Oracle<C, B> {
	/// Shared reference to the client.
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> Oracle<C, B> {
	/// Creates a new instance of the `Oracle` helper.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

pub enum Error {
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
impl<C, Block, ProviderId, Key, Value> OracleApiServer<<Block as BlockT>::Hash, ProviderId, Key, Value>
	for Oracle<C, Block>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	C::Api: OracleRuntimeApi<Block, ProviderId, Key, Value>,
	ProviderId: Codec,
	Key: Codec,
	Value: Codec,
{
	fn get_value(
		&self,
		provider_id: ProviderId,
		key: Key,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Option<Value>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_value(&at, provider_id, key).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get value.",
				Some(e.to_string()),
			))
			.into()
		})
	}

	fn get_all_values(
		&self,
		provider_id: ProviderId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<(Key, Option<Value>)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_all_values(&at, provider_id).map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				Error::RuntimeError.into(),
				"Unable to get all values.",
				Some(e.to_string()),
			))
			.into()
		})
	}
}
