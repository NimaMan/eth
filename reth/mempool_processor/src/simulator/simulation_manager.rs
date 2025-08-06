/// Simulation Manager
/// 
/// Manages transaction simulations based on priority and type

use std::sync::Arc;
use std::any::TypeId;
use tokio::sync::Mutex;
use tracing::{info, debug, warn, error};
use ethers::types::H256;
use crate::mempool_fetcher::MempoolTransaction;
use crate::tx_router::{TransactionCategory, SimulationPriority};
use crate::signal_detector::{SignalManager, SignalManagerConfig};
use crate::token_tracking::TokenTrackingCache;
use super::{SimulationQueue, UnifiedSimulator, SequenceSimulationResult};
use std::collections::HashMap;
use hex;

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
    // Full sequence simulation details for debugging
    pub sequence_result: Option<SequenceSimulationResult>,
    // Debug info for error analysis
    pub debug_info: Option<String>,
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
    unified_simulator: Arc<UnifiedSimulator>,
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
    /// Create new simulation manager with unified simulator
    pub fn new(
        unified_simulator: Arc<UnifiedSimulator>,
        token_cache: Arc<TokenTrackingCache>,
        signal_config: SignalManagerConfig,
        max_concurrent: usize,
    ) -> Self {
        let mut signal_manager = SignalManager::new(signal_config);
        signal_manager.set_token_cache(token_cache.clone());
        
        Self {
            unified_simulator,
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
        info!("=== SIMULATION MANAGER: Starting simulation for TX {} ===", request.tx.hash);
        info!("  Category: {:?}", request.category);
        info!("  Priority: {:?}", request.priority);
        info!("  Simulation Type: {:?}", request.simulation_type);
        
        let start = std::time::Instant::now();
        let mut result = SimulationResult {
            request: request.clone(),
            tx_state_changes: None,
            buy_sell_result: None,
            error: None,
            simulation_time_ms: 0.0,
            token_address: None,
            pool_address: None,
            sequence_result: None,
            debug_info: None,
        };

        // For both ContractCreation and CreatorTransaction, we always do:
        // 1. Simulate the transaction
        // 2. Run buy/sell tests
        
        match &request.category {
            TransactionCategory::ContractCreation { .. } => {
                // TODO: Handle contract creation properly
                result.error = Some("Contract creation simulation not implemented yet".to_string());
            }
            TransactionCategory::CreatorTransaction { .. } => {
                // Run combined tx + buy/sell simulation
                match self.simulate_tx_with_buy_sell(&request).await {
                    Ok((tx_state_changes, bs_result, token_addr, pool_addr, seq_result)) => {
                        result.tx_state_changes = tx_state_changes;
                        result.buy_sell_result = Some(bs_result);
                        result.token_address = Some(token_addr);
                        result.pool_address = pool_addr;
                        result.sequence_result = seq_result;
                    }
                    Err((e, partial_tx_changes)) => {
                        result.error = Some(e);
                        result.debug_info = Some(format!("Failed during buy/sell simulation"));
                        // Preserve any transaction state changes even if buy/sell failed
                        result.tx_state_changes = partial_tx_changes;
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
        info!("📤 Sending simulation result to signal manager for TX {}", result.request.tx.hash);
        info!("  Result has error: {}, has buy_sell: {}", 
            result.error.is_some(), 
            result.buy_sell_result.is_some()
        );
        let mut signal_manager = self.signal_manager.lock().await;
        let signals = signal_manager.process_simulation_result(&result).await;
        info!("  Signal manager returned {} signals", signals.len());
        
        result
    }

    /// Simulate transaction followed by buy/sell sequence
    async fn simulate_tx_with_buy_sell(&self, request: &SimulationRequest) -> Result<(Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>, BuySellResult, alloy_primitives::Address, Option<alloy_primitives::Address>, Option<SequenceSimulationResult>), (String, Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>)> {
        // Extract token address from category
        let token_address_str = match &request.category {
            TransactionCategory::CreatorTransaction { target_token, creator, .. } => {
                match target_token {
                    Some(token) => token.clone(),
                    None => {
                        // Try to get token from cache if not provided by router
                        if let Some(token_info) = self.token_cache.get_token_for_creator(creator).await {
                            token_info.token_address.clone()
                        } else {
                            return Err((format!("No token address found for creator {}", creator), None));
                        }
                    }
                }
            }
            TransactionCategory::ContractCreation { contract_address, deployer, .. } => {
                // For contract creation, the sequential simulator will:
                // 1. Execute the contract creation transaction (deploying the contract)
                // 2. The contract will then exist at its address
                // 3. Then run buy/sell tests on the deployed contract
                
                if contract_address == "pending" {
                    // We need to calculate the deterministic address where the contract will be deployed
                    // This is deterministic based on deployer + nonce
                    // For now, let's skip the buy/sell test and just simulate the creation
                    return Err(("Contract creation: address calculation not implemented yet".to_string(), None));
                } else {
                    contract_address.clone()
                }
            }
            _ => return Err(("Category doesn't support buy/sell simulation".to_string(), None)),
        };
        
        // Convert string addresses to alloy Address type
        let token_address = token_address_str.trim_start_matches("0x")
            .parse::<alloy_primitives::Address>()
            .map_err(|e| (format!("Invalid token address: {}", e), None))?;
            
        // Try to find pool address from token cache
        let pool_info = self.token_cache.get_primary_pool(&token_address_str).await;
        let pool_address = if let Some(pool) = pool_info {
            let addr = pool.pool_address.trim_start_matches("0x")
                .parse::<alloy_primitives::Address>()
                .unwrap_or(alloy_primitives::Address::ZERO);
            info!("  Found pool address from cache: {:?}", addr);
            addr
        } else {
            // If we don't have a pool yet (new token), we'll discover it during simulation
            info!("  No pool found in cache, using ZERO address");
            alloy_primitives::Address::ZERO
        };
        
        // Get current block number (simulate at latest)
        let block_number = None; // Use latest block
        info!("  Using block number: {:?}", block_number);
        
        // Create the transaction CallRequest
        let full_tx = crate::mempool_fetcher::FullTransaction {
            hash: request.tx.hash.clone(),
            tx_data: request.tx.data.clone(),
            detection_time: std::time::Instant::now(),
            latency_ns: request.tx.detection_ns,
        };
        
        
        let mut tx_call_request = reth_tx_simulator::ipc_to_call_request(&full_tx.tx_data)
            .map_err(|e| (format!("Failed to convert transaction: {}", e), None))?;
        
        // Log the parsed call request for debugging
        info!("  Parsed CallRequest:");
        info!("    from: {:?}", tx_call_request.from);
        info!("    to: {:?}", tx_call_request.to);
        info!("    value: {:?}", tx_call_request.value);
        info!("    gas: {:?}", tx_call_request.gas);
        info!("    gas_price: {:?}", tx_call_request.gas_price);
        info!("    max_fee_per_gas: {:?}", tx_call_request.max_fee_per_gas);
        info!("    data length: {} bytes", tx_call_request.data.as_ref().map(|d| d.len()).unwrap_or(0));
        
        // Check if gas is missing - this should never happen for mined transactions
        if tx_call_request.gas.is_none() {
            error!("WARNING: Gas limit is None for mined transaction!");
            error!("Raw IPC data: {}", serde_json::to_string_pretty(&full_tx.tx_data).unwrap_or_default());
            // Return error instead of using fallback
            return Err(("Gas limit missing from transaction - parsing error".to_string(), None));
        }
        
        // Remove nonce to let the sequential simulator manage it automatically
        tx_call_request.nonce = None;
        
        // Store original gas prices for retry logic
        let original_gas_price = tx_call_request.gas_price;
        let original_max_fee = tx_call_request.max_fee_per_gas;
        
        info!("  Using original gas prices from transaction");
        
        // Store request details for error reporting
        let tx_details = format!(
            "Original TX: from={:?}, to={:?}",
            tx_call_request.from, tx_call_request.to
        );
        
        info!("  Calling buy_sell_simulator.simulate_sequence_with_tx()...");
        info!("    Token: {:?}", token_address);
        info!("    Pool: {:?}", pool_address);
        info!("    from: {:?}", tx_call_request.from);
        info!("    to: {:?}", tx_call_request.to);
        
        // Try simulation with original gas price first
        let simulation_result = match self.unified_simulator.simulate_sequence_with_tx(
            Some(tx_call_request.clone()),
            token_address,
            pool_address,
            block_number
        ).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Check if it's a base fee error
                let error_str = e.to_string();
                if error_str.contains("GasPriceLessThanBasefee") || error_str.contains("base fee") {
                    info!("  Base fee error detected, retrying with 3x gas price");
                    
                    // Triple the gas prices and retry
                    let new_gas_price = original_gas_price.map(|p| p * 3);
                    let new_max_fee = original_max_fee.map(|p| p * 3);
                    
                    tx_call_request.gas_price = new_gas_price;
                    tx_call_request.max_fee_per_gas = new_max_fee;
                    tx_call_request.max_priority_fee_per_gas = Some(2_000_000_000u128); // 2 gwei priority
                    
                    info!("  Retrying with increased gas prices: {:?} gwei", 
                        new_gas_price.map(|p| p / 1_000_000_000));
                    
                    // Retry with higher gas price
                    self.unified_simulator.simulate_sequence_with_tx(
                        Some(tx_call_request.clone()),
                        token_address,
                        pool_address,
                        block_number
                    ).await
                } else {
                    Err(e)
                }
            }
        };
        
        // Process the result
        match simulation_result {
            Ok(result) => {
                info!("  Buy/sell simulation completed successfully:");
                info!("    Given TX: {:?}", result.given_tx_result.as_ref().map(|r| r.success));
                info!("    Buy TX: {}", result.buy_result.success);
                info!("    Sell TX: {}", result.sell_result.success);
                
                // Extract transaction state changes
                let tx_state_changes = result.given_tx_result.as_ref().map(|tx| tx.state_changes.clone());
                
                // Create buy/sell result with raw simulation data
                let bs_result = BuySellResult {
                    can_buy: result.buy_result.success,
                    can_sell: result.sell_result.success,
                    buy_state_changes: Some(result.buy_result.state_changes.clone()),
                    sell_state_changes: Some(result.sell_result.state_changes.clone()),
                };
                
                Ok((tx_state_changes, bs_result, token_address, if pool_address.is_zero() { None } else { Some(pool_address) }, Some(result)))
            }
            Err(e) => {
                info!("  ERROR in sequence simulation: {}", e);
                info!("  Error details: {:?}", e);
                
                // Log detailed error information for debugging
                if e.to_string().contains("LackOfFundForMaxFee") {
                    error!("LackOfFundForMaxFee error detected!");
                    error!("Transaction details:");
                    error!("  TX hash: {}", request.tx.hash);
                    error!("  From: {:?}", tx_call_request.from);
                    error!("  To: {:?}", tx_call_request.to);
                    error!("  Value: {:?}", tx_call_request.value);
                    error!("  Gas: {:?}", tx_call_request.gas);
                    error!("  Gas price: {:?}", tx_call_request.gas_price);
                    error!("  Max fee per gas: {:?}", tx_call_request.max_fee_per_gas);
                    error!("Raw IPC transaction data:");
                    error!("{}", serde_json::to_string_pretty(&full_tx.tx_data).unwrap_or_default());
                }
                
                let error_msg = format!("{}", e);
                Err((error_msg, None))
            }
        }
    }


    /// Get manager statistics
    pub async fn get_stats(&self) -> ManagerStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }
    
}