// signal_engine/function_detector.rs
//
// Simple function signature detector that categorizes transactions
// based on their function selectors (4-byte signatures)

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use tracing::{info, warn, error};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::Utc;
use zmq::{Context, Socket};
use serde::{Serialize, Deserialize};
use crate::token_tracking::TokenTrackingCache;
use crate::common::address::checksum_address;

lazy_static! {
    /// Log directory path - initialized once at startup
    static ref LOG_DIR: PathBuf = {
        let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let dir = PathBuf::from("/home/nima/code/crypto/logs/mempool")
            .join(format!("signal_detector_{}", timestamp));
        std::fs::create_dir_all(&dir).expect("Failed to create log directory");
        dir
    };
    
    /// Liquidity removal log file
    static ref LIQUIDITY_REMOVAL_LOG: Mutex<std::fs::File> = {
        let log_path = LOG_DIR.join("liquidity_removals.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open liquidity removal log file");
        
        Mutex::new(file)
    };
    
    /// Trading enabled log file
    static ref TRADING_ENABLED_LOG: Mutex<std::fs::File> = {
        let log_path = LOG_DIR.join("trading_enabled.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open trading enabled log file");
        
        Mutex::new(file)
    };
    
    
    
    /// Main signal detector log file
    static ref SIGNAL_DETECTOR_LOG: Mutex<std::fs::File> = {
        let log_path = LOG_DIR.join("signal_detector.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open signal detector log file");
        
        Mutex::new(file)
    };
    
    
    /// Global statistics
    static ref FUNCTION_STATS: Mutex<FunctionStats> = Mutex::new(FunctionStats::default());
    
    /// ZMQ Publisher for signals
    static ref ZMQ_PUBLISHER: Mutex<Option<Socket>> = {
        match Context::new().socket(zmq::PUB) {
            Ok(socket) => {
                // Set socket options
                let _ = socket.set_sndhwm(10000);
                let _ = socket.set_linger(0);
                
                // Bind to endpoint
                match socket.bind("tcp://127.0.0.1:5556") {
                    Ok(_) => {
                        info!("✅ ZMQ signal publisher bound to tcp://127.0.0.1:5556");
                        Mutex::new(Some(socket))
                    }
                    Err(e) => {
                        error!("Failed to bind ZMQ publisher: {}", e);
                        Mutex::new(None)
                    }
                }
            }
            Err(e) => {
                error!("Failed to create ZMQ socket: {}", e);
                Mutex::new(None)
            }
        }
    };
}

#[derive(Debug, Default)]
pub struct FunctionStats {
    pub total_checked: u64,
    pub liquidity_removals: u64,
    pub trading_enabled: u64,
    pub swaps: u64,
    pub other_functions: u64,
}

/// Signal alert for eth_kartal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalAlert {
    pub alert_type: String,
    pub function_name: String,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: String,
    pub value: String,
    pub gas_price: String,
    pub selector: String,
    pub timestamp: String,
    pub detection_latency_us: u64,
}



/// Function detector that categorizes transactions by their function signatures
pub struct FunctionDetector {
    liquidity_removal: LiquidityRemovalDetector,
    trading_enabled: TradingEnabledDetector,
    swap: SwapDetector,
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl FunctionDetector {
    pub fn new() -> Self {
        Self::new_with_cache(None)
    }
    
    pub fn new_with_cache(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        info!("🔍 Function detector initialized");
        info!("📁 Log directory: {}", LOG_DIR.display());
        
        // Log startup information to signal detector log
        if let Ok(mut log_file) = SIGNAL_DETECTOR_LOG.lock() {
            let _ = writeln!(log_file, "\n{} ==========================================", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🚀 Starting Mempool Signal Detection Service", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ⚡ Using non-blocking IPC for sub-millisecond latency", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🔍 Function detector initialized", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 📁 Log directory: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"), LOG_DIR.display());
            let _ = writeln!(log_file, "{} 🎯 Starting main processing loop...", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ==========================================\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = log_file.flush();
        }
        
        Self {
            liquidity_removal: LiquidityRemovalDetector::new(),
            trading_enabled: TradingEnabledDetector::new(),
            swap: SwapDetector::new(),
            token_cache,
        }
    }
    
    /// Check if transaction is a liquidity removal (simplified)
    pub fn is_liquidity_removal(&self, input_data: &[u8]) -> Option<&'static str> {
        if input_data.len() >= 4 {
            self.liquidity_removal.detect(&input_data[0..4])
        } else {
            None
        }
    }
    
