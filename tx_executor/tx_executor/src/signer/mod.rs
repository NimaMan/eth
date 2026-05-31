mod local;
mod remote;
mod wire;

pub use local::LocalTransactionSigner;
pub use remote::UnixSocketTransactionSigner;
pub use wire::{SignerStatus, SignerWireRequest, SignerWireResponse, ETH_SIGNER_WIRE_SCHEMA};

use async_trait::async_trait;
use ethers_core::types::Address;

use crate::{
    error::Result,
    types::{PreparedDirectRawTransaction, SignedTransaction},
};

#[async_trait]
pub trait TransactionSigner: Send + Sync {
    fn address(&self) -> Address;

    async fn sign_direct_raw(
        &self,
        request: &PreparedDirectRawTransaction,
    ) -> Result<SignedTransaction>;
}
