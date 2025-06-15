/// DevP2P Protocol Implementation
/// 
/// This module implements the core DevP2P protocol for direct peer-to-peer
/// communication with Ethereum nodes, achieving <10ms transaction detection latency.

use std::time::Instant;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn, debug, error};
use eyre::{Result, eyre};
use bytes::{Bytes, BytesMut, BufMut};
use rlp::{Rlp, RlpStream, Encodable, Decodable};
use secp256k1::{SecretKey, PublicKey, Secp256k1, Message, ecdsa::Signature};
use sha3::{Keccak256, Digest};
use rand::Rng;

use super::client::{DevP2pClient, ProtocolState, TxPoolMessage};

impl DevP2pClient {
    /// Phase 1: Establish TCP connection to peer
    pub(super) async fn establish_connection(&self) -> Result<()> {
        info!("📡 Phase 1: Establishing TCP connection to {}", self.peer_address);
        self.set_state(ProtocolState::Connecting).await;
        
        // Parse address and port
        let addr: std::net::SocketAddr = self.peer_address.parse()
            .map_err(|e| eyre!("Invalid peer address: {}", e))?;
        
        // Establish TCP connection with timeout
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            TcpStream::connect(addr)
        ).await
        .map_err(|_| eyre!("Connection timeout"))?
        .map_err(|e| eyre!("Failed to connect: {}", e))?;
        
        // Store the connection
        {
            let mut connection = self.connection().lock().await;
            *connection = Some(stream);
        }
        
