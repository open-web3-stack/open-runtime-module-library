pub use self::gen_client::Client as OracleClient;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use orml_oracle_rpc_runtime_api::OracleApi as OracleRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait OracleApi<BlockHash, Key, Value> {
	#[rpc(name = "oracle_getNoOp")]
	fn get_no_op(&self, key: Key, at: Option<BlockHash>) -> Result<Option<Value>>;
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

impl<C, Block, Key, Value> OracleApi<<Block as BlockT>::Hash, Key, Value> for Oracle<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: OracleRuntimeApi<Block, Key, Value>,
	Key: Codec,
	Value: Codec + ToString,
{
	fn get_no_op(&self, key: Key, at: Option<<Block as BlockT>::Hash>) -> Result<Option<Value>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.get_no_op(&at, key)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get value.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}
}
