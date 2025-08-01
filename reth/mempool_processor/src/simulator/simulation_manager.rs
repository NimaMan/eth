/// Simulation Manager
/// 
/// Manages transaction simulations based on priority and type

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, debug, warn};
use ethers::types::H256;
use crate::mempool_fetcher::MempoolTransaction;
use crate::tx_router::{TransactionCategory, SimulationPriority};
use crate::signal_detector::{SignalManager, SignalManagerConfig};
use crate::token_tracking::TokenTrackingCache;
use super::{SimulationQueue, SequentialBuySellSimulator};
use super::tx_simulator::TxSimulator;
use std::collections::HashMap;

/// Types of simulation to perform
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimulationType {
    /// Only simulate the transaction itself
    TransactionOnly,
    /// Simulate transaction then buy/sell sequence
    TransactionWithBuySell,
    /// Only simulate buy/sell (for existing tokens)
    BuySellOnly,
}

/// Request for simulation
#[derive(Debug, Clone)]
pub struct SimulationRequest {
    pub tx: MempoolTransaction,
    pub category: TransactionCategory,
    pub priority: SimulationPriority,
    pub simulation_type: SimulationType,
    pub tx_hash: H256,
}

/// Result of simulation
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub request: SimulationRequest,
    pub tx_state_changes: Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>,
    pub buy_sell_result: Option<BuySellResult>,
    pub error: Option<String>,
    pub simulation_time_ms: f64,
    // Addresses needed for tax calculation
    pub token_address: Option<alloy_primitives::Address>,
    pub pool_address: Option<alloy_primitives::Address>,
}

/// Buy/sell simulation result
#[derive(Debug, Clone)]
pub struct BuySellResult {
    pub can_buy: bool,
    pub can_sell: bool,
    // Raw state changes for tax calculation in signal manager
    pub buy_state_changes: Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>,
    pub sell_state_changes: Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>,
}

/// Manager for transaction simulations
pub struct SimulationManager {
    tx_simulator: Arc<TxSimulator>,
    buy_sell_simulator: Arc<SequentialBuySellSimulator>,
    queue: Arc<Mutex<SimulationQueue>>,
    
    // Signal detection
    signal_manager: Arc<Mutex<SignalManager>>,
    token_cache: Arc<TokenTrackingCache>,
    
    // Configuration
    max_concurrent_simulations: usize,
    
    // Statistics
    stats: Arc<Mutex<ManagerStats>>,
}

#[derive(Debug, Default, Clone)]
struct ManagerStats {
    total_requests: u64,
    successful_simulations: u64,
    failed_simulations: u64,
    buy_sell_tests: u64,
    avg_simulation_time_ms: f64,
    max_simulation_time_ms: f64,
}

impl SimulationManager {
    /// Create new simulation manager
    pub fn new(
        tx_simulator: Arc<TxSimulator>,
        buy_sell_simulator: Arc<SequentialBuySellSimulator>,
        token_cache: Arc<TokenTrackingCache>,
        signal_config: SignalManagerConfig,
        max_concurrent: usize,
    ) -> Self {
        let mut signal_manager = SignalManager::new(signal_config);
        signal_manager.set_token_cache(token_cache.clone());
        
        Self {
            tx_simulator,
            buy_sell_simulator,
            queue: Arc::new(Mutex::new(SimulationQueue::new())),
            signal_manager: Arc::new(Mutex::new(signal_manager)),
            token_cache,
            max_concurrent_simulations: max_concurrent,
            stats: Arc::new(Mutex::new(ManagerStats::default())),
        }
    }

    /// Submit a simulation request
    pub async fn submit(&self, request: SimulationRequest) -> Result<(), String> {
        let mut queue = self.queue.lock().await;
        queue.push(request)?;
        
        let mut stats = self.stats.lock().await;
        stats.total_requests += 1;
        
        Ok(())
    }

    /// Process pending simulations
    pub async fn process_queue(&self) -> Vec<SimulationResult> {
        let mut results = Vec::new();
        
        // Get batch of requests based on priority
        let requests = {
            let mut queue = self.queue.lock().await;
            queue.pop_batch(self.max_concurrent_simulations)
        };

        if requests.is_empty() {
            return results;
        }

        debug!("Processing {} simulation requests", requests.len());

        // Process requests concurrently
        let futures: Vec<_> = requests
            .into_iter()
            .map(|req| self.simulate_request(req))
            .collect();

        let batch_results = futures::future::join_all(futures).await;
        
        // Update statistics
        let mut stats = self.stats.lock().await;
        for result in &batch_results {
            if result.error.is_none() {
                stats.successful_simulations += 1;
            } else {
                stats.failed_simulations += 1;
            }
            
            if result.buy_sell_result.is_some() {
                stats.buy_sell_tests += 1;
            }
            
            // Update average and max time
            let total = stats.successful_simulations + stats.failed_simulations;
            stats.avg_simulation_time_ms = 
                (stats.avg_simulation_time_ms * (total - 1) as f64 + result.simulation_time_ms) / total as f64;
            stats.max_simulation_time_ms = stats.max_simulation_time_ms.max(result.simulation_time_ms);
        }

        results.extend(batch_results);
        results
    }