    /// Detect function for a transaction and return the function name if interesting
    pub fn detect_function(&self, tx: &crate::mempool_fetcher::MempoolTransaction) -> Option<String> {
        if tx.input.len() < 4 {
            return None;
        }
        
        let selector = &tx.input[0..4];
        
        // Check liquidity removal
        if let Some(function_name) = self.liquidity_removal.detect(selector) {
            return Some(function_name.to_string());
        }
        
        // Check trading enabled
        if let Some(function_name) = self.trading_enabled.detect(selector) {
            return Some(function_name.to_string());
        }
        
        // Check swaps
        if let Some(function_name) = self.swap.detect(selector) {
            return Some(function_name.to_string());
        }
        
        
        None
    }
    
    
    /// Detect all function types in the transaction
    pub fn detect_from_ipc(&self, ipc_tx: &crate::mempool_fetcher::MempoolTransaction) {
        // Use pre-parsed fields directly
        if ipc_tx.input.is_empty() {
            return;
        }
        
        let tx_hash = &ipc_tx.hash;
        let from = format!("0x{}", hex::encode(&ipc_tx.from));
        let to = ipc_tx.to.as_ref()
            .map(|addr| format!("0x{}", hex::encode(addr)))
            .unwrap_or_else(|| "contract_creation".to_string());
        let value = format!("0x{:x}", ipc_tx.value);
        let gas_price = format!("0x{:x}", ipc_tx.gas_price.unwrap_or_default());
        
        self.detect_all(tx_hash, &from, &to, &value, &gas_price, &ipc_tx.input);
    }
    
