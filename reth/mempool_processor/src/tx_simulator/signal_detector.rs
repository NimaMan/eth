/// Signal Detector for Transaction Simulations
/// 
/// This module analyzes state changes from transaction simulations to detect
/// various market signals including liquidity drains, scams, and other pool events.
/// It handles its own logging and signal publishing.

use std::collections::HashMap;
use std::sync::Arc;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::path::PathBuf;
use tracing::{info, debug, warn, error};
use lazy_static::lazy_static;
use chrono::{Local, Utc};
use serde::{Serialize, Deserialize};
use zmq::Context;

use alloy_primitives::Address;
use crate::common::address::alloy_address_to_checksum;
use super::AddressStateChange;

lazy_static! {
    /// Base log directory
    static ref LOG_DIR: PathBuf = {
        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let dir = PathBuf::from("/home/nima/code/crypto/logs/mempool")
            .join(format!("simulation_signals_{}", timestamp));
        std::fs::create_dir_all(&dir).expect("Failed to create log directory");
        dir
    };
    
    /// Main simulation signals log file
    static ref SIMULATION_SIGNALS_LOG: Mutex<std::fs::File> = {
        let log_path = LOG_DIR.join("simulation_signals.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open simulation signals log file");
        
        Mutex::new(file)
    };
    
    /// Liquidity drain events log
    static ref LIQUIDITY_DRAIN_LOG: Mutex<std::fs::File> = {
        let log_path = LOG_DIR.join("liquidity_drains.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open liquidity drain log file");
        
        Mutex::new(file)
    };
    
    /// Scam alerts log
    static ref SCAM_ALERTS_LOG: Mutex<std::fs::File> = {
        let log_path = LOG_DIR.join("scam_alerts.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .expect("Failed to open scam alerts log file");
        
        Mutex::new(file)
    };
    
    /// Global statistics
    static ref SIGNAL_STATS: Mutex<SignalStats> = Mutex::new(SignalStats::default());
    
    /// ZMQ Publisher for simulation signals
    static ref ZMQ_PUBLISHER: Mutex<Option<zmq::Socket>> = {
        match Context::new().socket(zmq::PUB) {
            Ok(socket) => {
                // Set socket options
                let _ = socket.set_sndhwm(10000);
                let _ = socket.set_linger(0);
                
                // Bind to different port than function detector
                match socket.bind("tcp://127.0.0.1:5557") {
                    Ok(_) => {
                        info!("✅ Simulation signal publisher bound to tcp://127.0.0.1:5557");
                        Mutex::new(Some(socket))
                    }
                    Err(e) => {
                        error!("Failed to bind simulation signal publisher: {}", e);
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

#[derive(Debug, Default, Clone)]
pub struct SignalStats {
    pub total_simulations_analyzed: u64,
    pub liquidity_drains_detected: u64,
    pub scams_detected: u64,
    pub liquidity_warnings: u64,
    pub mev_opportunities: u64,
    pub addresses_analyzed: u64,
}

/// Configuration for signal detection
#[derive(Debug, Clone)]
pub struct SignalDetectionConfig {
    /// ETH threshold below which pool is considered drained
    pub eth_threshold: f64,
    
    /// Percentage drain to trigger scam alert
    pub scam_drain_percent: f64,
    
    /// Percentage change to trigger liquidity warning
    pub liquidity_warning_percent: f64,
    
    /// Minimum value change to consider significant (in ETH)
    pub min_value_change: f64,
    
    /// Known DEX router addresses
    pub dex_routers: Vec<String>,
}

impl Default for SignalDetectionConfig {
    fn default() -> Self {
        Self {
            eth_threshold: 0.01,
            scam_drain_percent: 0.5,  // 50%
            liquidity_warning_percent: 0.2,  // 20%
            min_value_change: 0.1,  // 0.1 ETH
            dex_routers: vec![
                "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(), // Uniswap V2
                "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(), // Uniswap V3
                "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string(), // SushiSwap
            ],
        }
    }
}

/// Signal alert for publishing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationSignal {
    pub signal_type: String,
    pub severity: String,
    pub tx_hash: String,
    pub from_address: String,
    pub to_address: Option<String>,
    pub affected_addresses: Vec<String>,
    pub eth_change: f64,
    pub token_changes: HashMap<String, f64>,
    pub details: String,
    pub timestamp: String,
    pub simulation_time_us: u64,
}

/// Transaction signal detector
pub struct SignalDetector {
    config: SignalDetectionConfig,
}

impl SignalDetector {
    pub fn new(config: SignalDetectionConfig) -> Self {
        info!("🔍 Signal detector initialized");
        info!("📁 Log directory: {}", LOG_DIR.display());
        
        // Log startup information
        if let Ok(mut log_file) = SIMULATION_SIGNALS_LOG.lock() {
            let _ = writeln!(log_file, "\n{} ==========================================", Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🚀 Starting Transaction Simulation Signal Detection", Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 📊 Analyzing state changes from simulations", Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ⚡ Publishing signals to tcp://127.0.0.1:5557", Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ==========================================\n", Local::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = log_file.flush();
        }
        
        Self { config }
    }
    
    /// Analyze state changes from a transaction simulation
    pub fn analyze_simulation_result(
        &self,
        tx_hash: &str,
        from_address: Address,
        to_address: Option<Address>,
        state_changes: &HashMap<Address, AddressStateChange>,
        simulation_time_us: u64,
    ) -> Vec<SimulationSignal> {
        let mut signals = Vec::new();
        let mut stats = SIGNAL_STATS.lock().unwrap();
        stats.total_simulations_analyzed += 1;
        stats.addresses_analyzed += state_changes.len() as u64;
        
        debug!("🔍 Analyzing {} address changes for tx {}", state_changes.len(), tx_hash);
        
        // Log analysis to file
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        if let Ok(mut log_file) = SIMULATION_SIGNALS_LOG.lock() {
            let _ = writeln!(log_file, "[{}] TX: {} | Addresses: {} | Simulation: {}μs", 
                           timestamp, tx_hash, state_changes.len(), simulation_time_us);
        }
        
        // Analyze each address change
        for (address, changes) in state_changes {
            let address_str = alloy_address_to_checksum(*address);
            
            // Check if this is a DEX router
            let is_dex = self.config.dex_routers.contains(&address_str);
            
            // Calculate ETH change
            let eth_change = changes.eth_net;
            
            // Skip if change is too small
            if eth_change.abs() < self.config.min_value_change {
                continue;
            }
            
            // Detect different types of signals
            if is_dex && eth_change < -self.config.min_value_change {
                // Liquidity removal from DEX
                if let Some(signal) = self.detect_liquidity_drain(
                    tx_hash,
                    &from_address,
                    &to_address,
                    &address_str,
                    eth_change,
                    changes,
                    simulation_time_us,
                ) {
                    stats.liquidity_drains_detected += 1;
                    signals.push(signal);
                }
            }
            
            // Check for potential scams (large drains)
            if eth_change < -1.0 { // More than 1 ETH drain
                if let Some(signal) = self.detect_potential_scam(
                    tx_hash,
                    &from_address,
                    &to_address,
                    &address_str,
                    eth_change,
                    changes,
                    simulation_time_us,
                ) {
                    stats.scams_detected += 1;
                    signals.push(signal);
                }
            }
        }
        
        // Publish signals
        for signal in &signals {
            self.publish_signal(signal);
        }
        
        // Log summary
        if !signals.is_empty() {
            info!("🎯 Found {} signals from simulation of {}", signals.len(), tx_hash);
        }
        
        signals
    }
    
    /// Detect liquidity drain signal
    fn detect_liquidity_drain(
        &self,
        tx_hash: &str,
        from_address: &Address,
        to_address: &Option<Address>,
        pool_address: &str,
        eth_change: f64,
        changes: &AddressStateChange,
        simulation_time_us: u64,
    ) -> Option<SimulationSignal> {
        let drain_percent = eth_change.abs() * 100.0; // Would need pool reserves for accurate percentage
        
        warn!("💧 LIQUIDITY DRAIN: {} ETH removed from {}", eth_change.abs(), pool_address);
        
        // Log to liquidity drain file
        if let Ok(mut log_file) = LIQUIDITY_DRAIN_LOG.lock() {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(log_file,
                "[{}] TX: {} | Pool: {} | ETH Removed: {:.6} | From: {}",
                timestamp, tx_hash, pool_address, eth_change.abs(), 
                alloy_address_to_checksum(*from_address)
            );
            let _ = log_file.flush();
        }
        
        // Extract token changes from DebugAddressStateChange
        let token_changes = changes.token_net.clone();
        
        Some(SimulationSignal {
            signal_type: "liquidity_drain".to_string(),
            severity: if eth_change.abs() > 10.0 { "critical" } else { "high" }.to_string(),
            tx_hash: tx_hash.to_string(),
            from_address: alloy_address_to_checksum(*from_address),
            to_address: to_address.map(|a| alloy_address_to_checksum(a)),
            affected_addresses: vec![pool_address.to_string()],
            eth_change,
            token_changes,
            details: format!("Liquidity removal of {:.6} ETH from DEX pool", eth_change.abs()),
            timestamp: Utc::now().to_rfc3339(),
            simulation_time_us,
        })
    }
    
    /// Detect potential scam
    fn detect_potential_scam(
        &self,
        tx_hash: &str,
        from_address: &Address,
        to_address: &Option<Address>,
        affected_address: &str,
        eth_change: f64,
        changes: &AddressStateChange,
        simulation_time_us: u64,
    ) -> Option<SimulationSignal> {
        error!("🚨 POTENTIAL SCAM: {} ETH drained from {}", eth_change.abs(), affected_address);
        
        // Log to scam alerts file
        if let Ok(mut log_file) = SCAM_ALERTS_LOG.lock() {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(log_file,
                "[{}] TX: {} | Address: {} | ETH Drained: {:.6} | From: {}",
                timestamp, tx_hash, affected_address, eth_change.abs(),
                alloy_address_to_checksum(*from_address)
            );
            let _ = log_file.flush();
        }
        
        Some(SimulationSignal {
            signal_type: "scam_alert".to_string(),
            severity: "critical".to_string(),
            tx_hash: tx_hash.to_string(),
            from_address: alloy_address_to_checksum(*from_address),
            to_address: to_address.map(|a| alloy_address_to_checksum(a)),
            affected_addresses: vec![affected_address.to_string()],
            eth_change,
            token_changes: HashMap::new(),
            details: format!("Critical ETH drain of {:.6} ETH detected", eth_change.abs()),
            timestamp: Utc::now().to_rfc3339(),
            simulation_time_us,
        })
    }
    
    /// Publish signal via ZMQ
    fn publish_signal(&self, signal: &SimulationSignal) {
        if let Ok(publisher) = ZMQ_PUBLISHER.lock() {
            if let Some(ref socket) = *publisher {
                match serde_json::to_string(signal) {
                    Ok(json) => {
                        match socket.send(&json, zmq::DONTWAIT) {
                            Ok(_) => {
                                debug!("📡 Published simulation signal: {}", signal.signal_type);
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
    
    /// Get statistics
    pub fn get_stats(&self) -> SignalStats {
        SIGNAL_STATS.lock().unwrap().clone()
    }
    
    /// Log periodic statistics summary
    pub fn log_stats_summary(&self) {
        let stats = self.get_stats();
        
        if let Ok(mut log_file) = SIMULATION_SIGNALS_LOG.lock() {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(log_file, "\n[{}] === SIMULATION SIGNAL STATISTICS ===", timestamp);
            let _ = writeln!(log_file, "  Total Simulations Analyzed: {}", stats.total_simulations_analyzed);
            let _ = writeln!(log_file, "  Addresses Analyzed: {}", stats.addresses_analyzed);
            let _ = writeln!(log_file, "  Liquidity Drains: {}", stats.liquidity_drains_detected);
            let _ = writeln!(log_file, "  Scam Alerts: {}", stats.scams_detected);
            let _ = writeln!(log_file, "  Liquidity Warnings: {}", stats.liquidity_warnings);
            let _ = writeln!(log_file, "  MEV Opportunities: {}", stats.mev_opportunities);
            let _ = log_file.flush();
        }
        
        info!("📊 Simulation Signal Statistics:");
        info!("   Total simulations analyzed: {}", stats.total_simulations_analyzed);
        info!("   Liquidity drains detected: {}", stats.liquidity_drains_detected);
        info!("   Scam alerts: {}", stats.scams_detected);
    }
}