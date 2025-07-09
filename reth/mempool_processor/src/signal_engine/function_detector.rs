// signal_engine/function_detector.rs
//
// Simple function signature detector that categorizes transactions
// based on their function selectors (4-byte signatures)

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use lazy_static::lazy_static;
use tracing::{info, warn, error};
use std::collections::HashMap;
use std::path::PathBuf;
use chrono::Utc;
use zmq::{Context, Socket};
use serde::{Serialize, Deserialize};

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

/// Transaction with detected function information
#[derive(Debug, Clone)]
pub struct TransactionWithFunctions {
    pub tx: crate::mempool_fetcher::NonBlockingTransaction,
    pub functions: Vec<String>,
    pub has_liquidity_removal: bool,
    pub has_trading_enabled: bool,
}

/// Function detector that categorizes transactions by their function signatures
pub struct FunctionDetector {
    liquidity_removal: LiquidityRemovalDetector,
    trading_enabled: TradingEnabledDetector,
    swap: SwapDetector,
}

impl FunctionDetector {
    pub fn new() -> Self {
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
        }
    }
    
    /// Check if transaction is a liquidity removal (simplified)
    pub fn is_liquidity_removal(&self, input_data: &[u8]) -> Option<&'static str> {
        if input_data.len() >= 4 {
            let selector = hex::encode(&input_data[0..4]);
            self.liquidity_removal.detect(&selector)
        } else {
            None
        }
    }
    
    /// Detect function for a transaction and return the function name if interesting
    pub fn detect_function(&self, tx: &crate::mempool_fetcher::NonBlockingTransaction) -> Option<String> {
        if tx.input.len() < 4 {
            return None;
        }
        
        let selector = hex::encode(&tx.input[0..4]);
        
        // Check liquidity removal
        if let Some(function_name) = self.liquidity_removal.detect(&selector) {
            return Some(function_name.to_string());
        }
        
        // Check trading enabled
        if let Some(function_name) = self.trading_enabled.detect(&selector) {
            return Some(function_name.to_string());
        }
        
        // Check swaps
        if let Some(function_name) = self.swap.detect(&selector) {
            return Some(function_name.to_string());
        }
        
        None
    }
    
    
    /// Detect all function types in the transaction
    pub fn detect_from_ipc(&self, ipc_tx: &crate::mempool_fetcher::NonBlockingTransaction) {
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
    pub fn detect_batch(&self, transactions: Vec<crate::mempool_fetcher::NonBlockingTransaction>) -> Vec<TransactionWithFunctions> {
        transactions.into_iter().map(|tx| {
            let mut functions = Vec::new();
            let mut has_liquidity_removal = false;
            let mut has_trading_enabled = false;
            
            // Skip if no input data
            if tx.input.len() < 4 {
                return TransactionWithFunctions {
                    tx,
                    functions,
                    has_liquidity_removal,
                    has_trading_enabled,
                };
            }
            
            let selector = hex::encode(&tx.input[0..4]);
            
            // Check liquidity removal
            if let Some(function_name) = self.liquidity_removal.detect(&selector) {
                functions.push(function_name.to_string());
                has_liquidity_removal = true;
            }
            
            // Check trading enabled
            if let Some(function_name) = self.trading_enabled.detect(&selector) {
                functions.push(function_name.to_string());
                has_trading_enabled = true;
            }
            
            // Check swaps
            if let Some(function_name) = self.swap.detect(&selector) {
                functions.push(function_name.to_string());
            }
            
            // Still call the existing detection for logging and ZMQ publishing
            self.detect_from_ipc(&tx);
            
            TransactionWithFunctions {
                tx,
                functions,
                has_liquidity_removal,
                has_trading_enabled,
            }
        }).collect()
    }
    
    /// Internal function to detect all function types with extracted details
    fn detect_all(&self, tx_hash: &str, from: &str, to: &str, value: &str, gas_price: &str, input_data: &[u8]) {
        if input_data.len() < 4 {
            return;
        }
        
        let selector = hex::encode(&input_data[0..4]);
        let mut stats = FUNCTION_STATS.lock().unwrap();
        stats.total_checked += 1;
        
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        
        // Check liquidity removal first
        if let Some(function_name) = self.liquidity_removal.detect(&selector) {
            stats.liquidity_removals += 1;
            info!("💧 LIQUIDITY REMOVAL: {} in tx {}", function_name, tx_hash);
            
            // Create signal alert
            let signal = SignalAlert {
                alert_type: "liquidity_removal".to_string(),
                function_name: function_name.to_string(),
                tx_hash: tx_hash.to_string(),
                from_address: from.to_string(),
                to_address: to.to_string(),
                value: value.to_string(),
                gas_price: gas_price.to_string(),
                selector: selector.clone(),
                timestamp: timestamp.to_string(),
                detection_latency_us: 0, // Will be set by receiver
            };
            
            // Publish via ZMQ
            self.publish_signal(&signal);
            
            // Log to liquidity removal file with full transaction details
            if let Ok(mut log_file) = LIQUIDITY_REMOVAL_LOG.lock() {
                let _ = writeln!(log_file, 
                    "[{}] TX: {} | From: {} | To: {} | Value: {} | GasPrice: {} | Function: {} | Selector: {}", 
                    timestamp, tx_hash, from, to, value, gas_price, function_name, selector
                );
                let _ = log_file.flush();
            }
            return;
        }
        
        // Check trading enabled
        if let Some(function_name) = self.trading_enabled.detect(&selector) {
            stats.trading_enabled += 1;
            info!("🎯 TRADING ENABLED: {} in tx {}", function_name, tx_hash);
            
            // Create signal alert
            let signal = SignalAlert {
                alert_type: "trading_enabled".to_string(),
                function_name: function_name.to_string(),
                tx_hash: tx_hash.to_string(),
                from_address: from.to_string(),
                to_address: to.to_string(),
                value: value.to_string(),
                gas_price: gas_price.to_string(),
                selector: selector.clone(),
                timestamp: timestamp.to_string(),
                detection_latency_us: 0, // Will be set by receiver
            };
            
            // Publish via ZMQ
            self.publish_signal(&signal);
            
            // Log to trading enabled file with full transaction details
            if let Ok(mut log_file) = TRADING_ENABLED_LOG.lock() {
                let _ = writeln!(log_file, 
                    "[{}] TX: {} | From: {} | To: {} | Value: {} | GasPrice: {} | Function: {} | Selector: {}", 
                    timestamp, tx_hash, from, to, value, gas_price, function_name, selector
                );
                let _ = log_file.flush();
            }
            return;
        }
        
        // Check swaps - we count them but don't log/alert
        if let Some(_function_name) = self.swap.detect(&selector) {
            stats.swaps += 1;
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
        FUNCTION_STATS.lock().unwrap().clone()
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
            let _ = writeln!(log_file, "\n[{}] === SIGNAL DETECTOR PERFORMANCE ===", timestamp);
            let _ = writeln!(log_file, "  Total Processed: {}", total_processed);
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
    signatures: HashMap<&'static str, &'static str>,
}

impl LiquidityRemovalDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();
        
        // Uniswap V2 Router
        signatures.insert("02751cec", "removeLiquidityETH");
        signatures.insert("baa2abde", "removeLiquidity");
        signatures.insert("af2979eb", "removeLiquidityETHSupportingFeeOnTransferTokens");
        signatures.insert("5b0d5984", "removeLiquidityETHWithPermit");
        signatures.insert("ded9382a", "removeLiquidityETHWithPermitSupportingFeeOnTransferTokens");
        
        // Uniswap V3 Position Manager
        signatures.insert("0c49ccbe", "decreaseLiquidity");
        
        // Balancer
        signatures.insert("8bdb3913", "exitPool");
        
        // Curve
        signatures.insert("1a4d01d2", "remove_liquidity");
        signatures.insert("517a55a3", "remove_liquidity_one_coin");
        signatures.insert("5b36389c", "remove_liquidity_imbalance");
        
        // SushiSwap (same as Uniswap V2)
        // PancakeSwap (same as Uniswap V2)
        
        Self { signatures }
    }
    
    fn detect(&self, selector: &str) -> Option<&'static str> {
        self.signatures.get(selector).copied()
    }
}