    /// Process batch of transactions and return with function information
    pub fn detect_batch(&self, mut transactions: Vec<crate::mempool_fetcher::MempoolTransaction>) -> Vec<crate::mempool_fetcher::MempoolTransaction> {
        for tx in transactions.iter_mut() {
            let mut functions = Vec::new();
            
            // Skip if no input data
            if tx.input.len() < 4 {
                tx.functions = functions;
                continue;
            }
            
            let selector = &tx.input[0..4];
            
            // Check for approve function first - needs special handling for LP tokens
            if selector == &hex_to_bytes("095ea7b3") {
                // Check if this is an LP token approval
                let to_address = tx.to.as_ref()
                    .map(|addr| format!("0x{}", hex::encode(addr)))
                    .unwrap_or_else(|| "contract_creation".to_string());
                    
                let is_lp_approval = if let Some(ref cache) = self.token_cache {
                    let to_checksum = checksum_address(&to_address.trim_start_matches("0x"));
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            cache.pools.is_pool_address(&to_checksum).await
                        })
                    })
                } else {
                    false
                };
                
                if is_lp_approval {
                    functions.push("approve (LP Token)".to_string());
                }
            }
            
            // Check liquidity removal
            if let Some(function_name) = self.liquidity_removal.detect(selector) {
                functions.push(function_name.to_string());
            }
            
            // Check trading enabled
            if let Some(function_name) = self.trading_enabled.detect(selector) {
                functions.push(function_name.to_string());
            }
            
            // Check swaps
            if let Some(function_name) = self.swap.detect(selector) {
                functions.push(function_name.to_string());
            }
            
            
            // Still call the existing detection for logging and ZMQ publishing
            self.detect_from_ipc(&tx);
            
            tx.functions = functions;
        }
        
        transactions
    }
    
    /// Internal function to detect all function types with extracted details
    fn detect_all(&self, tx_hash: &str, from: &str, to: &str, value: &str, gas_price: &str, input_data: &[u8]) {
        if input_data.len() < 4 {
            return;
        }
        
        let selector_bytes = &input_data[0..4];
        let mut stats = match FUNCTION_STATS.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire function stats lock: {}", e);
                return;
            }
        };
        stats.total_checked += 1;
        
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        
        // Check liquidity removal first
        if let Some(function_name) = self.liquidity_removal.detect(selector_bytes) {
            stats.liquidity_removals += 1;
            info!("💧 LIQUIDITY REMOVAL: {} in tx {}", function_name, tx_hash);
            
            // Create signal alert
            let selector_hex = hex::encode(selector_bytes);
            let signal = SignalAlert {
                alert_type: "liquidity_removal".to_string(),
                function_name: function_name.to_string(),
                tx_hash: tx_hash.to_string(),
                from_address: from.to_string(),
                to_address: to.to_string(),
                value: value.to_string(),
                gas_price: gas_price.to_string(),
                selector: selector_hex.clone(),
                timestamp: timestamp.to_string(),
                detection_latency_us: 0, // Will be set by receiver
            };
            
            // Publish via ZMQ
            self.publish_signal(&signal);
            
            // Log to liquidity removal file with full transaction details
            if let Ok(mut log_file) = LIQUIDITY_REMOVAL_LOG.lock() {
                let _ = writeln!(log_file, 
                    "[{}] TX: {} | From: {} | To: {} | Value: {} | GasPrice: {} | Function: {} | Selector: {}", 
                    timestamp, tx_hash, from, to, value, gas_price, function_name, selector_hex
                );
                let _ = log_file.flush();
            }
            return;
        }
        
        // Check trading enabled
        if let Some(function_name) = self.trading_enabled.detect(selector_bytes) {
            stats.trading_enabled += 1;
            info!("🎯 TRADING ENABLED: {} in tx {}", function_name, tx_hash);
            
            // Try to get the actual token address from creator cache
            let token_address = if let Some(ref cache) = self.token_cache {
                // Use tokio runtime to run async function
                let from_checksum = checksum_address(&from.trim_start_matches("0x"));
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        cache.creators.get_token_by_creator(&from_checksum).await
                    })
                })
            } else {
                None
            };
            
            // Use found token address or fall back to 'to' address or empty
            let token_addr = token_address.unwrap_or_else(|| {
                if to != "contract_creation" {
                    to.to_string()
                } else {
                    String::new()
                }
            });
            
            // Create signal alert
            let selector_hex = hex::encode(selector_bytes);
            let signal = SignalAlert {
                alert_type: "trading_enabled".to_string(),
                function_name: function_name.to_string(),
                tx_hash: tx_hash.to_string(),
                from_address: from.to_string(),
                to_address: to.to_string(),
                value: value.to_string(),
                gas_price: gas_price.to_string(),
                selector: selector_hex.clone(),
                timestamp: timestamp.to_string(),
                detection_latency_us: 0, // Will be set by receiver
            };
            
            // Publish via ZMQ
            self.publish_signal(&signal);
            
            // Log to trading enabled file with pattern-friendly format including token address
            if let Ok(mut log_file) = TRADING_ENABLED_LOG.lock() {
                let _ = writeln!(log_file, 
                    "[{}] {} | {} | {} | {}", 
                    timestamp, function_name, token_addr, from, tx_hash
                );
                let _ = log_file.flush();
            }
            return;
        }
        
        // Check swaps - we count them but don't log/alert
        if let Some(_function_name) = self.swap.detect(selector_bytes) {
            stats.swaps += 1;
            return;
        }
        
        // Check for approve function first - needs special handling
        if selector_bytes == &hex_to_bytes("095ea7b3") {
            // This is an approve function - check if it's an LP token approval
            let is_lp_approval = if let Some(ref cache) = self.token_cache {
                let to_checksum = checksum_address(&to.trim_start_matches("0x"));
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        cache.pools.is_pool_address(&to_checksum).await
                    })
                })
            } else {
                false
            };
            
            if is_lp_approval {
                // This is an LP token approval - critical signal!
                stats.liquidity_removals += 1; // Count as liquidity removal preparation
                info!("🚨 LP TOKEN APPROVAL: Preparing for liquidity removal in tx {}", tx_hash);
                
                // Create critical signal alert
                let selector_hex = hex::encode(selector_bytes);
                let signal = SignalAlert {
                    alert_type: "lp_token_approval".to_string(),
                    function_name: "approve (LP Token)".to_string(),
                    tx_hash: tx_hash.to_string(),
                    from_address: from.to_string(),
                    to_address: to.to_string(),
                    value: value.to_string(),
                    gas_price: gas_price.to_string(),
                    selector: selector_hex.clone(),
                    timestamp: timestamp.to_string(),
                    detection_latency_us: 0,
                    };
                
                // Publish via ZMQ
                self.publish_signal(&signal);
                
                // Log to liquidity removal file as preparation
                if let Ok(mut log_file) = LIQUIDITY_REMOVAL_LOG.lock() {
                    let _ = writeln!(log_file, 
                        "[{}] LP APPROVAL TX: {} | From: {} | LP Pair: {} | Value: {} | GasPrice: {} | Function: approve (LP Token) | Selector: {}", 
                        timestamp, tx_hash, from, to, value, gas_price, selector_hex
                    );
                    let _ = log_file.flush();
                }
                return;
            }
            // Regular token approval - just log as other function
            stats.other_functions += 1;
            return;
        }
        
        
        // All other functions - just count them
        stats.other_functions += 1;
    }
    
    /// Get the log directory path
    pub fn get_log_dir(&self) -> &std::path::Path {
        &*LOG_DIR
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> FunctionStats {
        match FUNCTION_STATS.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to acquire function stats lock: {}", e);
                FunctionStats::default()
            }
        }
    }
    
    /// Publish signal via ZMQ
    fn publish_signal(&self, signal: &SignalAlert) {
        if let Ok(publisher) = ZMQ_PUBLISHER.lock() {
            if let Some(ref socket) = *publisher {
                match serde_json::to_string(signal) {
                    Ok(json) => {
                        match socket.send(&json, zmq::DONTWAIT) {
                            Ok(_) => {
                                // Signal published successfully
                            }
                            Err(zmq::Error::EAGAIN) => {
                                warn!("ZMQ publisher buffer full, signal dropped");
                            }
                            Err(e) => {
                                error!("Failed to publish signal: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize signal: {}", e);
                    }
                }
            }
        }
    }
    
    /// Log periodic statistics summary
    pub fn log_stats_summary(&self) {
        let stats = self.get_stats();
        
        info!("📊 Function Detection Statistics:");
        info!("   Total transactions checked: {}", stats.total_checked);
        info!("   Liquidity removals: {}", stats.liquidity_removals);
        info!("   Trading enabled: {}", stats.trading_enabled);
        info!("   Swaps: {}", stats.swaps);
        info!("   Other functions: {}", stats.other_functions);
    }
    
    /// Log performance metrics to the performance log
    pub fn log_performance_metrics(&self, total_processed: u64, 
                                  avg_detection_ms: f64, max_detection_ms: f64,
                                  avg_function_ms: f64, max_function_ms: f64) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        
        if let Ok(mut log_file) = SIGNAL_DETECTOR_LOG.lock() {
            let _ = writeln!(log_file, "\n[{}] === SIGNAL DETECTOR PERFORMANCE ({} processed) ===", timestamp, total_processed);
            let _ = writeln!(log_file, "  IPC Detection Latency: Average: {:.3}ms, Maximum: {:.3}ms", 
                           avg_detection_ms, max_detection_ms);
            let _ = writeln!(log_file, "  Function Detection Time: Average: {:.3}ms, Maximum: {:.3}ms", 
                           avg_function_ms, max_function_ms);
            let _ = log_file.flush();
        }
    }
}

