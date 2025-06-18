use std::{net::SocketAddr, sync::Arc};
use rand::Rng;
use tokio::sync::broadcast;
use tracing::{info, warn};

use reth_network::config::SecretKey;
use reth_network::{config::NetworkConfigBuilder, NetworkManager, EthNetworkPrimitives, NetworkHandle};
use reth_provider::test_utils::NoopProvider;
use reth_tasks::TokioTaskExecutor;
use reth_transaction_pool::{
    blobstore::InMemoryBlobStore, Pool, TransactionValidationTaskExecutor, TransactionPool,
    CoinbaseTipOrdering, EthPooledTransaction,
};

use crate::config::EmbeddedRethConfig;
use crate::transaction::EmbeddedTransaction;
use crate::metrics::TransactionMetrics;

/// High-performance embedded Reth mempool listener
/// 
/// Provides ultra-low latency transaction detection by embedding Reth's networking
/// components directly into your application, eliminating IPC overhead.
pub struct EmbeddedRethListener {
    /// Transaction pool for mempool operations  
    pool: Pool<TransactionValidationTaskExecutor<reth_transaction_pool::EthTransactionValidator<NoopProvider, EthPooledTransaction>>, CoinbaseTipOrdering<EthPooledTransaction>, InMemoryBlobStore>,
    /// Network handle for P2P operations
    _network_handle: NetworkHandle,
    /// Performance metrics
    metrics: Arc<TransactionMetrics>,
    /// Broadcast sender for transaction events
    tx_sender: broadcast::Sender<EmbeddedTransaction>,
    /// Configuration
    config: EmbeddedRethConfig,
}

impl EmbeddedRethListener {
    /// Create a new embedded Reth listener
    /// 
    /// This will:
    /// 1. Set up the transaction pool
    /// 2. Initialize P2P networking on the specified port
    /// 3. Begin listening for mempool transactions
    /// 
    /// # Example
    /// ```rust,no_run
    /// use embedded_reth_mempool::{EmbeddedRethListener, EmbeddedRethConfig};
    /// 
    /// #[tokio::main]
    /// async fn main() -> eyre::Result<()> {
    ///     let config = EmbeddedRethConfig::default();
    ///     let listener = EmbeddedRethListener::new(config).await?;
    ///     // Use listener...
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(config: EmbeddedRethConfig) -> eyre::Result<Self> {
        info!("Initializing embedded Reth mempool listener on port {}", config.listen_port);
        
        // 1. Set up transaction pool
        let blob_store = InMemoryBlobStore::default();
        let pool = Pool::eth_pool(
            TransactionValidationTaskExecutor::eth(
                NoopProvider::default(),
                blob_store.clone(),
                TokioTaskExecutor::default(),
            ),
            blob_store,
            Default::default(),
        );
        
        // 2. Set up P2P networking
        let mut nodekey = [0u8; 32];
        rand::rng().fill(&mut nodekey);
        let secret_key = SecretKey::from_slice(&nodekey).unwrap();
        
        let listen_addr: SocketAddr = format!("0.0.0.0:{}", config.listen_port).parse()?;
        let net_cfg = NetworkConfigBuilder::<EthNetworkPrimitives>::new(secret_key)
            .listener_addr(listen_addr)
            .build_with_noop_provider(config.chain_spec.clone());
            
        let (network_handle, network, _transactions, _) = NetworkManager::builder(net_cfg)
            .await?
            .transactions(pool.clone(), Default::default())
            .split_with_handle();
        
        // 3. Spawn network task
        tokio::spawn(network);
        
        // 4. Set up metrics and broadcast channel
        let metrics = Arc::new(TransactionMetrics::new());
        let (tx_sender, _) = broadcast::channel(1000); // Buffer 1000 transactions
        
        let listener = Self {
            pool,
            _network_handle: network_handle,
            metrics,
            tx_sender,
            config,
        };
        
        info!("✅ Embedded Reth mempool listener initialized successfully");
        info!("   - Listening on port: {}", listener.config.listen_port);
        info!("   - Chain: {:?}", listener.config.chain_spec.chain);
        info!("   - Discovery: {}", listener.config.enable_discovery);
        
        Ok(listener)
    }
    