/// Detector for trading enabled functions
struct TradingEnabledDetector {
    signatures: HashMap<&'static str, &'static str>,
}

impl TradingEnabledDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();
        
        // Trading enabled functions
        signatures.insert("8a8c523c", "setTradingEnabled");
        signatures.insert("8ee88c53", "enableTrading");
        signatures.insert("c9567bf9", "openTrading");
        signatures.insert("fb201b1d", "startTrading");
        
        Self { signatures }
    }
    
    fn detect(&self, selector: &str) -> Option<&'static str> {
        self.signatures.get(selector).copied()
    }
}


/// Detector for swap functions
struct SwapDetector {
    signatures: HashMap<&'static str, &'static str>,
}

impl SwapDetector {
    fn new() -> Self {
        let mut signatures = HashMap::new();
        
        // Uniswap V2/V3 and forks
        signatures.insert("38ed1739", "swapExactTokensForTokens");
        signatures.insert("8803dbee", "swapTokensForExactTokens");
        signatures.insert("7ff36ab5", "swapExactETHForTokens");
        signatures.insert("4a25d94a", "swapTokensForExactETH");
        signatures.insert("18cbafe5", "swapExactTokensForETH");
        signatures.insert("fb3bdb41", "swapETHForExactTokens");
        signatures.insert("791ac947", "swapExactTokensForETHSupportingFeeOnTransferTokens");
        signatures.insert("b6f9de95", "swapExactETHForTokensSupportingFeeOnTransferTokens");
        
        // Uniswap V3
        signatures.insert("414bf389", "exactInputSingle");
        signatures.insert("db3e2198", "exactOutputSingle");
        signatures.insert("c04b8d59", "exactInput");
        signatures.insert("f28c0498", "exactOutput");
        
        // 1inch
        signatures.insert("2e95b6c8", "swap");
        signatures.insert("7c025200", "swap_1inch_v2");
        signatures.insert("e449022e", "uniswapV3Swap");
        
        // 0x Protocol
        signatures.insert("d9627aa4", "sellToUniswap");
        signatures.insert("3598d8ab", "sellToLiquidityProvider");
        
        // Curve
        signatures.insert("3df02124", "exchange");
        signatures.insert("5b41b908", "exchange_underlying");
        
        // Balancer
        signatures.insert("52bbbe29", "swap_balancer");
        signatures.insert("945bcec9", "batchSwap");
        
        Self { signatures }
    }
    
    fn detect(&self, selector: &str) -> Option<&'static str> {
        self.signatures.get(selector).copied()
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