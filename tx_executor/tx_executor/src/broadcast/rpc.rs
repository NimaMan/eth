use crate::error::{EthTxExecutorError, Result};
use ethers_core::types::H256;
use ethers_providers::{Http, Provider};

#[derive(Clone)]
pub struct RpcBroadcaster {
    provider: Provider<Http>,
}

impl RpcBroadcaster {
    pub fn new(provider: Provider<Http>) -> Self {
        Self { provider }
    }

    pub async fn send_raw_transaction(&self, raw_tx_hex: &str) -> Result<H256> {
        self.provider
            .request("eth_sendRawTransaction", [raw_tx_hex])
            .await
            .map_err(|err| EthTxExecutorError::Broadcast(err.to_string()))
    }
}
