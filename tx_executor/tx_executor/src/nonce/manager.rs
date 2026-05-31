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

#[cfg(test)]
mod tests {
    use super::*;
    use ethers_core::types::H160;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[tokio::test]
    async fn invalidate_forces_pending_nonce_refetch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let server_requests = Arc::clone(&requests);
        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let request_index = server_requests.fetch_add(1, Ordering::SeqCst);
                let result = if request_index == 0 { "0x3d" } else { "0x45" };
                let body = format!(r#"{{"jsonrpc":"2.0","id":1,"result":"{result}"}}"#);
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let mut buffer = [0u8; 2048];
                let _ = stream.read(&mut buffer).await;
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let provider = Provider::<Http>::try_from(format!("http://{addr}").as_str()).unwrap();
        let manager = NonceManager::new(provider, H160::zero());

        assert_eq!(manager.reserve(None).await.unwrap(), U256::from(0x3d));
        assert_eq!(manager.reserve(None).await.unwrap(), U256::from(0x3e));
        assert_eq!(requests.load(Ordering::SeqCst), 1);

        manager.invalidate().await;

        assert_eq!(manager.reserve(None).await.unwrap(), U256::from(0x45));
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }
}