/// Detector for liquidity removal functions
struct LiquidityRemovalDetector {
    signatures: HashMap<[u8; 4], &'static str>,
}

impl LiquidityRemovalDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();
        
        // Uniswap V2 Router
        signatures.insert(hex_to_bytes("02751cec"), "removeLiquidityETH");
        signatures.insert(hex_to_bytes("baa2abde"), "removeLiquidity");
        signatures.insert(hex_to_bytes("af2979eb"), "removeLiquidityETHSupportingFeeOnTransferTokens");
        signatures.insert(hex_to_bytes("5b0d5984"), "removeLiquidityETHWithPermit");
        signatures.insert(hex_to_bytes("ded9382a"), "removeLiquidityETHWithPermitSupportingFeeOnTransferTokens");
        
        // Uniswap V3 Position Manager
        signatures.insert(hex_to_bytes("0c49ccbe"), "decreaseLiquidity");
        
        // Balancer
        signatures.insert(hex_to_bytes("8bdb3913"), "exitPool");
        
        // Curve
        signatures.insert(hex_to_bytes("1a4d01d2"), "remove_liquidity");
        signatures.insert(hex_to_bytes("517a55a3"), "remove_liquidity_one_coin");
        signatures.insert(hex_to_bytes("5b36389c"), "remove_liquidity_imbalance");
        
        // SushiSwap (same as Uniswap V2)
        // PancakeSwap (same as Uniswap V2)
        
        Self { signatures }
    }
    
    fn detect(&self, selector: &[u8]) -> Option<&'static str> {
        if selector.len() >= 4 {
            let mut key = [0u8; 4];
            key.copy_from_slice(&selector[0..4]);
            self.signatures.get(&key).copied()
        } else {
            None
        }
    }
}

