use clap::Parser;
use ethers::prelude::*;
use eyre::Result;
use std::{sync::Arc, time::Duration};
use tokio_stream::StreamExt;
use tracing::{info, warn, debug, trace, error};
use zmq;
use hex::encode as hex_encode;
use flatbuffers::FlatBufferBuilder;
use std::sync::atomic::{AtomicUsize, Ordering};
use serde_json::Value;
use std::collections::{HashSet, HashMap};
use std::time::Instant;

// Import our ERC20 and DEX decoding utilities
use mempool_fetcher::common::erc20::{decode_erc20_method, ERC20Method, format_token_amount};
use mempool_fetcher::common::dex::{detect_dex_interaction, DexInteraction, is_dex_router};

#[derive(Parser, Debug)]
struct Args {
    /// WebSocket endpoint for real-time subscriptions
    #[arg(long, env = "WS_RPC_URL", default_value = "ws://localhost:8546")]
    ws_rpc_url: String,
    /// HTTP endpoint for detailed transaction fetching
    #[arg(long, env = "HTTP_RPC_URL", default_value = "http://localhost:8545")]
    http_rpc_url: String,
    /// Value threshold in wei (1 ETH default)
    #[arg(long, env = "THRESHOLD_WEI", default_value = "1000000000000000000")]
    threshold: U256,
    /// Fallback poll interval for mempool in milliseconds (used if WebSocket fails)
    #[arg(long, env = "POLL_INTERVAL_MS", default_value = "500")]
    poll_interval_ms: u64,
    /// Token addresses to watch (comma-separated)
    #[arg(long, env = "WATCHED_TOKENS", default_value = "")]
    watched_tokens: String,
    /// Verbose logging mode
    #[arg(short, long)]
    verbose: bool,
}

// Create a byte buffer for a FlatBuffer Alert with fixed-size fields
// This is a simplified approach to avoid compatibility issues
fn create_alert_buffer(hash: &[u8], value: u64, from: &[u8], to: &[u8]) -> Vec<u8> {
    let mut builder = FlatBufferBuilder::new();
    
    // Create vectors for byte arrays
    let hash_vec = builder.create_vector(hash);
    let from_vec = builder.create_vector(from);
    let to_vec = builder.create_vector(to);
    
    // Root table offsets
    let start = builder.start_table();
    builder.push_slot_always(0, hash_vec);  // hash at index 0
    builder.push_slot(1, value, 0);         // value at index 1
    builder.push_slot_always(2, from_vec);  // from at index 2
    builder.push_slot_always(3, to_vec);    // to at index 3
    let root = builder.end_table(start);
    
    // Finish and get buffer
    builder.finish(root, None);
    builder.finished_data().to_vec()
}

// Get current mempool status
async fn get_mempool_stats(provider: &Provider<Http>) -> Result<(usize, usize)> {
    debug!("Requesting txpool_status...");
    // Use the TxpoolStatus method to get quick stats
    let txpool: TxpoolStatus = provider.request("txpool_status", ()).await?;
    
    // Extract pending and queued counts
    let pending_count = txpool.pending.as_u64() as usize;
    let queued_count = txpool.queued.as_u64() as usize;
    
    Ok((pending_count, queued_count))
}

