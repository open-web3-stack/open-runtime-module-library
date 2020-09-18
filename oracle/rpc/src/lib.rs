use std::sync::Arc;

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub use self::gen_client::Client as OracleClient;
pub use orml_oracle_rpc_runtime_api::OracleApi as OracleRuntimeApi;

#[rpc]
pub trait OracleApi<BlockHash, ProviderId, Key, Value> {
	#[rpc(name = "oracle_getValue")]
	fn get_value(&self, provider_id: ProviderId, key: Key, at: Option<BlockHash>) -> Result<Option<Value>>;
	#[rpc(name = "oracle_getAllValues")]
	fn get_all_values(&self, provider_id: ProviderId, at: Option<BlockHash>) -> Result<Vec<(Key, Option<Value>)>>;
}

/// A struct that implements the [`OracleApi`].
pub struct Oracle<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> Oracle<C, B> {
	/// Create new `Oracle` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Oracle {
			client,
			_marker: Default::default(),
		}
	}
}

pub enum Error {
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

impl<C, Block, ProviderId, Key, Value> OracleApi<<Block as BlockT>::Hash, ProviderId, Key, Value> for Oracle<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
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
	) -> Result<Option<Value>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or(
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash,
		));
		api.get_value(&at, provider_id, key).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get value.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_all_values(
		&self,
		provider_id: ProviderId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Vec<(Key, Option<Value>)>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or(
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash,
		));
		api.get_all_values(&at, provider_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get all values.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
