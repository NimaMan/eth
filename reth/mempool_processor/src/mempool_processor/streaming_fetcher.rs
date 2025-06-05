/// Streaming transaction fetcher that gets only NEW transactions as they arrive
/// This replaces the polling approach with true real-time streaming
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{info, warn, debug, error};
use eyre::Result;
use ethers::prelude::*;
use ethers::providers::{Provider, Ws};
use futures::stream::StreamExt;
use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::mempool_processor::types::TransactionView;

pub struct StreamingFetcher {
    /// WebSocket provider for real-time subscription
    ws_provider: Arc<Provider<Ws>>,
    /// HTTP provider for fetching full transaction details
    http_provider: Arc<Provider<Http>>,
    /// Channel to send new transactions
    tx_sender: mpsc::Sender<TransactionView>,
    /// Channel to receive new transactions
    tx_receiver: Arc<Mutex<mpsc::Receiver<TransactionView>>>,
    /// Track subscription status
    is_subscribed: Arc<Mutex<bool>>,
}

impl StreamingFetcher {
    /// Create a new streaming fetcher
    pub async fn new(ws_url: &str, http_url: &str) -> Result<Self> {
        info!("🚀 Initializing STREAMING transaction fetcher");
        info!("   📡 WebSocket: {}", ws_url);
        info!("   🔗 HTTP: {}", http_url);
        
        // Connect to WebSocket
        let ws = Ws::connect(ws_url).await?;
        let ws_provider = Provider::new(ws);
        
        // Create HTTP provider
        let http_provider = Provider::<Http>::try_from(http_url)?;
        
        // Create channel for streaming transactions
        let (tx_sender, tx_receiver) = mpsc::channel(10000); // Large buffer for burst handling
        
        Ok(Self {
            ws_provider: Arc::new(ws_provider),
            http_provider: Arc::new(http_provider),
            tx_sender,
            tx_receiver: Arc::new(Mutex::new(tx_receiver)),
            is_subscribed: Arc::new(Mutex::new(false)),
        })
    }
    
    /// Start the subscription to new pending transactions
    pub async fn start_subscription(&self) -> Result<()> {
        let mut is_subscribed = self.is_subscribed.lock().await;
        if *is_subscribed {
            warn!("Already subscribed to new transactions");
            return Ok(());
        }
        
        info!("📡 Starting WebSocket subscription to newPendingTransactions");
        
        // Subscribe to new pending transactions
        let mut stream = self.ws_provider.subscribe_pending_txs().await?;
        *is_subscribed = true;
        
        info!("✅ Subscription active - receiving only NEW transactions as they arrive!");
        
        // Spawn task to handle incoming transaction hashes
        let tx_sender = self.tx_sender.clone();
        let http_provider = self.http_provider.clone();
        
        tokio::spawn(async move {
            let mut count = 0;
            let start_time = Instant::now();
            
            while let Some(tx_hash) = stream.next().await {
                count += 1;
                
                // Log progress every 1000 transactions
                if count % 1000 == 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let tps = count as f64 / elapsed;
                    debug!("📊 Streamed {} new transactions ({:.1} TPS)", count, tps);
                }
                
                // Fetch full transaction details
                let tx_sender_clone = tx_sender.clone();
                let http_provider_clone = http_provider.clone();
                
                // Spawn task to fetch transaction details without blocking the stream
                tokio::spawn(async move {
                    match http_provider_clone.get_transaction(tx_hash).await {
                        Ok(Some(tx)) => {
                            // Convert to TransactionView
                            let tx_view = TransactionView {
                                hash: tx.hash.as_bytes().to_vec(),
                                from: tx.from.as_bytes().to_vec(),
                                to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                                value: tx.value,
                                gas_price: tx.gas_price,
                                gas_limit: Some(tx.gas),
                                nonce: Some(tx.nonce),
                                input_data: Some(tx.input.to_vec()),
                            };
                            
                            // Send to processing queue
                            if let Err(e) = tx_sender_clone.send(tx_view).await {
                                error!("Failed to send transaction to queue: {}", e);
                            }
                        }
                        Ok(None) => {
                            // Transaction might have been removed quickly
                            debug!("Transaction {} not found", tx_hash);
                        }
                        Err(e) => {
                            warn!("Error fetching transaction {}: {}", tx_hash, e);
                        }
                    }
                });
            }
            
            error!("❌ WebSocket stream ended unexpectedly!");
        });
        
        Ok(())
    }
    
    /// Get new transactions from the stream (non-blocking)
    pub async fn get_new_transactions(&self, max_batch: usize) -> Result<Vec<TransactionView>> {
        let mut receiver = self.tx_receiver.lock().await;
        let mut transactions = Vec::with_capacity(max_batch);
        
        // Get up to max_batch transactions without blocking
        while transactions.len() < max_batch {
            match receiver.try_recv() {
                Ok(tx) => transactions.push(tx),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    error!("Transaction channel disconnected!");
                    break;
                }
            }
        }
        
        Ok(transactions)
    }
    
    /// Get statistics about the streaming performance
    pub async fn get_stats(&self) -> (bool, usize) {
        let is_subscribed = *self.is_subscribed.lock().await;
        let pending = self.tx_sender.capacity() - self.tx_sender.max_capacity();
        (is_subscribed, pending)
    }
}