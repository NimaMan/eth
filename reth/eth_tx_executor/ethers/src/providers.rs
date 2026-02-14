use crate::types::transaction::eip2718::TypedTransaction;
use crate::types::{Address, Block, BlockId, BlockNumber, Bytes, H256, Transaction, TransactionReceipt, U256, U64};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

#[derive(Debug, thiserror::Error, Clone)]
pub enum ProviderError {
    #[error("JSON-RPC client error: {0}")]
    JsonRpcClientError(String),
    #[error("Transport error: {0}")]
    TransportError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("RPC error {code}: {message}")]
    RpcError { code: i64, message: String },
}

#[derive(Debug, Clone)]
pub struct Http {
    url: String,
}

impl Http {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

#[derive(Debug, Clone)]
pub struct Ws {
    url: String,
    http_fallback: String,
}

impl Ws {
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        let http_fallback = if url.starts_with("ws://") {
            url.replacen("ws://", "http://", 1)
        } else if url.starts_with("wss://") {
            url.replacen("wss://", "https://", 1)
        } else {
            url.clone()
        };
        Self { url, http_fallback }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

pub trait Middleware {}

pub trait Transport: Clone + Send + Sync + 'static {
    fn rpc_url(&self) -> &str;
}

impl Transport for Http {
    fn rpc_url(&self) -> &str {
        &self.url
    }
}

impl Transport for Ws {
    fn rpc_url(&self) -> &str {
        &self.http_fallback
    }
}

#[derive(Clone, Debug)]
pub struct Provider<T: Transport> {
    transport: T,
    client: Client,
    poll_interval: Duration,
    max_polls: u32,
}

impl<T: Transport> Middleware for Provider<T> {}

impl<T: Transport> Provider<T> {
    fn new(transport: T) -> Result<Self, ProviderError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| ProviderError::TransportError(e.to_string()))?;