    /// Subscribe to transaction events
    /// 
    /// Returns a stream of transactions with improved latency over IPC methods.
    /// Each transaction includes timing information for performance analysis.
    /// 
    /// # Example
    /// ```rust,no_run
    /// use futures::StreamExt;
    /// 
    /// let mut tx_stream = listener.subscribe();
    /// while let Some(tx) = tx_stream.next().await {
    ///     println!("TX: {} | Latency: {}μs", tx.hash_short(), tx.latency_us());
    /// }
    /// ```
    pub fn subscribe(&self) -> TransactionStream {
        let receiver = self.tx_sender.subscribe();
        TransactionStream { receiver }
    }
    
    /// Start the transaction processing loop
    /// 
    /// This begins listening for transactions from the mempool and broadcasting
    /// them to all subscribers. Call this after creating subscribers.
    pub async fn start_processing(&self) -> eyre::Result<()> {
        let mut pool_events = self.pool.new_transactions_listener();
        let metrics = self.metrics.clone();
        let sender = self.tx_sender.clone();
        
        info!("🚀 Starting transaction processing loop...");
        
        tokio::spawn(async move {
            while let Some(event) = pool_events.recv().await {
                let receipt_time = std::time::Instant::now();
                
                // Convert to our transaction type
                let embedded_tx = EmbeddedTransaction::from(&event);
                let latency_us = embedded_tx.latency_us();
                
                // Record metrics
                metrics.record_transaction(latency_us);
                
                // Broadcast to subscribers
                if let Err(e) = sender.send(embedded_tx) {
                    // Only warn if there are no receivers (expected)
                    if sender.receiver_count() > 0 {
                        warn!("Failed to broadcast transaction: {}", e);
                    }
                }
                
                let processing_time = receipt_time.elapsed();
                if processing_time.as_micros() > 100 {
                    warn!("Slow transaction processing: {}μs", processing_time.as_micros());
                }
            }
            
            info!("Transaction processing loop ended");
        });
        
        Ok(())
    }
    
    /// Get performance metrics
    pub fn metrics(&self) -> Arc<TransactionMetrics> {
        self.metrics.clone()
    }
    
    /// Get current configuration
    pub fn config(&self) -> &EmbeddedRethConfig {
        &self.config
    }
    
    /// Graceful shutdown
    pub async fn shutdown(self) -> eyre::Result<()> {
        info!("Shutting down embedded Reth listener...");
        
        // Print final metrics
        let report = self.metrics.report();
        info!("Final performance report:\n{}", report);
        
        info!("✅ Shutdown complete");
        Ok(())
    }
}

/// Stream of embedded transactions
pub struct TransactionStream {
    receiver: broadcast::Receiver<EmbeddedTransaction>,
}

impl TransactionStream {
    /// Get the next transaction
    pub async fn next(&mut self) -> Option<EmbeddedTransaction> {
        match self.receiver.recv().await {
            Ok(tx) => Some(tx),
            Err(broadcast::error::RecvError::Closed) => None,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!("Transaction stream lagged, skipped {} transactions", skipped);
                // Try to get the next transaction recursively (requires boxing)
                Box::pin(self.next()).await
            }
        }
    }
}

impl futures::Stream for TransactionStream {
    type Item = EmbeddedTransaction;
    
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;
        
        use futures::Future;
        
        
        let mut future = Box::pin(self.receiver.recv());
        match future.as_mut().poll(cx) {
            Poll::Ready(Ok(tx)) => Poll::Ready(Some(tx)),
            Poll::Ready(Err(broadcast::error::RecvError::Closed)) => Poll::Ready(None),
            Poll::Ready(Err(broadcast::error::RecvError::Lagged(skipped))) => {
                warn!("Transaction stream lagged, skipped {} transactions", skipped);
                // Wake up to try again
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }
}