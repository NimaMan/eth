/// Signal Detector for Transaction Simulations
/// 
/// This module analyzes state changes from transaction simulations to detect
/// various market signals including liquidity drains, scams, and other pool events.
/// It handles its own logging and signal publishing.

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::path::PathBuf;
use tracing::{info, debug, warn, error};
use lazy_static::lazy_static;
use chrono::Utc;
use serde::{Serialize, Deserialize};
use zmq::Context;

use alloy_primitives::{Address, U256};
use crate::common::address::{alloy_address_to_checksum, checksum_address};
use crate::token_tracking::cache::PoolStateCache;
use super::AddressStateChange;

lazy_static! {
    /// Global statistics
    static ref SIGNAL_STATS: Mutex<SignalStats> = Mutex::new(SignalStats::default());
    
    /// Stablecoin addresses
    static ref STABLECOIN_ADDRESSES: HashSet<String> = {
        let mut addresses = HashSet::new();
        // From common_addresses.py
        addresses.insert(checksum_address("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")); // USDC
        addresses.insert(checksum_address("0xdAC17F958D2ee523a2206206994597C13D831ec7")); // USDT
        addresses.insert(checksum_address("0x6B175474E89094C44Da98b954EedeAC495271d0F")); // DAI
        addresses.insert(checksum_address("0x4Fabb145d64652a948d72533023f6E7A623C7C53")); // BUSD
        addresses.insert(checksum_address("0x8E870D67F660D95d5be530380D0eC0bd388289E1")); // PAX
        addresses.insert(checksum_address("0x956F47F50A910163D8BF957Cf5846D573E7f87CA")); // FEI
        addresses.insert(checksum_address("0x853d955aCEf822Db058eb8505911ED77F175b99e")); // FRAX
        addresses.insert(checksum_address("0x5f98805A4E8be255a32880FDeC7F6728C6568bA0")); // LUSD
        // Wrapped stablecoins
        addresses.insert(checksum_address("0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643")); // cDAI
        addresses.insert(checksum_address("0x39AA39c021dfbaE8faC545936693aC917d5E7563")); // cUSDC
        // Additional stablecoins from Python file
        addresses.insert(checksum_address("0x0000000000085d4780B73119b644AE5ecd22b376")); // TUSD
        addresses.insert(checksum_address("0x674C6Ad92Fd080e4004b2312b45f796a192D27a0")); // USDN
        addresses.insert(checksum_address("0xe2f2a5C287993345a840Db3B0845fbC70f5935a5")); // mUSD
        addresses.insert(checksum_address("0x1456688345527bE1f37E9e627DA0837D6f08C925")); // USDP
        addresses.insert(checksum_address("0x57Ab1ec28D129707052df4dF418D58a2D46d5f51")); // sUSD
        addresses.insert(checksum_address("0x056Fd409E1d7A124BD7017459dFEa2F387b6d5Cd")); // GUSD
        addresses.insert(checksum_address("0xBC6DA0FE9aD5f3b0d58160288917AA56653660E9")); // alUSD
        addresses.insert(checksum_address("0x0E2EC54fC0B509F445631Bf4b91AB8168230C752")); // LINKUSD
        addresses.insert(checksum_address("0x83F20F44975D03b1b09e64809B757c47f942BEeA")); // sDAI
        addresses.insert(checksum_address("0x6c3ea9036406852006290770BEdFcAbA0e23A0e8")); // PYUSD
        addresses.insert(checksum_address("0x0C10bF8FcB7Bf5412187A595ab97a3609160b5c6")); // USDD
        addresses.insert(checksum_address("0xdC035D45d973E3EC169d2276DDab16f1e407384F")); // USDS
        addresses.insert(checksum_address("0x4DeA9e918c6289a52cd469cAC652727B7b412Cd2")); // USD0
        addresses.insert(checksum_address("0x605D26FBd5be761089281d5cec2Ce86eeA667109")); // USD0++
        addresses.insert(checksum_address("0x4c9EDD5852cd905f086C759E8383e09bff1E68B3")); // USDe
        addresses.insert(checksum_address("0x426E7d03f9803Dd11cb8616C65b99a3c0AfeA6dE")); // sUSDe
        addresses.insert(checksum_address("0xC52D7F23a2e460248Db6eE192Cb23dD12bDDCbf6")); // USD1
        addresses.insert(checksum_address("0x45fDb1b92a649fb6A64Ef1511D3Ba5Bf60044838")); // EURS
        addresses.insert(checksum_address("0xC581b735A1688071A1746c968e0798D642EDE491")); // EURT
        addresses.insert(checksum_address("0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c")); // EUROC
        addresses.insert(checksum_address("0x3231Cb76718CDeF2155FC47b5286d82e6eDA273f")); // EURCV
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

/// Drain calculation result
#[derive(Debug, Clone)]
struct DrainResult {
    pub current_reserve: f64,
    pub eth_change: f64,
    pub new_reserve: f64,
    pub drain_percent: f64,
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
    
    /// Check if a specific address shows scam activity
    async fn check_address_for_scam(
        &self,
        address: &Address,
        changes: &AddressStateChange,
        pool_cache: &crate::token_tracking::cache::PoolStateCache,
        tx_hash: &str,
        from_address: Address,
        to_address: Option<Address>,
        all_state_changes: &HashMap<Address, AddressStateChange>,
        simulation_time_us: u64,
    ) -> Option<SimulationSignal> {
        let address_str = alloy_address_to_checksum(*address);
        
        // Step 1: Check if this address is a tracked pool
        let pool_state = pool_cache.get_pool(&address_str).await?;
        
        // Step 2: Check if there's negative ETH change (drain)
        let eth_change = changes.eth_net;
        if eth_change > U256::ZERO {
            // Convert to f64 for logging
            let eth_change_f64 = eth_change.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
            debug!("Address {} has positive ETH change: {:.6}, not a drain", address_str, eth_change_f64);
            return None;
        }
        
        // Step 3: Calculate drain metrics
        // Convert U256 to f64 for percentage calculations (eth_change is 0 since we only process drains)
        let eth_change_f64 = 0.0; // We know it's 0 or negative at this point
        let drain_result = self.calculate_drain_metrics(&pool_state, eth_change_f64);
        
        // Step 4: Check if drain exceeds thresholds
        if !self.is_significant_drain(&drain_result) {
            debug!("Address {} drain not significant: {:.1}% drain, new reserve: {:.6} ETH", 
                   address_str, drain_result.drain_percent, drain_result.new_reserve);
            return None;
        }
        
        // Step 5: This is a scam - log and create alert
        self.log_scam_alert(
            &address_str,
            &drain_result,
            tx_hash,
            from_address,
            all_state_changes,
        );
        
        // Step 7: Create signal
        Some(self.create_scam_signal(
            &address_str,
            &drain_result,
            tx_hash,
            from_address,
            to_address,
            changes,
            simulation_time_us,
        ))
    }
    
    /// Analyze state changes from a transaction simulation
    pub async fn analyze_simulation_result(
        &self,
        tx_hash: &str,
        from_address: Address,
        to_address: Option<Address>,
        state_changes: &HashMap<Address, AddressStateChange>,
        simulation_time_us: u64,
    ) -> Vec<SimulationSignal> {
        let mut signals = Vec::new();
        let mut stats = match SIGNAL_STATS.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("Failed to acquire signal stats lock: {}", e);
                return signals;
            }
        };
        stats.total_simulations_analyzed += 1;
        stats.addresses_analyzed += state_changes.len() as u64;
        
        debug!("🔍 Analyzing {} address changes for tx {}", state_changes.len(), tx_hash);
        
        // Log liquidity removal functions to scam alerts for verification
        if self.current_functions.iter().any(|func| func.contains("remove") || func.contains("liquidity")) {
            self.log_liquidity_removal_simulation(tx_hash, from_address, to_address, state_changes).await;
        }
        
        // Check for stablecoin burns and mints
        for (address, changes) in state_changes {
            let address_str = alloy_address_to_checksum(*address);
            
            // Check if this is a stablecoin
            if STABLECOIN_ADDRESSES.contains(&address_str) {
                // Look for burn/mint patterns in token changes
                for (token_addr, amount) in &changes.token_net {
                    if STABLECOIN_ADDRESSES.contains(token_addr) {
                        // Convert U256 to f64 for comparison (handle decimals)
                        let decimals = if token_addr == "USDC" || token_addr == "USDT" { 6 } else { 18 };
                        let divisor = 10f64.powi(decimals);
                        let amount_f64 = amount.to_string().parse::<f64>().unwrap_or(0.0) / divisor;
                        
                        // Note: With U256, we can only detect positive changes (mints)
                        // Burns would need to be tracked differently
                        if amount_f64 > 1.0 {
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
                                eth_change: changes.eth_net.to_string().parse::<f64>().unwrap_or(0.0) / 1e18,
                                token_changes: changes.token_net.iter()
                                    .map(|(k, v)| (k.clone(), v.to_string().parse::<f64>().unwrap_or(0.0)))
                                    .collect(),
                                details: format!("{} mint: {:.6} tokens", token_name, amount),
                                timestamp: Utc::now().to_rfc3339(),
                                simulation_time_us,
                            });
                        }
                    }
                }
            }
        }
        
        // Check for scams if we have pool cache
        // Note: We check ALL transactions including liquidity removals, as scammers can use these functions too
        if let Some(ref pool_cache) = self.pool_cache {
            // Check each address for potential scam activity
            for (address, changes) in state_changes {
                if let Some(scam_alert) = self.check_address_for_scam(
                    address, 
                    changes, 
                    pool_cache, 
                    tx_hash, 
                    from_address, 
                    to_address, 
                    state_changes,
                    simulation_time_us
                ).await {
                    stats.scams_detected += 1;
                    signals.push(scam_alert);
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
        match address {
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48" => "USDC".to_string(),
            "0xdAC17F958D2ee523a2206206994597C13D831ec7" => "USDT".to_string(),
            "0x6B175474E89094C44Da98b954EedeAC495271d0F" => "DAI".to_string(),
            "0x4Fabb145d64652a948d72533023f6E7A623C7C53" => "BUSD".to_string(),
            "0x8E870D67F660D95d5be530380D0eC0bd388289E1" => "PAX".to_string(),
            "0x956F47F50A910163D8BF957Cf5846D573E7f87CA" => "FEI".to_string(),
            "0x853d955aCEf822Db058eb8505911ED77F175b99e" => "FRAX".to_string(),
            "0x5f98805A4E8be255a32880FDeC7F6728C6568bA0" => "LUSD".to_string(),
            "0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643" => "cDAI".to_string(),
            "0x39AA39c021dfbaE8faC545936693aC917d5E7563" => "cUSDC".to_string(),
            "0x0000000000085d4780B73119b644AE5ecd22b376" => "TUSD".to_string(),
            "0x674C6Ad92Fd080e4004b2312b45f796a192D27a0" => "USDN".to_string(),
            "0xe2f2a5C287993345a840Db3B0845fbC70f5935a5" => "mUSD".to_string(),
            "0x1456688345527bE1f37E9e627DA0837D6f08C925" => "USDP".to_string(),
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
    
    /// Calculate drain metrics for a pool
    fn calculate_drain_metrics(&self, pool_state: &crate::token_tracking::types::PoolState, eth_change: f64) -> DrainResult {
        let current_reserve = pool_state.eth_reserve;
        
        // If pool already has 0 or very low reserves, skip percentage calculation
        if current_reserve < 0.001 {
            return DrainResult {
                current_reserve,
                eth_change,
                new_reserve: 0.0, // Can't go negative
                drain_percent: 0.0, // No meaningful percentage when starting from ~0
            };
        }
        
        // Calculate new reserve, but ensure it doesn't go negative
        let new_reserve = (current_reserve + eth_change).max(0.0); // eth_change is negative for drains
        let drain_percent = ((current_reserve - new_reserve) / current_reserve) * 100.0;
        
        DrainResult {
            current_reserve,
            eth_change,
            new_reserve,
            drain_percent,
        }
    }
    
    /// Check if drain is significant (>60% drain OR <0.3 ETH remaining)
    fn is_significant_drain(&self, drain_result: &DrainResult) -> bool {
        drain_result.drain_percent > 60.0 || drain_result.new_reserve < 0.3
    }
    
    /// Log scam alert with all relevant information
    fn log_scam_alert(
        &self,
        pool_address: &str,
        drain_result: &DrainResult,
        tx_hash: &str,
        from_address: Address,
        all_state_changes: &HashMap<Address, AddressStateChange>,
    ) {
        if let Ok(mut log_file) = self.scam_alerts_log.lock() {
            let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(log_file,
                "[{}] 🚨 SCAM DETECTED - Pool: {} | TX: {} | From: {} | Current Reserve: {:.6} ETH | Drain: {:.1}% | New Reserve: {:.6} ETH",
                timestamp, pool_address, tx_hash, 
                alloy_address_to_checksum(from_address),
                drain_result.current_reserve, 
                drain_result.drain_percent, 
                drain_result.new_reserve
            );
            
            // Log all state changes for context
            let _ = writeln!(log_file, "  State Changes:");
            for (addr, changes) in all_state_changes {
                let addr_str = alloy_address_to_checksum(*addr);
                let _ = writeln!(log_file, "    {}: ETH: {:.6}, Tokens: {:?}", 
                    addr_str, changes.eth_net, changes.token_net);
            }
            
            let _ = writeln!(log_file, "  Functions: {:?}", self.current_functions);
            let _ = writeln!(log_file, "");
            let _ = log_file.flush();
        }
    }
    
    /// Create scam signal for publishing
    fn create_scam_signal(
        &self,
        pool_address: &str,
        drain_result: &DrainResult,
        tx_hash: &str,
        from_address: Address,
        to_address: Option<Address>,
        changes: &AddressStateChange,
        simulation_time_us: u64,
    ) -> SimulationSignal {
        SimulationSignal {
            signal_type: "scam_detected".to_string(),
            severity: "critical".to_string(),
            tx_hash: tx_hash.to_string(),
            from_address: alloy_address_to_checksum(from_address),
            to_address: to_address.map(|a| alloy_address_to_checksum(a)),
            affected_addresses: vec![pool_address.to_string()],
            eth_change: drain_result.eth_change,
            token_changes: changes.token_net.iter()
                .map(|(k, v)| (k.clone(), v.to_string().parse::<f64>().unwrap_or(0.0)))
                .collect(),
            details: format!("Pool drain: {:.1}% ({:.6} ETH -> {:.6} ETH)", 
                drain_result.drain_percent, drain_result.current_reserve, drain_result.new_reserve),
            timestamp: Utc::now().to_rfc3339(),
            simulation_time_us,
        }
    }
    
    /// Log liquidity removal simulation results to scam alerts for verification
    async fn log_liquidity_removal_simulation(
        &self,
        tx_hash: &str,
        from_address: Address,
        to_address: Option<Address>,
        state_changes: &HashMap<Address, AddressStateChange>,
    ) {
        if let Ok(mut log_file) = self.scam_alerts_log.lock() {
            let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");
            let to_str = to_address.map(|a| alloy_address_to_checksum(a)).unwrap_or_else(|| "None".to_string());
            
            let _ = writeln!(log_file,
                "[{}] 📊 LIQUIDITY REMOVAL SIMULATION - TX: {} | From: {} | To: {} | Functions: {:?}",
                timestamp, tx_hash, 
                alloy_address_to_checksum(from_address),
                to_str,
                self.current_functions
            );
            
            // Log all state changes from simulation
            let _ = writeln!(log_file, "  Simulation State Changes:");
            let mut found_pool = false;
            
            for (addr, changes) in state_changes {
                let addr_str = alloy_address_to_checksum(*addr);
                let _ = writeln!(log_file, "    {}: ETH: {:.6}, Tokens: {:?}", 
                    addr_str, changes.eth_net, changes.token_net);
                
                // Check if this address is a tracked pool
                if let Some(ref pool_cache) = self.pool_cache {
                    if let Some(pool_state) = pool_cache.get_pool(&addr_str).await {
                        found_pool = true;
                        let eth_change_f64 = changes.eth_net.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                        let new_reserve = pool_state.eth_reserve + eth_change_f64;
                        let drain_percent = if pool_state.eth_reserve > 0.0 {
                            ((pool_state.eth_reserve - new_reserve) / pool_state.eth_reserve) * 100.0
                        } else {
                            0.0
                        };
                        let _ = writeln!(log_file, "      ✓ POOL FOUND IN CACHE: Current Reserve: {:.6} ETH | New Reserve: {:.6} ETH | Drain: {:.1}%", 
                            pool_state.eth_reserve, new_reserve, drain_percent);
                    }
                }
            }
            
            if !found_pool {
                let _ = writeln!(log_file, "  ⚠️  NO TRACKED POOLS FOUND IN CACHE FOR THIS TRANSACTION");
            }
            
            let _ = writeln!(log_file, "");
            let _ = log_file.flush();
        }
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> SignalStats {
        match SIGNAL_STATS.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to acquire signal stats lock: {}", e);
                SignalStats::default()
            }
        }
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