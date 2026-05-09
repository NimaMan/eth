use crate::error::{EthTxExecutorError, Result};
use ethers_core::types::{Address, BlockId, BlockNumber, U256};
use ethers_providers::{Http, Middleware, Provider};
use tokio::sync::Mutex;

pub struct NonceManager {
    provider: Provider<Http>,
    address: Address,
    next_nonce: Mutex<Option<U256>>,
}

impl NonceManager {
    pub fn new(provider: Provider<Http>, address: Address) -> Self {
        Self {
            provider,
            address,
            next_nonce: Mutex::new(None),
        }
    }

    pub async fn reserve(&self, requested: Option<U256>) -> Result<U256> {
        if let Some(nonce) = requested {
            return Ok(nonce);
        }

        let mut guard = self.next_nonce.lock().await;
        let nonce = match *guard {
            Some(value) => value,
            None => self
                .provider
                .get_transaction_count(self.address, Some(BlockId::Number(BlockNumber::Pending)))
                .await
                .map_err(|err| EthTxExecutorError::Nonce(err.to_string()))?,
        };
        *guard = Some(nonce + U256::one());
        Ok(nonce)
    }

    pub async fn invalidate(&self) {
        *self.next_nonce.lock().await = None;
    }
}
