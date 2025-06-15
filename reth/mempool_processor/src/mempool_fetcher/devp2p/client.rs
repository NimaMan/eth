/*
 * DevP2P Transaction Client
 * 
 * ALGORITHMIC DESCRIPTION:
 * This module implements direct peer-to-peer communication with Ethereum nodes
 * to receive transaction announcements immediately when they hit the network,
 * bypassing RPC polling delays for true real-time scam detection.
 * 
 * PROTOCOL FLOW:
 * 1. Establish DevP2P connection to Ethereum peer
 * 2. Perform ETH protocol handshake
 * 3. Subscribe to NewPooledTransactionHashes messages
 * 4. Request full transaction data via GetPooledTransactions
 * 5. Process transactions immediately for scam detection
 * 
 * PERFORMANCE TARGET:
 * - Transaction arrival: <10ms (vs 12-92ms RPC polling)
 * - End-to-end latency: <10ms (vs 92ms current)
 * - SLA compliance: 95%+ (vs current 4.5%)
 */

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::timeout;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn, debug, error};
use eyre::{Result, eyre};
use bytes::{Bytes, BytesMut, BufMut};
use rlp::{Rlp, RlpStream, Encodable, Decodable};
use secp256k1::{SecretKey, PublicKey, Secp256k1};
use sha3::{Keccak256, Digest};
use rand::{Rng, RngCore};

use crate::mempool_fetcher::types::TransactionView;

/// DevP2P protocol message types for transaction pool communication
#[derive(Debug, Clone)]
pub enum TxPoolMessage {
    /// Announcement of new transaction hashes in the pool
    NewPooledTransactionHashes {
        hashes: Vec<[u8; 32]>,
        types: Vec<u8>,
        sizes: Vec<u32>,
    },
    /// Request for full transaction data
    GetPooledTransactions {
        request_id: u64,
        hashes: Vec<[u8; 32]>,
    },
    /// Response with full transaction data
    PooledTransactions {
        request_id: u64,
        transactions: Vec<TransactionView>,
    },
}

/// DevP2P protocol states
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolState {
    Disconnected,
    Connecting,
    HandshakeInitiated,
    HandshakeCompleted,
    EthProtocolNegotiated,
    Connected,
    Failed(String),
}

/// DevP2P message IDs for ETH protocol
pub const ETH_STATUS: u8 = 0x00;
const ETH_NEW_BLOCK_HASHES: u8 = 0x01;
const ETH_TRANSACTIONS: u8 = 0x02;
const ETH_GET_BLOCK_HEADERS: u8 = 0x03;
const ETH_BLOCK_HEADERS: u8 = 0x04;
const ETH_GET_BLOCK_BODIES: u8 = 0x05;
const ETH_BLOCK_BODIES: u8 = 0x06;
const ETH_NEW_BLOCK: u8 = 0x07;
pub const ETH_NEW_POOLED_TRANSACTION_HASHES: u8 = 0x08;
pub const ETH_GET_POOLED_TRANSACTIONS: u8 = 0x09;
pub const ETH_POOLED_TRANSACTIONS: u8 = 0x0a;

/// DevP2P handshake authentication data
#[derive(Debug)]
struct AuthData {
    signature: [u8; 65],
    initiator_pubkey: [u8; 64],
    nonce: [u8; 32],
    version: u8,
}

/// DevP2P client for real-time transaction fetching
pub struct DevP2pClient {
    /// Connection to Ethereum peer
    pub(crate) peer_address: String,
    /// TCP connection to peer
    connection: Arc<Mutex<Option<TcpStream>>>,
    /// Protocol state
    state: Arc<RwLock<ProtocolState>>,
    /// Channel for receiving transaction announcements
    tx_announcements: Arc<RwLock<mpsc::Receiver<TxPoolMessage>>>,
    /// Channel for sending transaction requests
    tx_requests: Arc<RwLock<mpsc::Sender<TxPoolMessage>>>,
    /// Cache for pending transaction requests
    pending_requests: Arc<RwLock<HashMap<u64, Vec<[u8; 32]>>>>,
    /// Our node's private key for authentication
    private_key: SecretKey,
    /// Remote peer's public key (if known)
    remote_pubkey: Arc<RwLock<Option<PublicKey>>>,
    /// Performance metrics
    connection_start_time: Instant,
    transactions_received: Arc<RwLock<u64>>,
    average_latency_ms: Arc<RwLock<f64>>,
    /// Request counter for generating unique request IDs
    request_counter: Arc<RwLock<u64>>,
}

