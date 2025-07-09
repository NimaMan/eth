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
use chrono::Utc;
use serde::{Serialize, Deserialize};
use zmq::Context;

use alloy_primitives::Address;
use crate::common::address::alloy_address_to_checksum;
use crate::pool_subscriber::cache::PoolStateCache;
use super::AddressStateChange;
use std::collections::HashSet;

lazy_static! {
    /// Global statistics
    static ref SIGNAL_STATS: Mutex<SignalStats> = Mutex::new(SignalStats::default());
    
    /// Stablecoin addresses
    static ref STABLECOIN_ADDRESSES: HashSet<String> = {
        let mut addresses = HashSet::new();
        // From common_addresses.py
        addresses.insert("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_lowercase()); // USDC
        addresses.insert("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_lowercase()); // USDT
        addresses.insert("0x6B175474E89094C44Da98b954EedeAC495271d0F".to_lowercase()); // DAI
        addresses.insert("0x4Fabb145d64652a948d72533023f6E7A623C7C53".to_lowercase()); // BUSD
        addresses.insert("0x8E870D67F660D95d5be530380D0eC0bd388289E1".to_lowercase()); // PAX
        addresses.insert("0x956F47F50A910163D8BF957Cf5846D573E7f87CA".to_lowercase()); // FEI
        addresses.insert("0x853d955aCEf822Db058eb8505911ED77F175b99e".to_lowercase()); // FRAX
        addresses.insert("0x5f98805A4E8be255a32880FDeC7F6728C6568bA0".to_lowercase()); // LUSD
        // Wrapped stablecoins
        addresses.insert("0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643".to_lowercase()); // cDAI
        addresses.insert("0x39AA39c021dfbaE8faC545936693aC917d5E7563".to_lowercase()); // cUSDC
        // Additional stablecoins from Python file
        addresses.insert("0x0000000000085d4780B73119b644AE5ecd22b376".to_lowercase()); // TUSD
        addresses.insert("0x674C6Ad92Fd080e4004b2312b45f796a192D27a0".to_lowercase()); // USDN
        addresses.insert("0xe2f2a5C287993345a840Db3B0845fbC70f5935a5".to_lowercase()); // mUSD
        addresses.insert("0x1456688345527bE1f37E9e627DA0837D6f08C925".to_lowercase()); // USDP
        addresses.insert("0x57Ab1ec28D129707052df4dF418D58a2D46d5f51".to_lowercase()); // sUSD
        addresses.insert("0x056Fd409E1d7A124BD7017459dFEa2F387b6d5Cd".to_lowercase()); // GUSD
        addresses.insert("0xBC6DA0FE9aD5f3b0d58160288917AA56653660E9".to_lowercase()); // alUSD
        addresses.insert("0x0E2EC54fC0B509F445631Bf4b91AB8168230C752".to_lowercase()); // LINKUSD
        addresses.insert("0x83F20F44975D03b1b09e64809B757c47f942BEeA".to_lowercase()); // sDAI
        addresses.insert("0x6c3ea9036406852006290770BEdFcAbA0e23A0e8".to_lowercase()); // PYUSD
        addresses.insert("0x0C10bF8FcB7Bf5412187A595ab97a3609160b5c6".to_lowercase()); // USDD
        addresses.insert("0xdC035D45d973E3EC169d2276DDab16f1e407384F".to_lowercase()); // USDS
        addresses.insert("0x4DeA9e918c6289a52cd469cAC652727B7b412Cd2".to_lowercase()); // USD0
        addresses.insert("0x605D26FBd5be761089281d5cec2Ce86eeA667109".to_lowercase()); // USD0++
        addresses.insert("0x4c9EDD5852cd905f086C759E8383e09bff1E68B3".to_lowercase()); // USDe
        addresses.insert("0x426E7d03f9803Dd11cb8616C65b99a3c0AfeA6dE".to_lowercase()); // sUSDe
        addresses.insert("0xC52D7F23a2e460248Db6eE192Cb23dD12bDDCbf6".to_lowercase()); // USD1
        addresses.insert("0x45fDb1b92a649fb6A64Ef1511D3Ba5Bf60044838".to_lowercase()); // EURS
        addresses.insert("0xC581b735A1688071A1746c968e0798D642EDE491".to_lowercase()); // EURT
        addresses.insert("0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c".to_lowercase()); // EUROC
        addresses.insert("0x3231Cb76718CDeF2155FC47b5286d82e6eDA273f".to_lowercase()); // EURCV
        addresses
    };
    
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
    pub stablecoin_burns: u64,
    pub stablecoin_mints: u64,
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
    pool_cache: Option<PoolStateCache>,
    scam_alerts_log: Mutex<std::fs::File>,
    simulation_signals_log: Mutex<std::fs::File>,
    stablecoin_burns_log: Mutex<std::fs::File>,
    stablecoin_mints_log: Mutex<std::fs::File>,
    current_functions: Vec<String>,
}

impl SignalDetector {
    pub fn new(config: SignalDetectionConfig, log_dir: PathBuf) -> Self {
        info!("🔍 Signal detector initialized");
        info!("📁 Using log directory: {}", log_dir.display());
        
        // Create scam alerts log file
        let scam_alerts_path = log_dir.join("scam_alerts.log");
        let scam_alerts_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&scam_alerts_path)
            .expect("Failed to open scam alerts log file");
        let scam_alerts_log = Mutex::new(scam_alerts_file);
        
        // Create simulation signals log file  
        let simulation_signals_path = log_dir.join("simulation_signals.log");
        let simulation_signals_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&simulation_signals_path)
            .expect("Failed to open simulation signals log file");
        let simulation_signals_log = Mutex::new(simulation_signals_file);
        
        // Create stablecoin burns log file
        let stablecoin_burns_path = log_dir.join("stablecoin_burns.log");
        let stablecoin_burns_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stablecoin_burns_path)
            .expect("Failed to open stablecoin burns log file");
        let stablecoin_burns_log = Mutex::new(stablecoin_burns_file);
        
        // Create stablecoin mints log file
        let stablecoin_mints_path = log_dir.join("stablecoin_mints.log");
        let stablecoin_mints_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stablecoin_mints_path)
            .expect("Failed to open stablecoin mints log file");
        let stablecoin_mints_log = Mutex::new(stablecoin_mints_file);
        
        // Log startup information to simulation signals log
        if let Ok(mut log_file) = simulation_signals_log.lock() {
            let _ = writeln!(log_file, "\n{} ==========================================", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🚀 Starting Transaction Simulation Signal Detection", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 📊 Analyzing state changes from simulations", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ⚡ Publishing signals to tcp://127.0.0.1:5557", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ==========================================\n", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = log_file.flush();
        }
        
        // Log startup information to scam alerts log (includes all pool drains)
        if let Ok(mut log_file) = scam_alerts_log.lock() {
            let _ = writeln!(log_file, "\n{} ==========================================", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🚀 Starting Scam Detection (Simulation)", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 📊 Detecting pool drains: >40% drain OR <0.3 ETH remaining", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ⚡ Only tracking pools from Python pool subscriber", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ==========================================\n", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = log_file.flush();
        }
        
        // Log startup information to stablecoin burns log
        if let Ok(mut log_file) = stablecoin_burns_log.lock() {
            let _ = writeln!(log_file, "\n{} ==========================================", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🚀 Starting Stablecoin Burn Detection", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 📊 Tracking burns for {} stablecoins", Utc::now().format("%Y-%m-%d %H:%M:%S"), STABLECOIN_ADDRESSES.len());
            let _ = writeln!(log_file, "{} ⚡ Detecting from transaction simulations", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ==========================================\n", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = log_file.flush();
        }
        
        // Log startup information to stablecoin mints log
        if let Ok(mut log_file) = stablecoin_mints_log.lock() {
            let _ = writeln!(log_file, "\n{} ==========================================", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 🚀 Starting Stablecoin Mint Detection", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} 📊 Tracking mints for {} stablecoins", Utc::now().format("%Y-%m-%d %H:%M:%S"), STABLECOIN_ADDRESSES.len());
            let _ = writeln!(log_file, "{} ⚡ Detecting from transaction simulations", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = writeln!(log_file, "{} ==========================================\n", Utc::now().format("%Y-%m-%d %H:%M:%S"));
            let _ = log_file.flush();
        }
        
        Self { 
            config,
            pool_cache: None,
            scam_alerts_log,
            simulation_signals_log,
            stablecoin_burns_log,
            stablecoin_mints_log,
            current_functions: Vec::new(),
        }
    }
    
    /// Set the pool state cache for drain detection
    pub fn set_pool_cache(&mut self, pool_cache: PoolStateCache) {
        self.pool_cache = Some(pool_cache);
        info!("📊 Pool state cache connected to signal detector");
    }
    
    /// Set the functions detected for the current transaction
    pub fn set_functions(&mut self, functions: Vec<String>) {
        self.current_functions = functions;
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
        
        // Check for stablecoin burns and mints
        for (address, changes) in state_changes {
            let address_str = alloy_address_to_checksum(*address);
            
            // Check if this is a stablecoin
            if STABLECOIN_ADDRESSES.contains(&address_str.to_lowercase()) {
                // Look for burn/mint patterns in token changes
                for (token_addr, amount) in &changes.token_net {
                    if STABLECOIN_ADDRESSES.contains(&token_addr.to_lowercase()) {
                        if *amount < -1.0 {
                            // This is a burn (negative balance change > 1 token)
                            stats.stablecoin_burns += 1;
                            
                            let token_name = self.get_stablecoin_name(token_addr);
                            error!("🔥 STABLECOIN BURN: {} burned {:.2} tokens in tx {}", 
                                   token_name, amount.abs(), tx_hash);
                            
                            // Log to stablecoin burns
                            if let Ok(mut log_file) = self.stablecoin_burns_log.lock() {
                                let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
                                let _ = writeln!(log_file,
                                    "[{}] BURN - TX: {} | Token: {} ({}) | Amount: {:.6} | From: {}",
                                    timestamp, tx_hash, token_name, token_addr, amount.abs(),
                                    alloy_address_to_checksum(from_address));
                                let _ = log_file.flush();
                            }
                            
                            // Create signal
                            signals.push(SimulationSignal {
                                signal_type: "stablecoin_burn".to_string(),
                                severity: "high".to_string(),
                                tx_hash: tx_hash.to_string(),
                                from_address: alloy_address_to_checksum(from_address),
                                to_address: to_address.map(|a| alloy_address_to_checksum(a)),
                                affected_addresses: vec![token_addr.clone()],
                                eth_change: changes.eth_net,
                                token_changes: changes.token_net.clone(),
                                details: format!("{} burn: {:.6} tokens", token_name, amount.abs()),
                                timestamp: Utc::now().to_rfc3339(),
                                simulation_time_us,
                            });
                        } else if *amount > 1.0 {
                            // This is a mint (positive balance change > 1 token)
                            stats.stablecoin_mints += 1;
                            
                            let token_name = self.get_stablecoin_name(token_addr);
                            info!("💰 STABLECOIN MINT: {} minted {:.2} tokens in tx {}", 
                                  token_name, amount, tx_hash);
                            
                            // Log to stablecoin mints
                            if let Ok(mut log_file) = self.stablecoin_mints_log.lock() {
                                let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
                                let _ = writeln!(log_file,
                                    "[{}] MINT - TX: {} | Token: {} ({}) | Amount: {:.6} | From: {}",
                                    timestamp, tx_hash, token_name, token_addr, amount,
                                    alloy_address_to_checksum(from_address));
                                let _ = log_file.flush();
                            }
                            
                            // Create signal
                            signals.push(SimulationSignal {
                                signal_type: "stablecoin_mint".to_string(),
                                severity: "medium".to_string(),
                                tx_hash: tx_hash.to_string(),
                                from_address: alloy_address_to_checksum(from_address),
                                to_address: to_address.map(|a| alloy_address_to_checksum(a)),
                                affected_addresses: vec![token_addr.clone()],
                                eth_change: changes.eth_net,
                                token_changes: changes.token_net.clone(),
                                details: format!("{} mint: {:.6} tokens", token_name, amount),
                                timestamp: Utc::now().to_rfc3339(),
                                simulation_time_us,
                            });
                        }
                    }
                }
            }
        }
        
        // Only check for scams if we have pool cache
        if let Some(ref pool_cache) = self.pool_cache {
            // Check each address to see if it's a tracked pool
            for (address, changes) in state_changes {
                let address_str = alloy_address_to_checksum(*address);
                
                // Only process if this is a tracked pool
                if let Some(pool_state) = pool_cache.get_pool(&address_str) {
                    let eth_change = changes.eth_net;
                    
                    // Only check for drains (negative ETH change)
                    if eth_change < 0.0 {
                        // Calculate what the new reserve would be
                        let new_reserve = pool_state.eth_reserve + eth_change;
                        let drain_percent = (eth_change.abs() / pool_state.eth_reserve) * 100.0;
                        
                        // Check if pool loses >40% OR goes below 0.3 ETH
                        if drain_percent > 40.0 || new_reserve < 0.3 {
                            // Check if this is a swap function - if so, skip scam detection
                            let is_swap = self.current_functions.iter().any(|f| 
                                f.contains("swap") || 
                                f.contains("Swap") || 
                                f == "exactInputSingle" ||
                                f == "exactOutputSingle" ||
                                f == "exactInput" ||
                                f == "exactOutput" ||
                                f == "exchange" ||
                                f == "exchange_underlying" ||
                                f == "batchSwap" ||
                                f == "sellToUniswap" ||
                                f == "sellToLiquidityProvider"
                            );
                            
                            if is_swap {
                                debug!("Skipping scam alert for swap transaction: {:?}", self.current_functions);
                                continue;
                            }
                            
                            // This is a scam - log it
                            error!("🚨 SCAM DETECTED: Pool {} draining {:.1}% ({:.6} ETH), new reserve: {:.6} ETH", 
                                   address_str, drain_percent, eth_change.abs(), new_reserve);
                            
                            // Log to scam alerts with all state changes
                            if let Ok(mut log_file) = self.scam_alerts_log.lock() {
                                let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
                                let _ = writeln!(log_file,
                                    "\n[{}] SCAM ALERT - TX: {}", timestamp, tx_hash);
                                let _ = writeln!(log_file,
                                    "Pool: {} | ETH Drained: {:.6} ({:.1}%) | From: {} | New Reserve: {:.6} ETH",
                                    address_str, eth_change.abs(), drain_percent,
                                    alloy_address_to_checksum(from_address), new_reserve);
                                
                                // Log all address state changes for this transaction
                                let _ = writeln!(log_file, "All State Changes:");
                                for (addr, change) in state_changes {
                                    let addr_str = alloy_address_to_checksum(*addr);
                                    if change.eth_net.abs() > 0.001 || !change.token_net.is_empty() {
                                        let _ = writeln!(log_file, "  - {}: ETH: {:.6}", addr_str, change.eth_net);
                                        for (token, amount) in &change.token_net {
                                            let _ = writeln!(log_file, "    Token {}: {:.6}", token, amount);
                                        }
                                    }
                                }
                                let _ = log_file.flush();
                            }
                            
                            // Create signal
                            stats.scams_detected += 1;
                            signals.push(SimulationSignal {
                                signal_type: "scam_alert".to_string(),
                                severity: "critical".to_string(),
                                tx_hash: tx_hash.to_string(),
                                from_address: alloy_address_to_checksum(from_address),
                                to_address: to_address.map(|a| alloy_address_to_checksum(a)),
                                affected_addresses: vec![address_str.clone()],
                                eth_change,
                                token_changes: changes.token_net.clone(),
                                details: format!("Pool drain: {:.1}% ({:.6} ETH), new reserve: {:.6} ETH", 
                                                drain_percent, eth_change.abs(), new_reserve),
                                timestamp: Utc::now().to_rfc3339(),
                                simulation_time_us,
                            });
                        }
                    }
                }
            }
        } else {
            debug!("No pool cache available, skipping scam detection");
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
    
    /// Get stablecoin name from address
    fn get_stablecoin_name(&self, address: &str) -> String {
        match address.to_lowercase().as_str() {
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => "USDC".to_string(),
            "0xdac17f958d2ee523a2206206994597c13d831ec7" => "USDT".to_string(),
            "0x6b175474e89094c44da98b954eedeac495271d0f" => "DAI".to_string(),
            "0x4fabb145d64652a948d72533023f6e7a623c7c53" => "BUSD".to_string(),
            "0x8e870d67f660d95d5be530380d0ec0bd388289e1" => "PAX".to_string(),
            "0x956f47f50a910163d8bf957cf5846d573e7f87ca" => "FEI".to_string(),
            "0x853d955acef822db058eb8505911ed77f175b99e" => "FRAX".to_string(),
            "0x5f98805a4e8be255a32880fdec7f6728c6568ba0" => "LUSD".to_string(),
            "0x5d3a536e4d6dbd6114cc1ead35777bab948e3643" => "cDAI".to_string(),
            "0x39aa39c021dfbae8fac545936693ac917d5e7563" => "cUSDC".to_string(),
            "0x0000000000085d4780b73119b644ae5ecd22b376" => "TUSD".to_string(),
            "0x674c6ad92fd080e4004b2312b45f796a192d27a0" => "USDN".to_string(),
            "0xe2f2a5c287993345a840db3b0845fbc70f5935a5" => "mUSD".to_string(),
            "0x1456688345527be1f37e9e627da0837d6f08c925" => "USDP".to_string(),
            "0x57ab1ec28d129707052df4df418d58a2d46d5f51" => "sUSD".to_string(),
            "0x056fd409e1d7a124bd7017459dfea2f387b6d5cd" => "GUSD".to_string(),
            "0xbc6da0fe9ad5f3b0d58160288917aa56653660e9" => "alUSD".to_string(),
            "0x0e2ec54fc0b509f445631bf4b91ab8168230c752" => "LINKUSD".to_string(),
            "0x83f20f44975d03b1b09e64809b757c47f942beea" => "sDAI".to_string(),
            "0x6c3ea9036406852006290770bedfcaba0e23a0e8" => "PYUSD".to_string(),
            "0x0c10bf8fcb7bf5412187a595ab97a3609160b5c6" => "USDD".to_string(),
            "0xdc035d45d973e3ec169d2276ddab16f1e407384f" => "USDS".to_string(),
            "0x4dea9e918c6289a52cd469cac652727b7b412cd2" => "USD0".to_string(),
            "0x605d26fbd5be761089281d5cec2ce86eea667109" => "USD0++".to_string(),
            "0x4c9edd5852cd905f086c759e8383e09bff1e68b3" => "USDe".to_string(),
            "0x426e7d03f9803dd11cb8616c65b99a3c0afea6de" => "sUSDe".to_string(),
            "0xc52d7f23a2e460248db6ee192cb23dd12bddcbf6" => "USD1".to_string(),
            "0x45fdb1b92a649fb6a64ef1511d3ba5bf60044838" => "EURS".to_string(),
            "0xc581b735a1688071a1746c968e0798d642ede491" => "EURT".to_string(),
            "0x1abaea1f7c830bd89acc67ec4af516284b1bc33c" => "EUROC".to_string(),
            "0x3231cb76718cdef2155fc47b5286d82e6eda273f" => "EURCV".to_string(),
            _ => "Unknown Stablecoin".to_string(),
        }
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> SignalStats {
        SIGNAL_STATS.lock().unwrap().clone()
    }
    
    /// Log periodic statistics summary
    pub fn log_stats_summary(&self) {
        let stats = self.get_stats();
        
        if let Ok(mut log_file) = self.simulation_signals_log.lock() {
            let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(log_file, "\n[{}] === SIMULATION SIGNAL STATISTICS ===", timestamp);
            let _ = writeln!(log_file, "  Total Simulations Analyzed: {}", stats.total_simulations_analyzed);
            let _ = writeln!(log_file, "  Addresses Analyzed: {}", stats.addresses_analyzed);
            let _ = writeln!(log_file, "  Liquidity Drains: {}", stats.liquidity_drains_detected);
            let _ = writeln!(log_file, "  Scam Alerts: {}", stats.scams_detected);
            let _ = writeln!(log_file, "  Liquidity Warnings: {}", stats.liquidity_warnings);
            let _ = writeln!(log_file, "  MEV Opportunities: {}", stats.mev_opportunities);
            let _ = writeln!(log_file, "  Stablecoin Burns: {}", stats.stablecoin_burns);
            let _ = writeln!(log_file, "  Stablecoin Mints: {}", stats.stablecoin_mints);
            let _ = log_file.flush();
        }
        
        info!("📊 Simulation Signal Statistics:");
        info!("   Total simulations analyzed: {}", stats.total_simulations_analyzed);
        info!("   Liquidity drains detected: {}", stats.liquidity_drains_detected);
        info!("   Scam alerts: {}", stats.scams_detected);
        info!("   Stablecoin burns: {}", stats.stablecoin_burns);
        info!("   Stablecoin mints: {}", stats.stablecoin_mints);
    }
}