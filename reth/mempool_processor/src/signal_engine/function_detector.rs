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
use chrono::Local;
use zmq::{Context, Socket};
use serde::{Serialize, Deserialize};

lazy_static! {
    /// Log directory path - initialized once at startup
    static ref LOG_DIR: PathBuf = {
        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
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
}

impl FunctionDetector {
    pub fn new() -> Self {
        info!("🔍 Function detector initialized");
        info!("📁 Log directory: {}", LOG_DIR.display());
        
        // Log startup information to signal detector log
        if let Ok(mut log_file) = SIGNAL_DETECTOR_LOG.lock() {
            let _ = writeln!(log_file, "\n{} ==========================================", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🚀 Starting Mempool Signal Detection Service", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ⚡ Using non-blocking IPC for sub-millisecond latency", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🔍 Function detector initialized", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 📁 Log directory: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), LOG_DIR.display());
            let _ = writeln!(log_file, "{} 🎯 Starting main processing loop...", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ==========================================\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = log_file.flush();
        }
        
        Self {
            liquidity_removal: LiquidityRemovalDetector::new(),
            trading_enabled: TradingEnabledDetector::new(),
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
        
        // All other functions - just count them
        stats.other_functions += 1;
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
        signatures.insert("42966c68", "burn"); // Burns liquidity NFT
        
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


impl Clone for FunctionStats {
    fn clone(&self) -> Self {
        Self {
            total_checked: self.total_checked,
            liquidity_removals: self.liquidity_removals,
            trading_enabled: self.trading_enabled,
            other_functions: self.other_functions,
        }
    }
}