        Ok(Self {
            transport,
            client,
            poll_interval: Duration::from_millis(400),
            max_polls: 120,
        })
    }

    fn block_param(block: Option<BlockId>) -> Result<Value, ProviderError> {
        match block.unwrap_or(BlockNumber::Latest.into()) {
            BlockId::Hash(hash) => Ok(json!({ "blockHash": hash })),
            BlockId::Number(number) => serde_json::to_value(number)
                .map_err(|e| ProviderError::SerializationError(e.to_string())),
        }
    }

    async fn rpc_request<R: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<R, ProviderError> {
        #[derive(Debug, Deserialize)]
        struct RpcErrorBody {
            code: i64,
            message: String,
        }

        #[derive(Debug, Deserialize)]
        struct RpcEnvelope<T> {
            #[allow(dead_code)]
            jsonrpc: String,
            #[allow(dead_code)]
            id: u64,
            result: Option<T>,
            error: Option<RpcErrorBody>,
        }

        let body = json!({
            "jsonrpc": "2.0",
            "id": 1u64,
            "method": method,
            "params": params,
        });

        let response = self
            .client
            .post(self.transport.rpc_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::TransportError(e.to_string()))?;

        let envelope: RpcEnvelope<R> = response
            .json()
            .await
            .map_err(|e| ProviderError::SerializationError(e.to_string()))?;

        if let Some(err) = envelope.error {
            return Err(ProviderError::RpcError {
                code: err.code,
                message: err.message,
            });
        }

        envelope
            .result
            .ok_or_else(|| ProviderError::JsonRpcClientError("missing result".to_string()))
    }

    pub async fn get_transaction_count(
        &self,
        address: Address,
        block: Option<BlockId>,
    ) -> Result<U256, ProviderError> {
        let block = Self::block_param(block)?;
        self.rpc_request("eth_getTransactionCount", json!([address, block])).await
    }

    pub async fn get_balance(&self, address: Address, block: Option<BlockId>) -> Result<U256, ProviderError> {
        let block = Self::block_param(block)?;
        self.rpc_request("eth_getBalance", json!([address, block])).await
    }

    pub async fn estimate_gas(
        &self,
        tx: &TypedTransaction,
        block: Option<BlockId>,
    ) -> Result<U256, ProviderError> {
        let tx = serde_json::to_value(tx).map_err(|e| ProviderError::SerializationError(e.to_string()))?;
        let params = if let Some(block) = block {
            json!([tx, Self::block_param(Some(block))?])
        } else {
            json!([tx])
        };
        self.rpc_request("eth_estimateGas", params).await
    }

    pub async fn get_code(&self, address: Address, block: Option<BlockId>) -> Result<Bytes, ProviderError> {
        let block = Self::block_param(block)?;
        self.rpc_request("eth_getCode", json!([address, block])).await
    }

    pub async fn call(
        &self,
        tx: &TypedTransaction,
        block: Option<BlockId>,
    ) -> Result<Bytes, ProviderError> {
        let tx = serde_json::to_value(tx).map_err(|e| ProviderError::SerializationError(e.to_string()))?;
        let block = Self::block_param(block)?;
        self.rpc_request("eth_call", json!([tx, block])).await
    }

    pub async fn get_block_number(&self) -> Result<U64, ProviderError> {
        self.rpc_request("eth_blockNumber", json!([])).await
    }

    pub async fn get_gas_price(&self) -> Result<U256, ProviderError> {
        self.rpc_request("eth_gasPrice", json!([])).await
    }

    pub async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<Option<TransactionReceipt>, ProviderError> {
        self.rpc_request("eth_getTransactionReceipt", json!([tx_hash])).await
    }

    pub async fn get_block_with_txs<B: Into<BlockId>>(
        &self,
        block: B,
    ) -> Result<Option<Block<Transaction>>, ProviderError> {
        match block.into() {
            BlockId::Hash(hash) => self.rpc_request("eth_getBlockByHash", json!([hash, true])).await,
            BlockId::Number(number) => {
                let number = serde_json::to_value(number)
                    .map_err(|e| ProviderError::SerializationError(e.to_string()))?;
                self.rpc_request("eth_getBlockByNumber", json!([number, true])).await
            }
        }
    }

    pub async fn send_raw_transaction(
        &self,
        raw_tx: Bytes,
    ) -> Result<PendingTransaction<T>, ProviderError> {
        let raw = format!("0x{}", hex::encode(raw_tx.as_ref()));
        let tx_hash: H256 = self.rpc_request("eth_sendRawTransaction", json!([raw])).await?;
        Ok(PendingTransaction::new(self.clone(), tx_hash))
    }

    pub async fn sign_transaction(
        &self,
        _tx: &TypedTransaction,
        _from: Address,
    ) -> Result<crate::types::Signature, ProviderError> {
        Err(ProviderError::JsonRpcClientError(
            "sign_transaction is not available on lightweight provider".to_string(),
        ))
    }
}

impl Provider<Ws> {
    pub async fn connect(url: &str) -> Result<Self, ProviderError> {
        Self::new(Ws::new(url))
    }
}

impl TryFrom<&str> for Provider<Http> {
    type Error = ProviderError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Provider::new(Http::new(value))
    }
}

impl TryFrom<String> for Provider<Http> {
    type Error = ProviderError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Provider::try_from(value.as_str())
    }
}

impl TryFrom<&String> for Provider<Http> {
    type Error = ProviderError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Provider::try_from(value.as_str())
    }
}

pub struct PendingTransaction<T: Transport> {
    tx_hash: H256,
    inner: Pin<Box<dyn Future<Output = Result<Option<TransactionReceipt>, ProviderError>> + Send>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Transport> Unpin for PendingTransaction<T> {}

impl<T: Transport> PendingTransaction<T> {
    fn new(provider: Provider<T>, tx_hash: H256) -> Self {
        let poll_interval = provider.poll_interval;
        let max_polls = provider.max_polls;

        let inner = async move {
            for _ in 0..max_polls {
                if let Some(receipt) = provider.get_transaction_receipt(tx_hash).await? {
                    return Ok(Some(receipt));
                }
                tokio::time::sleep(poll_interval).await;
            }
            Ok(None)
        };

        Self {
            tx_hash,
            inner: Box::pin(inner),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn tx_hash(&self) -> H256 {
        self.tx_hash
    }
}

impl<T: Transport> Future for PendingTransaction<T> {
    type Output = Result<Option<TransactionReceipt>, ProviderError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.inner.as_mut().poll(cx)
    }
}