    /// Simulate a single request
    async fn simulate_request(&self, request: SimulationRequest) -> SimulationResult {
        let start = std::time::Instant::now();
        let mut result = SimulationResult {
            request: request.clone(),
            tx_state_changes: None,
            buy_sell_result: None,
            error: None,
            simulation_time_ms: 0.0,
            token_address: None,
            pool_address: None,
        };

        // For both ContractCreation and CreatorTransaction, we always do:
        // 1. Simulate the transaction
        // 2. Run buy/sell tests
        
        match &request.category {
            TransactionCategory::ContractCreation { .. } | 
            TransactionCategory::CreatorTransaction { .. } => {
                // Run combined tx + buy/sell simulation
                match self.simulate_tx_with_buy_sell(&request).await {
                    Ok((tx_state_changes, bs_result, token_addr, pool_addr)) => {
                        result.tx_state_changes = tx_state_changes;
                        result.buy_sell_result = Some(bs_result);
                        result.token_address = Some(token_addr);
                        result.pool_address = pool_addr;
                    }
                    Err(e) => {
                        result.error = Some(e);
                    }
                }
            }
            _ => {
                // For other transaction types, we don't simulate
                result.error = Some("Transaction type not supported for simulation".to_string());
            }
        }

        result.simulation_time_ms = start.elapsed().as_secs_f64() * 1000.0;
        
        // Send results to signal manager
        // TEMPORARILY COMMENTED OUT FOR SIMULATION-ONLY TESTING
        // let mut signal_manager = self.signal_manager.lock().await;
        // signal_manager.process_simulation_result(&result).await;
        
        result
    }

    /// Simulate transaction followed by buy/sell sequence
    async fn simulate_tx_with_buy_sell(&self, request: &SimulationRequest) -> Result<(Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>, BuySellResult, alloy_primitives::Address, Option<alloy_primitives::Address>), String> {
        // Extract token address from category
        let token_address_str = match &request.category {
            TransactionCategory::CreatorTransaction { target_token, creator, .. } => {
                match target_token {
                    Some(token) => token.clone(),
                    None => {
                        // Try to get token from cache if not provided by router
                        if let Some(token_info) = self.token_cache.get_token_for_creator(creator).await {
                            info!("Found token {} for creator {} from cache", token_info.token_address, creator);
                            token_info.token_address.clone()
                        } else {
                            return Err(format!("No token address found for creator {}", creator));
                        }
                    }
                }
            }
            TransactionCategory::ContractCreation { contract_address, .. } => {
                // For new contracts, the contract address IS the token address
                contract_address.clone()
            }
            _ => return Err("Category doesn't support buy/sell simulation".to_string()),
        };

        info!("Running buy/sell simulation for token {}", token_address_str);
        
        // Convert string addresses to alloy Address type
        let token_address = token_address_str.trim_start_matches("0x")
            .parse::<alloy_primitives::Address>()
            .map_err(|e| format!("Invalid token address: {}", e))?;
            
        // Try to find pool address from token cache
        let pool_info = self.token_cache.get_primary_pool(&token_address_str).await;
        let pool_address = if let Some(pool) = pool_info {
            pool.pool_address.trim_start_matches("0x")
                .parse::<alloy_primitives::Address>()
                .unwrap_or(alloy_primitives::Address::ZERO)
        } else {
            // If we don't have a pool yet (new token), we'll discover it during simulation
            alloy_primitives::Address::ZERO
        };
        
        // Get current block number (simulate at latest)
        let block_number = None;
        
        // Create the transaction CallRequest
        let full_tx = crate::mempool_fetcher::FullTransaction {
            hash: request.tx.hash.clone(),
            tx_data: request.tx.data.clone(),
            detection_time: std::time::Instant::now(),
            latency_ns: request.tx.detection_ns,
        };
        
        let tx_call_request = reth_tx_simulator::ipc_to_call_request(&full_tx.tx_data)
            .map_err(|e| format!("Failed to convert transaction: {}", e))?;
        
        // Run the sequential simulation: TX -> Buy -> Approve -> Sell
        match self.buy_sell_simulator.simulate_sequence_with_tx(
            Some(tx_call_request),
            token_address,
            pool_address,
            block_number
        ).await {
            Ok(result) => {
                // Extract transaction state changes
                let tx_state_changes = result.given_tx_result.map(|tx| tx.state_changes);
                
                // Create buy/sell result with raw simulation data
                let bs_result = BuySellResult {
                    can_buy: result.buy_result.success,
                    can_sell: result.sell_result.success,
                    buy_state_changes: Some(result.buy_result.state_changes),
                    sell_state_changes: Some(result.sell_result.state_changes),
                };
                
                Ok((tx_state_changes, bs_result, token_address, if pool_address.is_zero() { None } else { Some(pool_address) }))
            }
            Err(e) => {
                warn!("Sequential simulation failed: {}", e);
                Err(format!("Simulation failed: {}", e))
            }
        }
    }

    /// Get manager statistics
    pub async fn get_stats(&self) -> ManagerStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }
    
}