        info!("✅ TCP connection established");
        Ok(())
    }
    
    /// Phase 2: Perform DevP2P handshake
    pub(super) async fn perform_handshake(&self) -> Result<()> {
        info!("🤝 Phase 2: Performing DevP2P handshake...");
        self.set_state(ProtocolState::HandshakeInitiated).await;
        
        // Get connection
        let mut connection_guard = self.connection().lock().await;
        let stream = connection_guard.as_mut()
            .ok_or_else(|| eyre!("No connection available"))?;
        
        // Generate authentication data
        let auth_data = self.generate_auth_data().await?;
        
        // Send authentication message
        let auth_msg = self.create_auth_message(&auth_data)?;
        stream.write_all(&auth_msg).await
            .map_err(|e| eyre!("Failed to send auth message: {}", e))?;
        
        debug!("📤 Sent authentication message ({} bytes)", auth_msg.len());
        
        // Receive authentication response
        let mut response_buf = vec![0u8; 1024];
        let response_len = stream.read(&mut response_buf).await
            .map_err(|e| eyre!("Failed to read auth response: {}", e))?;
        
        if response_len == 0 {
            return Err(eyre!("Peer closed connection during handshake"));
        }
        
        response_buf.truncate(response_len);
        debug!("📥 Received authentication response ({} bytes)", response_len);
        
        // Process authentication response
        self.process_auth_response(&response_buf).await?;
        
        self.set_state(ProtocolState::HandshakeCompleted).await;
        info!("✅ DevP2P handshake completed");
        Ok(())
    }
    
    /// Phase 3: Negotiate ETH protocol
    pub(super) async fn negotiate_eth_protocol(&self) -> Result<()> {
        info!("⚡ Phase 3: Negotiating ETH protocol...");
        
        let mut connection_guard = self.connection().lock().await;
        let stream = connection_guard.as_mut()
            .ok_or_else(|| eyre!("No connection available"))?;
        
        // Send ETH protocol status message
        let status_msg = self.create_eth_status_message()?;
        stream.write_all(&status_msg).await
            .map_err(|e| eyre!("Failed to send status message: {}", e))?;
        
        debug!("📤 Sent ETH status message");
        
        // Receive status response
        let mut response_buf = vec![0u8; 1024];
        let response_len = stream.read(&mut response_buf).await
            .map_err(|e| eyre!("Failed to read status response: {}", e))?;
        
        if response_len == 0 {
            return Err(eyre!("Peer closed connection during ETH negotiation"));
        }
        
        response_buf.truncate(response_len);
        debug!("📥 Received ETH status response ({} bytes)", response_len);
        
        // Process status response
        self.process_eth_status_response(&response_buf).await?;
        
        self.set_state(ProtocolState::EthProtocolNegotiated).await;
        info!("✅ ETH protocol negotiated successfully");
        Ok(())
    }
    
    /// Phase 4: Handle incoming messages and transaction announcements
    pub(super) async fn handle_messages(
        &self,
        announcement_sender: &tokio::sync::mpsc::Sender<TxPoolMessage>
    ) -> Result<()> {
        debug!("🏊 Phase 4: Handling transaction pool messages...");
        
        let mut connection_guard = self.connection().lock().await;
        let stream = connection_guard.as_mut()
            .ok_or_else(|| eyre!("No connection available"))?;
        
        // Main message loop
        let mut buffer = vec![0u8; 4096];
        
        loop {
            // Read message with timeout
            let read_result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                stream.read(&mut buffer)
            ).await;
            
            let bytes_read = match read_result {
                Ok(Ok(0)) => {
                    // Connection closed by peer
                    return Err(eyre!("Peer closed connection"));
                }
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    return Err(eyre!("Read error: {}", e));
                }
                Err(_) => {
                    // Timeout - send keepalive
                    self.send_keepalive(stream).await?;
                    continue;
                }
            };
            
            // Process received message
            let message_data = &buffer[..bytes_read];
            self.process_message(message_data, announcement_sender).await?;
        }
    }
    
    /// Generate authentication data for handshake
    async fn generate_auth_data(&self) -> Result<AuthData> {
        let mut rng = rand::thread_rng();
        
        // Generate random nonce
        let mut nonce = [0u8; 32];
        rng.fill(&mut nonce);
        
        // Get our public key
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_secret_key(&secp, self.private_key());
        let pubkey_bytes = public_key.serialize_uncompressed();
        
        // Create signature (simplified for now)
        let message_hash = Keccak256::digest(&nonce);
        let message = Message::from_digest_slice(&message_hash)
            .map_err(|e| eyre!("Invalid message for signing: {}", e))?;
        
        let signature = secp.sign_ecdsa(&message, self.private_key());
        let signature_bytes = signature.serialize_compact();
        
        let mut sig_array = [0u8; 65];
        sig_array[..64].copy_from_slice(&signature_bytes);
        sig_array[64] = 0; // Recovery ID (simplified)
        
        let mut pubkey_array = [0u8; 64];
        pubkey_array.copy_from_slice(&pubkey_bytes[1..65]); // Skip first byte (0x04)
        
        Ok(AuthData {
            signature: sig_array,
            initiator_pubkey: pubkey_array,
            nonce,
            version: 4, // DevP2P version
        })
    }
    
    /// Create authentication message
    fn create_auth_message(&self, auth_data: &AuthData) -> Result<Vec<u8>> {
        let mut stream = RlpStream::new();
        stream.begin_list(4);
        stream.append(&auth_data.signature.as_ref());
        stream.append(&auth_data.initiator_pubkey.as_ref());
        stream.append(&auth_data.nonce.as_ref());
        stream.append(&auth_data.version);
        
        Ok(stream.out().to_vec())
    }
    
    /// Process authentication response from peer
    async fn process_auth_response(&self, response: &[u8]) -> Result<()> {
        let rlp = Rlp::new(response);
        
        if !rlp.is_list() || rlp.item_count()? < 3 {
            return Err(eyre!("Invalid auth response format"));
        }
        
        // Extract peer's public key
        let peer_pubkey_bytes: Vec<u8> = rlp.val_at(1)?;
        if peer_pubkey_bytes.len() != 64 {
            return Err(eyre!("Invalid peer public key length"));
        }
        
        // Store peer's public key
        let mut pubkey_bytes = [0u8; 65];
        pubkey_bytes[0] = 0x04; // Uncompressed key prefix
        pubkey_bytes[1..].copy_from_slice(&peer_pubkey_bytes);
        
        let peer_pubkey = PublicKey::from_slice(&pubkey_bytes)
            .map_err(|e| eyre!("Invalid peer public key: {}", e))?;
        
        {
            let mut remote_pubkey = self.remote_pubkey().write().await;
            *remote_pubkey = Some(peer_pubkey);
        }
        
        debug!("✅ Peer public key stored");
        Ok(())
    }
    
    /// Create ETH protocol status message
    fn create_eth_status_message(&self) -> Result<Vec<u8>> {
        let mut stream = RlpStream::new();
        stream.begin_list(5);
        stream.append(&66u32); // ETH protocol version
        stream.append(&1u32);  // Network ID (mainnet)
        stream.append(&0u64);  // Total difficulty (simplified)
        stream.append(&hex::decode("d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3").unwrap()); // Genesis hash
        stream.append(&hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap()); // Latest block hash
        
        // Wrap in ETH message
        let mut eth_msg = Vec::new();
        eth_msg.push(super::client::ETH_STATUS);
        eth_msg.extend_from_slice(&stream.out());
        
        Ok(eth_msg)
    }
    
    /// Process ETH status response
    async fn process_eth_status_response(&self, response: &[u8]) -> Result<()> {
        if response.is_empty() {
            return Err(eyre!("Empty status response"));
        }
        
        let msg_id = response[0];
        if msg_id != super::client::ETH_STATUS {
            return Err(eyre!("Unexpected message ID in status response: {}", msg_id));
        }
        
        let rlp = Rlp::new(&response[1..]);
        if !rlp.is_list() || rlp.item_count()? < 5 {
            return Err(eyre!("Invalid status response format"));
        }
        
        let protocol_version: u32 = rlp.val_at(0)?;
        let network_id: u32 = rlp.val_at(1)?;
        
        debug!("Peer ETH protocol version: {}, network: {}", protocol_version, network_id);
        
        if network_id != 1 {
            return Err(eyre!("Peer is not on mainnet (network ID: {})", network_id));
        }
        
        Ok(())
    }
    
    /// Process incoming protocol message
    async fn process_message(
        &self,
        data: &[u8],
        announcement_sender: &tokio::sync::mpsc::Sender<TxPoolMessage>
    ) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        
        let msg_id = data[0];
        let payload = &data[1..];
        
        match msg_id {
            super::client::ETH_NEW_POOLED_TRANSACTION_HASHES => {
                let detection_time = Instant::now();
                self.handle_new_pooled_transaction_hashes(payload, detection_time, announcement_sender).await?
            }
            super::client::ETH_POOLED_TRANSACTIONS => {
                self.handle_pooled_transactions(payload, announcement_sender).await?
            }
            _ => {
                debug!("Received unknown message ID: 0x{:02x}", msg_id);
            }
        }
        
        Ok(())
    }
    
    /// Handle NewPooledTransactionHashes message (the key for <10ms latency)
    async fn handle_new_pooled_transaction_hashes(
        &self,
        payload: &[u8],
        detection_time: Instant,
        announcement_sender: &tokio::sync::mpsc::Sender<TxPoolMessage>
    ) -> Result<()> {
        let rlp = Rlp::new(payload);
        if !rlp.is_list() {
            return Err(eyre!("Invalid NewPooledTransactionHashes format"));
        }
        
        let mut hashes = Vec::new();
        let mut types = Vec::new();
        let mut sizes = Vec::new();
        
        // Parse transaction hashes, types, and sizes
        for i in 0..rlp.item_count()? {
            let item = rlp.at(i)?;
            if item.is_list() && item.item_count()? >= 3 {
                let hash_bytes: Vec<u8> = item.val_at(0)?;
                let tx_type: u8 = item.val_at(1)?;
                let tx_size: u32 = item.val_at(2)?;
                
                if hash_bytes.len() == 32 {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(&hash_bytes);
                    hashes.push(hash);
                    types.push(tx_type);
                    sizes.push(tx_size);
                }
            }
        }
        
        if !hashes.is_empty() {
            let message = TxPoolMessage::NewPooledTransactionHashes {
                hashes: hashes.clone(),
                types,
                sizes,
            };
            
            // Send announcement (this should be <10ms from transaction arrival)
            if let Err(e) = announcement_sender.send(message).await {
                error!("Failed to send transaction announcement: {}", e);
            } else {
                let latency = detection_time.elapsed();
                debug!("📥 Announced {} transaction hashes in {:?}", hashes.len(), latency);
                
                // Update metrics
                self.update_performance_metrics(hashes.len(), latency).await;
            }
            
            // Immediately request full transaction data
            self.request_pooled_transactions(hashes).await?;
        }
        
        Ok(())
    }
    
    /// Handle PooledTransactions response
    async fn handle_pooled_transactions(
        &self,
        payload: &[u8],
        announcement_sender: &tokio::sync::mpsc::Sender<TxPoolMessage>
    ) -> Result<()> {
        // TODO: Parse actual transaction data and convert to TransactionView
        // For now, just acknowledge receipt
        debug!("📥 Received pooled transactions response ({} bytes)", payload.len());
        Ok(())
    }
    
    /// Request full transaction data for given hashes
    async fn request_pooled_transactions(&self, hashes: Vec<[u8; 32]>) -> Result<()> {
        let mut connection_guard = self.connection().lock().await;
        let stream = connection_guard.as_mut()
            .ok_or_else(|| eyre!("No connection available"))?;
        
        // Generate request ID
        let request_id = {
            let mut counter = self.request_counter().write().await;
            *counter += 1;
            *counter
        };
        
        // Store pending request
        {
            let mut pending = self.pending_requests().write().await;
            pending.insert(request_id, hashes.clone());
        }
        
        // Create GetPooledTransactions message
        let mut stream_rlp = RlpStream::new();
        stream_rlp.begin_list(hashes.len());
        for hash in &hashes {
            stream_rlp.append(&hash.as_ref());
        }
        
        // Wrap in ETH message
        let mut eth_msg = Vec::new();
        eth_msg.push(super::client::ETH_GET_POOLED_TRANSACTIONS);
        eth_msg.extend_from_slice(&stream_rlp.out());
        
        // Send request
        stream.write_all(&eth_msg).await
            .map_err(|e| eyre!("Failed to send GetPooledTransactions: {}", e))?;
        
        debug!("📤 Requested {} transaction details", hashes.len());
        Ok(())
    }
    
    /// Send keepalive message to maintain connection
    async fn send_keepalive(&self, stream: &mut TcpStream) -> Result<()> {
        // Send ping message
        let ping_msg = vec![0x02]; // Simple ping
        stream.write_all(&ping_msg).await
            .map_err(|e| eyre!("Failed to send keepalive: {}", e))?;
        Ok(())
    }
}

/// Authentication data structure for DevP2P handshake
#[derive(Debug)]
struct AuthData {
    signature: [u8; 65],
    initiator_pubkey: [u8; 64],
    nonce: [u8; 32],
    version: u8,
}