/// Get a batch of transactions from txpool_content API
/// Only used as a fallback if websocket subscription fails
async fn get_mempool_transactions(provider: &Provider<Http>) -> Result<HashMap<H256, TransactionView>> {
    debug!("Requesting txpool_content...");
    let start = Instant::now();
    let response: Value = provider.request("txpool_content", ()).await?;
    let request_time = start.elapsed();
    debug!("txpool_content response received in {:?}", request_time);
    
    let mut transactions = HashMap::new();

    // Process pending transactions
    let pending_start = Instant::now();
    if let Some(pending) = response.get("pending").and_then(|p| p.as_object()) {
        debug!("Processing {} pending accounts", pending.len());
        let mut total_pending = 0;
        
        for (_address, nonce_map) in pending {
            if let Some(nonce_obj) = nonce_map.as_object() {
                total_pending += nonce_obj.len();
                for (_, tx_value) in nonce_obj {
                    if let Some(tx) = parse_transaction(tx_value) {
                        let hash_bytes = &tx.hash;
                        if hash_bytes.len() == 32 {
                            let mut hash_array = [0u8; 32];
                            hash_array.copy_from_slice(hash_bytes);
                            let hash = H256::from(hash_array);
                            transactions.insert(hash, tx);
                        }
                    }
                }
            }
        }
        debug!("Processed {} total pending transactions in {:?}", total_pending, pending_start.elapsed());
    }

    // Process queued transactions
    let queued_start = Instant::now();
    if let Some(queued) = response.get("queued").and_then(|q| q.as_object()) {
        debug!("Processing {} queued accounts", queued.len());
        let mut total_queued = 0;
        
        for (_address, nonce_map) in queued {
            if let Some(nonce_obj) = nonce_map.as_object() {
                total_queued += nonce_obj.len();
                for (_, tx_value) in nonce_obj {
                    if let Some(tx) = parse_transaction(tx_value) {
                        let hash_bytes = &tx.hash;
                        if hash_bytes.len() == 32 {
                            let mut hash_array = [0u8; 32];
                            hash_array.copy_from_slice(hash_bytes);
                            let hash = H256::from(hash_array);
                            transactions.insert(hash, tx);
                        }
                    }
                }
            }
        }
        debug!("Processed {} total queued transactions in {:?}", total_queued, queued_start.elapsed());
    }

    debug!("Completed txpool_content processing in {:?}, extracted {} transactions", start.elapsed(), transactions.len());
    Ok(transactions)
}

/// Helper to parse a transaction from JSON format
fn parse_transaction(tx_json: &Value) -> Option<TransactionView> {
    let hash = tx_json.get("hash")?.as_str()?;
    let from = tx_json.get("from")?.as_str()?;
    let to = tx_json.get("to").and_then(|v| v.as_str());
    let value_str = tx_json.get("value")?.as_str()?;
    let input_str = tx_json.get("input").and_then(|v| v.as_str());
    
    // Parse hash (removing 0x prefix)
    let hash = hash.strip_prefix("0x").unwrap_or(hash);
    let hash_bytes = hex::decode(hash).ok()?;
    
    // Parse addresses (removing 0x prefix)
    let from = from.strip_prefix("0x").unwrap_or(from);
    let from_bytes = hex::decode(from).ok()?;
    
    let to_bytes = if let Some(to) = to {
        let to = to.strip_prefix("0x").unwrap_or(to);
        Some(hex::decode(to).ok()?)
    } else {
        None
    };
    
    // Parse value
    let value_str = value_str.strip_prefix("0x").unwrap_or(value_str);
    let value = U256::from_str_radix(value_str, 16).ok()?;

    // Parse input data
    let input_data_bytes = if let Some(input) = input_str {
        let input = input.strip_prefix("0x").unwrap_or(input);
        if input == "" || input == "0x" {
            None
        } else {
            hex::decode(input).ok()
        }
    } else {
        None
    };
    
    Some(TransactionView {
        hash: hash_bytes,
        from: from_bytes,
        to: to_bytes,
        value,
        input_data: input_data_bytes,
    })
}

/// Simple struct to hold transaction data
#[derive(Clone)]
struct TransactionView {
    hash: Vec<u8>,
    from: Vec<u8>,
    to: Option<Vec<u8>>,
    value: U256,
    input_data: Option<Vec<u8>>,
}