/// Helper function to convert hex string to 4-byte array at compile time
const fn hex_to_bytes(hex: &'static str) -> [u8; 4] {
    let bytes = hex.as_bytes();
    let mut result = [0u8; 4];
    let mut i = 0;
    while i < 4 {
        let high = hex_char_to_byte(bytes[i * 2]);
        let low = hex_char_to_byte(bytes[i * 2 + 1]);
        result[i] = (high << 4) | low;
        i += 1;
    }
    result
}

const fn hex_char_to_byte(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

/// Detector for trading enabled functions
struct TradingEnabledDetector {
    signatures: HashMap<[u8; 4], &'static str>,
}

impl TradingEnabledDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();
        
        // Confirmed trading enabled functions
        signatures.insert(hex_to_bytes("8a8c523c"), "enableTrading");  // ✓ Confirmed
        signatures.insert(hex_to_bytes("c9567bf9"), "openTrading");    // ✓ Confirmed
        signatures.insert(hex_to_bytes("8ee88c53"), "enableTrading");
        signatures.insert(hex_to_bytes("fb201b1d"), "startTrading");
        
        Self { signatures }
    }
    
    fn detect(&self, selector: &[u8]) -> Option<&'static str> {
        if selector.len() >= 4 {
            let mut key = [0u8; 4];
            key.copy_from_slice(&selector[0..4]);
            self.signatures.get(&key).copied()
        } else {
            None
        }
    }
}


/// Detector for swap functions
struct SwapDetector {
    signatures: HashMap<[u8; 4], &'static str>,
}

impl SwapDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();
        
        // Uniswap V2/V3 and forks
        signatures.insert(hex_to_bytes("38ed1739"), "swapExactTokensForTokens");
        signatures.insert(hex_to_bytes("8803dbee"), "swapTokensForExactTokens");
        signatures.insert(hex_to_bytes("7ff36ab5"), "swapExactETHForTokens");
        signatures.insert(hex_to_bytes("4a25d94a"), "swapTokensForExactETH");
        signatures.insert(hex_to_bytes("18cbafe5"), "swapExactTokensForETH");
        signatures.insert(hex_to_bytes("fb3bdb41"), "swapETHForExactTokens");
        signatures.insert(hex_to_bytes("791ac947"), "swapExactTokensForETHSupportingFeeOnTransferTokens");
        signatures.insert(hex_to_bytes("b6f9de95"), "swapExactETHForTokensSupportingFeeOnTransferTokens");
        
        // Uniswap V3
        signatures.insert(hex_to_bytes("414bf389"), "exactInputSingle");
        signatures.insert(hex_to_bytes("db3e2198"), "exactOutputSingle");
        signatures.insert(hex_to_bytes("c04b8d59"), "exactInput");
        signatures.insert(hex_to_bytes("f28c0498"), "exactOutput");
        
        // 1inch
        signatures.insert(hex_to_bytes("2e95b6c8"), "swap");
        signatures.insert(hex_to_bytes("7c025200"), "swap_1inch_v2");
        signatures.insert(hex_to_bytes("e449022e"), "uniswapV3Swap");
        
        // 0x Protocol
        signatures.insert(hex_to_bytes("d9627aa4"), "sellToUniswap");
        signatures.insert(hex_to_bytes("3598d8ab"), "sellToLiquidityProvider");
        
        // Curve
        signatures.insert(hex_to_bytes("3df02124"), "exchange");
        signatures.insert(hex_to_bytes("5b41b908"), "exchange_underlying");
        
        // Balancer
        signatures.insert(hex_to_bytes("52bbbe29"), "swap_balancer");
        signatures.insert(hex_to_bytes("945bcec9"), "batchSwap");
        
        Self { signatures }
    }
    
    fn detect(&self, selector: &[u8]) -> Option<&'static str> {
        if selector.len() >= 4 {
            let mut key = [0u8; 4];
            key.copy_from_slice(&selector[0..4]);
            self.signatures.get(&key).copied()
        } else {
            None
        }
    }
}

impl Clone for FunctionStats {
    fn clone(&self) -> Self {
        Self {
            total_checked: self.total_checked,
            liquidity_removals: self.liquidity_removals,
            trading_enabled: self.trading_enabled,
            swaps: self.swaps,
            other_functions: self.other_functions,
        }
    }
}