impl DevP2pClient {
    /// Create a new DevP2P client for the specified peer
    pub async fn new(peer_address: &str) -> Result<Self> {
        info!("🔗 Initializing DevP2P client for peer: {}", peer_address);
        
        // Generate a private key for this session
        let mut rng = rand::thread_rng();
        let mut key_bytes = [0u8; 32];
        rng.fill(&mut key_bytes);
        let private_key = SecretKey::from_slice(&key_bytes)
            .expect("Failed to create private key");
        
        // Create channels for bidirectional communication
        let (tx_sender, _tx_receiver) = mpsc::channel(1000);
        let (announcement_sender, announcement_receiver) = mpsc::channel(1000);
        
        let client = Self {
            peer_address: peer_address.to_string(),
            connection: Arc::new(Mutex::new(None)),
            state: Arc::new(RwLock::new(ProtocolState::Disconnected)),
            tx_announcements: Arc::new(RwLock::new(announcement_receiver)),
            tx_requests: Arc::new(RwLock::new(tx_sender)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            private_key,
            remote_pubkey: Arc::new(RwLock::new(None)),
            connection_start_time: Instant::now(),
            transactions_received: Arc::new(RwLock::new(0)),
            average_latency_ms: Arc::new(RwLock::new(0.0)),
            request_counter: Arc::new(RwLock::new(0)),
        };
        
        // Start the connection process
        let client_clone = client.clone_for_connection().await;
        tokio::spawn(Self::connection_manager(
            client_clone,
            announcement_sender,
        ));
        
        info!("✅ DevP2P client initialized successfully");
        Ok(client)
    }
    
    /// Create a clone with shared state for connection management
    async fn clone_for_connection(&self) -> Self {
        Self {
            peer_address: self.peer_address.clone(),
            connection: self.connection.clone(),
            state: self.state.clone(),
            tx_announcements: self.tx_announcements.clone(),
            tx_requests: self.tx_requests.clone(),
            pending_requests: self.pending_requests.clone(),
            private_key: self.private_key,
            remote_pubkey: self.remote_pubkey.clone(),
            connection_start_time: self.connection_start_time,
            transactions_received: self.transactions_received.clone(),
            average_latency_ms: self.average_latency_ms.clone(),
            request_counter: self.request_counter.clone(),
        }
    }
    
    /// Connection manager that handles the full DevP2P protocol
    async fn connection_manager(
        mut client: Self,
        announcement_sender: mpsc::Sender<TxPoolMessage>,
    ) -> Result<()> {
        info!("🔌 Starting DevP2P connection manager for {}", client.peer_address);
        
        loop {
            // Check current state and take appropriate action
            let current_state = {
                let state = client.state.read().await;
                state.clone()
            };
            
            match current_state {
                ProtocolState::Disconnected => {
                    if let Err(e) = client.establish_connection().await {
                        error!("Failed to establish connection: {}", e);
                        client.set_state(ProtocolState::Failed(e.to_string())).await;
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        client.set_state(ProtocolState::Disconnected).await;
                        continue;
                    }
                }
                ProtocolState::Connecting => {
                    if let Err(e) = client.perform_handshake().await {
                        error!("Handshake failed: {}", e);
                        client.set_state(ProtocolState::Failed(e.to_string())).await;
                        continue;
                    }
                }
                ProtocolState::HandshakeCompleted => {
                    if let Err(e) = client.negotiate_eth_protocol().await {
                        error!("ETH protocol negotiation failed: {}", e);
                        client.set_state(ProtocolState::Failed(e.to_string())).await;
                        continue;
                    }
                }
                ProtocolState::EthProtocolNegotiated => {
                    client.set_state(ProtocolState::Connected).await;
                    info!("✅ DevP2P connection fully established!");
                }
                ProtocolState::Connected => {
                    // Main message handling loop
                    if let Err(e) = client.handle_messages(&announcement_sender).await {
                        error!("Message handling failed: {}", e);
                        client.set_state(ProtocolState::Failed(e.to_string())).await;
                        continue;
                    }
                }
                ProtocolState::Failed(ref reason) => {
                    warn!("Connection failed: {}. Retrying in 5 seconds...", reason);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    client.reset_connection().await;
                    continue;
                }
                _ => {
                    // Handle other intermediate states
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            }
            
            // Small delay to prevent busy loop
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    /// Set the protocol state
    pub async fn set_state(&self, new_state: ProtocolState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }
    
    /// Get the connection for protocol implementation
    pub fn connection(&self) -> &Arc<Mutex<Option<TcpStream>>> {
        &self.connection
    }
    
    /// Get the private key
    pub fn private_key(&self) -> &SecretKey {
        &self.private_key
    }
    
    /// Get the remote public key
    pub fn remote_pubkey(&self) -> &Arc<RwLock<Option<PublicKey>>> {
        &self.remote_pubkey
    }
    
    /// Get the request counter
    pub fn request_counter(&self) -> &Arc<RwLock<u64>> {
        &self.request_counter
    }
    
    /// Get the pending requests
    pub fn pending_requests(&self) -> &Arc<RwLock<HashMap<u64, Vec<[u8; 32]>>>> {
        &self.pending_requests
    }
    
    /// Reset connection for retry
    async fn reset_connection(&self) {
        let mut connection = self.connection.lock().await;
        *connection = None;
        self.set_state(ProtocolState::Disconnected).await;
    }
    
    /// Fetch new transactions from the DevP2P connection
    pub async fn fetch_new_transactions(&self) -> Result<Vec<TransactionView>> {
        let start_time = Instant::now();
        
        // Check if we're connected
        let state = self.state.read().await;
        if *state != ProtocolState::Connected {
            return Ok(Vec::new()); // Not connected yet
        }
        drop(state);
        
        // Try to receive transaction announcements with very low timeout for real-time
        let announcement_timeout = Duration::from_millis(1); // 1ms for ultra-fast detection
        
        let mut receiver = self.tx_announcements.write().await;
        let announcements = timeout(announcement_timeout, receiver.recv()).await;
        drop(receiver);
        
        match announcements {
            Ok(Some(TxPoolMessage::NewPooledTransactionHashes { hashes, .. })) => {
                let latency = start_time.elapsed();
                
                info!("⚡ Received {} transaction hashes via DevP2P in {:.2}ms", 
                      hashes.len(), latency.as_millis());
                
                // Update performance metrics
                self.update_performance_metrics(hashes.len(), latency).await;
                
                // For now, return empty - in full implementation, we'd return 
                // the TransactionView objects after requesting full data
                Ok(Vec::new())
            }
            Ok(Some(_)) => {
                debug!("📥 Received other message type");
                Ok(Vec::new())
            }
            Ok(None) => {
                // Channel closed
                warn!("Transaction announcement channel closed");
                Ok(Vec::new())
            }
            Err(_) => {
                // Timeout is normal - no new transactions
                Ok(Vec::new())
            }
        }
    }
    
    
    /// Get performance statistics for the DevP2P connection
    pub async fn get_performance_stats(&self) -> (u64, f64, Duration) {
        let received = *self.transactions_received.read().await;
        let avg_latency_ms = *self.average_latency_ms.read().await;
        let uptime = self.connection_start_time.elapsed();
        
        (received, avg_latency_ms, uptime)
    }
    
    /// Check if the DevP2P connection is healthy
    pub async fn is_connected(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, ProtocolState::Connected)
    }

    /// Update performance metrics (method called from protocol implementation)
    pub async fn update_performance_metrics(&self, tx_count: usize, latency: std::time::Duration) {
        if tx_count > 0 {
            let mut received = self.transactions_received.write().await;
            *received += tx_count as u64;
            
            let mut avg_latency = self.average_latency_ms.write().await;
            let new_latency_ms = latency.as_millis() as f64;
            *avg_latency = (*avg_latency * 0.9) + (new_latency_ms * 0.1); // Exponential moving average
        }
    }
    
}

/// Create a DevP2P client for the local Ethereum node
pub async fn create_local_devp2p_client() -> Result<DevP2pClient> {
    // Connect to local node's DevP2P interface
    // Most Ethereum clients expose DevP2P on port 30303
    DevP2pClient::new("127.0.0.1:30303").await
}

/// Performance comparison utilities
pub mod performance {
    use super::*;
    
    /// Compare DevP2P vs RPC performance
    pub fn log_performance_comparison(devp2p_latency_ms: f64, rpc_latency_ms: f64) {
        let improvement = (rpc_latency_ms - devp2p_latency_ms) / rpc_latency_ms * 100.0;
        
        info!("📊 PERFORMANCE COMPARISON:");
        info!("   🔗 DevP2P:  {:.2}ms transaction arrival", devp2p_latency_ms);
        info!("   📡 RPC:     {:.2}ms transaction arrival", rpc_latency_ms);
        info!("   🚀 Improvement: {:.1}% faster ({:.1}x speedup)", 
              improvement, rpc_latency_ms / devp2p_latency_ms);
        
        if devp2p_latency_ms < 10.0 {
            info!("✅ DevP2P achieving target <10ms latency!");
        } else {
            warn!("⚠️  DevP2P latency above 10ms target");
        }
    }
} 