/// Process a transaction and publish it if it meets criteria
fn process_transaction(
    tx: &TransactionView,
    threshold: U256,
    zmq_pub: &zmq::Socket,
    processed_count: &Arc<AtomicUsize>,
    seen_hashes: &mut HashSet<String>,
    watched_tokens: &HashSet<String>,
) -> bool {
    // Skip if no destination
    if tx.to.is_none() {
        trace!("Skipping transaction with no destination");
        return false;
    }
    
    let to_bytes = tx.to.as_ref().unwrap();
    let to_hex = hex_encode(&to_bytes);
    let hash_hex = hex_encode(&tx.hash);
    let from_hex = hex_encode(&tx.from);
    
    // Skip if already seen
    if seen_hashes.contains(&hash_hex) {
        trace!("Skipping already processed transaction: {}", hash_hex);
        return false;
    }
    
    // Check if this is a watched token transaction or DEX interaction
    let to_address_checksummed = format!("0x{}", to_hex);
    let is_token_tx = watched_tokens.contains(&to_address_checksummed.to_lowercase()) || 
                      watched_tokens.contains(&to_address_checksummed);
    
    // Check for DEX interactions that might affect watched tokens
    let is_dex_tx = is_dex_router(&to_address_checksummed) && !watched_tokens.is_empty();
    
    if is_token_tx {
        debug!("Detected transaction to watched token: {}", to_address_checksummed);
        
        // Decode ERC20 method if input data is available
        if let Some(input_data) = &tx.input_data {
            if let Some(method) = decode_erc20_method(input_data) {
                match method {
                    ERC20Method::Transfer { to, amount } => {
                        info!("🪙 TOKEN TRANSFER detected in tx {}", hash_hex);
                        info!("  Token: {}", to_address_checksummed);
                        info!("  From: 0x{}", from_hex);
                        info!("  To: {:?}", to);
                        info!("  Amount: {} (raw: {})", format_token_amount(amount, 18), amount);
                    },
                    ERC20Method::Approve { spender, amount } => {
                        info!("✅ TOKEN APPROVAL detected in tx {}", hash_hex);
                        info!("  Token: {}", to_address_checksummed);
                        info!("  Owner: 0x{}", from_hex);
                        info!("  Spender: {:?}", spender);
                        info!("  Amount: {} (raw: {})", format_token_amount(amount, 18), amount);
                    },
                    ERC20Method::TransferFrom { from, to, amount } => {
                        info!("🔄 TOKEN TRANSFER FROM detected in tx {}", hash_hex);
                        info!("  Token: {}", to_address_checksummed);
                        info!("  From: {:?}", from);
                        info!("  To: {:?}", to);
                        info!("  Amount: {} (raw: {})", format_token_amount(amount, 18), amount);
                    },
                    _ => {
                        debug!("Other ERC20 method detected: {:?}", method);
                    }
                }
            } else {
                debug!("Unknown method called on token contract");
            }
        }
        
        // Mark as seen and processed
        seen_hashes.insert(hash_hex.clone());
        
        // Increment processed count
        let _current = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
        
        // For token transactions, always send alert regardless of ETH value
        let value_u64 = tx.value.min(U256::from(u64::MAX)).as_u64();
        let buf = create_alert_buffer(
            &tx.hash,
            value_u64,
            &tx.from,
            &to_bytes
        );
        
        if let Err(e) = zmq_pub.send(&buf, 0) {
            error!("Failed to send token transaction alert: {}", e);
            return false;
        }
        
        return true;
    }
    
    // Check for DEX interactions
    if is_dex_tx {
        if let Some(input_data) = &tx.input_data {
            if let Some(dex_interaction) = detect_dex_interaction(&to_address_checksummed, input_data) {
                match &dex_interaction {
                    DexInteraction::RouterSwap { router_address, swap_type, .. } => {
                        info!("🔄 DEX SWAP detected in tx {}", hash_hex);
                        info!("  Router: {:?}", router_address);
                        info!("  Type: {:?}", swap_type);
                        info!("  From: 0x{}", from_hex);
                        info!("  Note: May affect watched tokens");
                    },
                    DexInteraction::PoolInteraction { pool_address, interaction_type } => {
                        info!("💱 POOL INTERACTION detected in tx {}", hash_hex);
                        info!("  Pool: {:?}", pool_address);
                        info!("  Type: {:?}", interaction_type);
                        info!("  From: 0x{}", from_hex);
                    },
                    _ => {}
                }
                
                // Mark as seen and send alert for DEX interactions
                seen_hashes.insert(hash_hex.clone());
                let _current = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
                
                let value_u64 = tx.value.min(U256::from(u64::MAX)).as_u64();
                let buf = create_alert_buffer(
                    &tx.hash,
                    value_u64,
                    &tx.from,
                    &to_bytes
                );
                
                if let Err(e) = zmq_pub.send(&buf, 0) {
                    error!("Failed to send DEX transaction alert: {}", e);
                    return false;
                }
                
                return true;
            }
        }
    }
    
    // Original logic for non-token transactions
    // Check value threshold
    if tx.value < threshold {
        trace!("Skipping transaction below threshold: {}", hash_hex);
        return false;
    }
    
    debug!("Processing high-value transaction: {} ({} wei)", hash_hex, tx.value);
    
    // Handle potential integer overflow for large values safely
    let value_u64 = tx.value.min(U256::from(u64::MAX)).as_u64();
    
    // Create FlatBuffer alert
    let buf_start = Instant::now();
    let buf = create_alert_buffer(
        &tx.hash,
        value_u64,
        &tx.from,
        &to_bytes
    );
    let buf_time = buf_start.elapsed();
    trace!("FlatBuffer creation took {:?}", buf_time);
    
    // Send the binary data
    let send_start = Instant::now();
    let result = zmq_pub.send(&buf, 0);
    let send_time = send_start.elapsed();
    
    if let Err(e) = result {
        error!("Failed to send transaction alert: {}", e);
        return false;
    }
    
    trace!("ZMQ send took {:?}", send_time);
    
    // Mark as seen
    seen_hashes.insert(hash_hex.clone());
    
    // Increment and log transaction count
    let current = processed_count.fetch_add(1, Ordering::SeqCst) + 1;
    
    // Always log individual high-value transactions when verbose
    info!("ALERT #{}: tx {} from {} to {}, value: {} wei", 
         current, 
         hash_hex,
         hex_encode(&tx.from), 
         hex_encode(&to_bytes), 
         tx.value);
    
    if current % 10 == 0 {
        info!("Processed {} transactions, latest: {} from {} to {}, value: {}", 
             current, 
             hash_hex,
             hex_encode(&tx.from), 
             hex_encode(&to_bytes), 
             tx.value);
    } else if current == 1 {
        // Always log the first transaction
        info!("First transaction processed: {} from {} to {}, value: {}", 
             hash_hex,
             hex_encode(&tx.from), 
             hex_encode(&to_bytes), 
             tx.value);
    }
    
    true
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Configure tracing based on verbosity
    let env_filter = if args.verbose {
        "mempool_processor=debug,info"
    } else {
        "mempool_processor=info"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(env_filter))
        .init();

    // Parse watched tokens
    let watched_tokens: HashSet<String> = if !args.watched_tokens.is_empty() {
        args.watched_tokens
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        HashSet::new()
    };

    info!("======== Ethereum Mempool Processor ========");
    info!("Version 0.1.0");
    info!("Starting with configuration:");
    info!("  WebSocket URL: {}", args.ws_rpc_url);
    info!("  HTTP URL: {}", args.http_rpc_url);
    info!("  Value threshold: {} wei", args.threshold);
    info!("  Poll interval: {}ms", args.poll_interval_ms);
    info!("  Watched tokens: {}", if watched_tokens.is_empty() { 
        "None".to_string() 
    } else { 
        watched_tokens.iter().cloned().collect::<Vec<_>>().join(", ") 
    });
    info!("  Verbose mode: {}", args.verbose);
    info!("============================================");
    
    // Set up HTTP provider for transaction details
    info!("Connecting to Ethereum HTTP node at {}", args.http_rpc_url);
    let http_provider = Provider::<Http>::try_from(args.http_rpc_url.clone())?;
    
    // Check basic node information
    let network_version = http_provider.get_net_version().await?;
    let block_number = http_provider.get_block_number().await?;
    let chain_id = http_provider.get_chainid().await?;
    info!("Connected to Ethereum network");
    info!("  Chain ID: {}", chain_id);
    info!("  Network ID: {}", network_version);
    info!("  Current block: {}", block_number);
    
    // Get mempool stats immediately
    info!("Checking mempool status...");
    match get_mempool_stats(&http_provider).await {
        Ok((pending, queued)) => {
            info!("Current mempool state: {} pending, {} queued transactions", pending, queued);
            if pending == 0 && queued == 0 {
                warn!("No transactions in mempool. If this persists, check if your node supports txpool methods.");
            }
        },
        Err(e) => {
            warn!("Could not get mempool stats: {}. This is normal if your node doesn't support txpool extensions.", e);
        }
    };
    
    // Setup WebSocket provider for real-time subscriptions
    info!("Connecting to WebSocket endpoint at {}", args.ws_rpc_url);
    let ws = Ws::connect(&args.ws_rpc_url).await?;
    let ws_provider = Provider::new(ws);
    
    info!("Watching mempool with value threshold of {} wei", args.threshold);
    
    // Set up ZMQ publisher
    info!("Setting up ZeroMQ publisher...");
    let zmq_ctx = zmq::Context::new();
    let zmq_pub = zmq_ctx.socket(zmq::PUB).unwrap();
    zmq_pub.bind("ipc:///tmp/mempool_feed").unwrap();
    info!("ZeroMQ publisher bound to ipc:///tmp/mempool_feed");
    debug!("ZeroMQ publisher ready to send alerts");

    // Counter for transaction processing
    let processed_count = Arc::new(AtomicUsize::new(0));
    
    // Store seen transaction hashes to avoid duplicates
    let mut seen_hashes = HashSet::new();
    
    // Try to use WebSocket subscription for real-time updates first
    info!("Attempting to subscribe to new pending transactions via WebSocket...");
    
    match ws_provider.subscribe_pending_txs().await {
        Ok(mut stream) => {
            info!("Successfully subscribed to pending transactions! Listening for new transactions...");
            
            // Setup interval for stats logging
            let stats_interval = Duration::from_secs(15);
            let mut stats_timer = tokio::time::interval(stats_interval);
            
            // Setup interval for pruning seen hashes
            let prune_interval = Duration::from_secs(30);
            let mut prune_timer = tokio::time::interval(prune_interval);
            
            // Clone references for the transaction fetch task
            let http_provider_worker = http_provider.clone();
            let processed_count_clone = processed_count.clone();
            let threshold = args.threshold;
            let watched_tokens_clone = watched_tokens.clone();
            
            // Create ZMQ publisher for the worker thread using the same context
            let zmq_pub_inner = zmq_ctx.socket(zmq::PUB).unwrap();
            zmq_pub_inner.connect("ipc:///tmp/mempool_feed").unwrap();
            
            // Counter for received tx hashes
            let rx_count = Arc::new(AtomicUsize::new(0));
            
            // Map to store pending transactions we're waiting on
            let _pending_txs: HashMap<H256, bool> = HashMap::new();
            
            // Use a channel to communicate between subscription and transaction fetch
            let (tx_sender, mut tx_receiver) = tokio::sync::mpsc::channel::<H256>(1000);
            
            info!("Starting transaction processing worker...");
            
            // Spawn task to fetch transaction details
    tokio::spawn(async move {
                info!("Transaction processing worker started");
                let mut processed = 0;
                let mut skipped = 0;
                
                while let Some(tx_hash) = tx_receiver.recv().await {
                    match http_provider_worker.get_transaction(tx_hash).await {
                        Ok(Some(tx)) => {
                            // Convert to our format
                            let from = tx.from;
                            let hash_bytes = tx.hash.as_bytes().to_vec();
                            let from_bytes = from.as_bytes().to_vec();
                            let to_bytes = tx.to.map(|to| to.as_bytes().to_vec());
                            let input_data = Some(tx.input.to_vec());
                            
                            let tx_view = TransactionView {
                                hash: hash_bytes,
                                from: from_bytes,
                                to: to_bytes,
                                value: tx.value,
                                input_data,
                            };
                            
                            // Process transaction
                            let mut seen = HashSet::new();  // Local cache just for this thread
                            let success = process_transaction(
                                &tx_view, 
                                threshold,
                                &zmq_pub_inner,
                                &processed_count_clone,
                                &mut seen,
                                &watched_tokens_clone
                            );
                            
                            if success {
                                processed += 1;
                                debug!("Successfully processed transaction {}", tx_hash);
                            } else {
                                skipped += 1;
                            }
                            
                            // Log processing stats periodically
                            if (processed + skipped) % 100 == 0 {
                                debug!("Worker stats: processed={}, skipped={}, total={}", processed, skipped, processed + skipped);
                            }
                        },
                        Ok(None) => {
                            trace!("Transaction not found: {}", tx_hash);
                            skipped += 1;
                        },
                        Err(e) => {
                            // Only log every so often to avoid log spam
                            if (processed + skipped) % 100 == 0 {
                                warn!("Error fetching transaction {}: {}", tx_hash, e);
                            }
                            skipped += 1;
                        }
                    }
                }
                
                info!("Transaction processing worker stopped");
            });
            
            info!("Waiting for transactions...");
            
            // Main event loop
            loop {
                tokio::select! {
                    // Process new transaction hashes from subscription
                    tx_hash_opt = stream.next() => {
                        if let Some(tx_hash) = tx_hash_opt {
                            let count = rx_count.fetch_add(1, Ordering::SeqCst) + 1;
                            if count % 100 == 0 {
                                info!("Received {} transaction hashes", count);
                            }
                            trace!("Received transaction hash: {}", tx_hash);
                            
                            // Send the hash to the transaction fetcher task
                            let _ = tx_sender.send(tx_hash).await;
                        } else {
                            // Stream ended - reconnect
                            warn!("WebSocket subscription ended, will attempt to reconnect...");
                            break;
                        }
                    }
                    
                    // Periodically log stats
                    _ = stats_timer.tick() => {
                        let rx_total = rx_count.load(Ordering::SeqCst);
                        let processed_total = processed_count.load(Ordering::SeqCst);
                        
                        info!("STATUS: Received {} tx hashes, processed {} alerts", rx_total, processed_total);
                        
                        // Check mempool stats every minute
                        let http_provider_stats = http_provider.clone();
                        match get_mempool_stats(&http_provider_stats).await {
                            Ok((pending, queued)) => {
                                info!("Current mempool: {} pending, {} queued transactions", pending, queued);
                            },
                            Err(_) => { /* Silent on error */ }
                        };
                    }
                    
                    // Periodically prune seen hashes
                    _ = prune_timer.tick() => {
                        if seen_hashes.len() > 10_000 {
                            info!("Pruning seen transaction hash set (size: {})", seen_hashes.len());
                            let to_remove: Vec<_> = seen_hashes.iter()
                                .take(seen_hashes.len() - 5_000)
                                .cloned()
                                .collect();
                            
                            for hash in to_remove {
                                seen_hashes.remove(&hash);
                            }
                            
                            info!("Pruned seen hashes to {} entries", seen_hashes.len());
                        }
                    }
                }
            }
        },
        Err(e) => {
            warn!("Failed to subscribe to pending transactions: {}", e);
            warn!("Falling back to polling approach...");
            
            // Fallback to polling approach if subscription fails
            let poll_interval = Duration::from_millis(args.poll_interval_ms);
            info!("Starting mempool polling with interval of {}ms", args.poll_interval_ms);
            
            // Store txs we've seen to avoid duplicates
            let mut known_txs = HashMap::new();
            let start_time = Instant::now();
            let mut poll_count = 0;

            loop {
                poll_count += 1;
                
                // Log periodic stats
                if poll_count % 20 == 0 {
                    let runtime = start_time.elapsed();
                    let processed = processed_count.load(Ordering::SeqCst);
                    let tps = if runtime.as_secs() > 0 {
                        processed as f64 / runtime.as_secs() as f64
                    } else {
                        0.0
                    };
                    info!("STATUS: {} polls, {} known txs, {} processed, {:.2} tx/sec", 
                         poll_count, known_txs.len(), processed, tps);
                }
                
                let poll_start = Instant::now();
                match get_mempool_transactions(&http_provider).await {
                    Ok(txs) => {
                        info!("Fetched {} transactions from mempool in {:?}", txs.len(), poll_start.elapsed());
                        
                        // Find new transactions
                        let mut new_txs = Vec::new();
                        for (hash, tx) in txs.iter() {
                            if !known_txs.contains_key(hash) {
                                new_txs.push(tx.clone());
                                known_txs.insert(*hash, tx.clone());
                            }
                        }
                        
                        // Process new transactions
                        if !new_txs.is_empty() {
                            info!("Found {} new transactions", new_txs.len());
                            let mut processed = 0;
                            for tx in new_txs {
                                if process_transaction(
                                    &tx,
                                    args.threshold,
                                    &zmq_pub,
                                    &processed_count,
                                    &mut seen_hashes,
                                    &watched_tokens
                                ) {
                                    processed += 1;
                                }
                            }
                            info!("Successfully processed {} new transactions", processed);
                        } else if poll_count % 5 == 0 {
                            // Only log this every 5 polls to avoid spam
                            debug!("No new transactions in this poll");
                        }
                        
                        // Prune old seen hashes more efficiently with bounded memory
                        const MAX_SEEN_HASHES: usize = 10_000;
                        const PRUNE_TO_SIZE: usize = 5_000;
                        
                        if seen_hashes.len() > MAX_SEEN_HASHES {
                            info!("Pruning seen transaction hash set (size: {})", seen_hashes.len());
                            
                            // Create new set with most recent hashes only
                            let mut new_seen = HashSet::with_capacity(PRUNE_TO_SIZE);
                            let skip_count = seen_hashes.len() - PRUNE_TO_SIZE;
                            
                            for (idx, hash) in seen_hashes.iter().enumerate() {
                                if idx >= skip_count {
                                    new_seen.insert(hash.clone());
                                }
                            }
                            
                            seen_hashes = new_seen;
                            info!("Pruned seen hashes to {} entries", seen_hashes.len());
                        }
                        
                        // Prune known_txs more efficiently with bounded memory
                        const MAX_KNOWN_TXS: usize = 20_000;
                        const PRUNE_KNOWN_TO: usize = 10_000;
                        
                        if known_txs.len() > MAX_KNOWN_TXS {
                            info!("Pruning known transactions map (size: {})", known_txs.len());
                            
                            // Keep only the most recent transactions
                            let mut sorted_hashes: Vec<_> = known_txs.keys().copied().collect();
                            sorted_hashes.sort(); // Deterministic ordering
                            
                            let to_remove = sorted_hashes.len() - PRUNE_KNOWN_TO;
                            for hash in sorted_hashes.into_iter().take(to_remove) {
                                known_txs.remove(&hash);
                            }
                            
                            info!("Pruned known_txs to {} entries", known_txs.len());
                        }
                    },
                    Err(e) => {
                        error!("Error fetching mempool transactions: {}", e);
                    }
                }
                
                let poll_duration = poll_start.elapsed();
                debug!("Poll cycle completed in {:?}", poll_duration);
                
                // Adjust sleep time based on how long the poll took
                let sleep_time = if poll_duration < poll_interval {
                    poll_interval - poll_duration
                } else {
                    Duration::from_millis(10) // Minimum sleep to avoid CPU spinning
                };
                
                trace!("Sleeping for {:?} before next poll", sleep_time);
                tokio::time::sleep(sleep_time).await;
            }
        }
    }

    Ok(())
}