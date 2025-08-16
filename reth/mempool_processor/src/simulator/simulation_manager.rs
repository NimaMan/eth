/// Simulation Manager
/// 
/// Manages transaction simulations on a PER-POOL basis.
/// 
/// Key Architecture:
/// - Each token can have multiple pools (WETH/TOKEN, USDC/TOKEN, etc.)
/// - Each pool is simulated INDEPENDENTLY
/// - Each pool generates its own signal with pool-specific data
/// - Signal = f(token_address, pool_address)
/// 
/// Flow for CreatorTransaction:
/// 1. Extract token address from transaction
/// 2. Get ALL pools for the token from cache
/// 3. Filter to V2 pools (V3/V4 not yet supported)
/// 4. FOR EACH POOL:
///    - Run transaction simulation
///    - Run buy/sell simulation for THIS pool
///    - Create pool-specific SimulationResult
///    - Send to signal manager
///    - Generate unique signal for (token, pool) pair

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, error};
use ethers::types::H256;
use crate::mempool_fetcher::MempoolTransaction;
use crate::tx_router::{TransactionCategory, SimulationPriority};
use crate::signal_detector::{SignalManager, SignalManagerConfig};
use crate::token_tracking::{TokenTrackingCache, calculate_buy_tax, calculate_sell_tax, TaxCalculationResult};
use tokio::sync::Mutex as TokioMutex;
use super::{SimulationQueue, UnifiedSimulator, SequenceSimulationResult};
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
    pub pool_type: Option<String>,  // Pool type (V2, V3, V4)
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
    // Raw state changes for tax calculation in signal manager (kept for debugging)
    pub buy_state_changes: Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>,
    pub sell_state_changes: Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>,
    // Tax calculation results (calculated immediately after simulation)
    pub buy_tax: Option<f64>,      // 0-100% or None if calculation failed
    pub sell_tax: Option<f64>,     // 0-100% or None if calculation failed
    pub buy_tax_error: Option<String>,   // Error message if buy tax calculation failed
    pub sell_tax_error: Option<String>,  // Error message if sell tax calculation failed
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
        publisher: Arc<TokioMutex<crate::signal_publisher::SignalPublisher>>,
        max_concurrent: usize,
    ) -> Self {
        let mut signal_manager = SignalManager::new(signal_config);
        signal_manager.set_token_cache(token_cache.clone());
        signal_manager.set_publisher(publisher);
        
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

    /// Handle LP approval detection without simulation
    pub async fn detect_lp_approval(
        &self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
        category: &crate::tx_router::TransactionCategory,
    ) -> Vec<crate::signal_detector::Signal> {
        let mut signal_manager = self.signal_manager.lock().await;
        signal_manager.detect_lp_approval(tx, category).await;
        // Return empty vec for now, signals are published internally
        Vec::new()
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
            pool_type: None,
            sequence_result: None,
            debug_info: None,
        };

        // For both ContractCreation and CreatorTransaction, we always do:
        // 1. Simulate the transaction
        // 2. Run buy/sell tests
        
        match &request.category {
            TransactionCategory::ContractCreation { .. } => {
                // TODO: Handle contract creation properly
                warn!("Contract creation simulation not implemented yet for TX {}", request.tx.hash);
                result.error = Some("Contract creation simulation not implemented yet".to_string());
            }
            TransactionCategory::CreatorTransaction { .. } => {
                // CRITICAL: Run simulation for EACH pool independently
                // Each pool will generate its own signal
                let all_results = self.simulate_tx_with_buy_sell_all_pools(&request).await;
                
                if all_results.is_empty() {
                    // No pools to simulate
                    result.error = Some("No pools found for token".to_string());
                    result.debug_info = Some(format!("No pools available for simulation"));
                    
                    // Still send to signal manager even with no pools
                    info!("📤 Sending no-pools result to signal manager for TX {}", result.request.tx.hash);
                    let mut signal_manager = self.signal_manager.lock().await;
                    let signals = signal_manager.process_simulation_result(&result).await;
                    info!("  Signal manager returned {} signals", signals.len());
                } else {
                    // CRITICAL: Process each pool's result INDEPENDENTLY
                    // Each pool gets its own SimulationResult and signal
                    for (pool_idx, pool_result) in all_results.into_iter().enumerate() {
                        // Create a UNIQUE SimulationResult for THIS pool
                        let mut pool_specific_result = SimulationResult {
                            request: request.clone(),
                            tx_state_changes: None,
                            buy_sell_result: None,
                            error: None,
                            simulation_time_ms: result.simulation_time_ms,
                            token_address: None,
                            pool_address: None,
                            pool_type: None,
                            sequence_result: None,
                            debug_info: None,
                        };
                        
                        match pool_result {
                            Ok((tx_state_changes, bs_result, token_addr, pool_addr, pool_type, seq_result)) => {
                                pool_specific_result.tx_state_changes = tx_state_changes;
                                pool_specific_result.buy_sell_result = Some(bs_result);
                                pool_specific_result.token_address = Some(token_addr);
                                pool_specific_result.pool_address = pool_addr;
                                pool_specific_result.pool_type = pool_type;
                                pool_specific_result.sequence_result = seq_result;
                            }
                            Err((e, partial_tx_changes)) => {
                                pool_specific_result.error = Some(e);
                                pool_specific_result.debug_info = Some(format!("Failed during buy/sell simulation for pool {}", pool_idx));
                                // Preserve any transaction state changes even if buy/sell failed
                                pool_specific_result.tx_state_changes = partial_tx_changes;
                            }
                        }
                        
                        // Send each pool's results to signal manager separately
                        info!("📤 Sending pool-specific result to signal manager for TX {} (pool {})", 
                            pool_specific_result.request.tx.hash, pool_idx);
                        info!("  Pool address: {:?}", pool_specific_result.pool_address);
                        info!("  Result has error: {}, has buy_sell: {}", 
                            pool_specific_result.error.is_some(), 
                            pool_specific_result.buy_sell_result.is_some()
                        );
                        let mut signal_manager = self.signal_manager.lock().await;
                        let signals = signal_manager.process_simulation_result(&pool_specific_result).await;
                        info!("  Signal manager returned {} signals for pool {}", signals.len(), pool_idx);
                        
                        // Keep the last successful result as the overall result (for backward compatibility)
                        if pool_specific_result.error.is_none() {
                            result = pool_specific_result;
                        }
                    }
                }
            }
            _ => {
                // For other transaction types, we don't simulate
                result.error = Some("Transaction type not supported for simulation".to_string());
            }
        }

        result.simulation_time_ms = start.elapsed().as_secs_f64() * 1000.0;
        
        // For non-CreatorTransaction types, send to signal manager
        // (CreatorTransaction already sends per-pool signals above)
        match &request.category {
            TransactionCategory::CreatorTransaction { .. } => {
                // Already handled per-pool above
            }
            _ => {
                // Send results to signal manager for other transaction types
                info!("📤 Sending simulation result to signal manager for TX {}", result.request.tx.hash);
                info!("  Result has error: {}, has buy_sell: {}", 
                    result.error.is_some(), 
                    result.buy_sell_result.is_some()
                );
                let mut signal_manager = self.signal_manager.lock().await;
                let signals = signal_manager.process_simulation_result(&result).await;
                info!("  Signal manager returned {} signals", signals.len());
            }
        }
        
        result
    }

    /// Simulate transaction followed by buy/sell sequence for ALL pools
    /// 
    /// CRITICAL: This function processes EACH pool independently:
    /// - Each pool gets its own simulation
    /// - Each pool's results are collected separately
    /// - Returns a Vec with one result per pool
    /// - Failed pools don't affect successful ones
    async fn simulate_tx_with_buy_sell_all_pools(&self, request: &SimulationRequest) -> Vec<Result<(Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>, BuySellResult, alloy_primitives::Address, Option<alloy_primitives::Address>, Option<String>, Option<SequenceSimulationResult>), (String, Option<HashMap<alloy_primitives::Address, reth_tx_simulator::AddressStateChange>>)>> {
        // Extract token address from category
        let token_address_str = match &request.category {
            TransactionCategory::CreatorTransaction { target_token, creator, .. } => {
                match target_token {
                    Some(token) => token.clone(),
                    None => {
                        // Try to get token from cache if not provided by router
                        if let Some(token_info) = self.token_cache.get_token_for_creator(creator).await {
                            token_info.address.clone()
                        } else {
                            return vec![Err((format!("No token address found for creator {}", creator), None))];
                        }
                    }
                }
            }
            TransactionCategory::ContractCreation { contract_address,  .. } => {
                // For contract creation, the sequential simulator will:
                // 1. Execute the contract creation transaction (deploying the contract)
                // 2. The contract will then exist at its address
                // 3. Then run buy/sell tests on the deployed contract
                
                if contract_address == "pending" {
                    // We need to calculate the deterministic address where the contract will be deployed
                    // This is deterministic based on deployer + nonce
                    // For now, let's skip the buy/sell test and just simulate the creation
                    return vec![Err(("Contract creation: address calculation not implemented yet".to_string(), None))];
                } else {
                    contract_address.clone()
                }
            }
            _ => return vec![Err(("Category doesn't support buy/sell simulation".to_string(), None))],
        };
        
        // Convert string addresses to alloy Address type
        let token_address = match token_address_str.trim_start_matches("0x")
            .parse::<alloy_primitives::Address>() {
            Ok(addr) => addr,
            Err(e) => return vec![Err((format!("Invalid token address: {}", e), None))],
        };
            
        // Get ALL pools for the token from cache
        let all_pools = self.token_cache.get_pools_for_token(&token_address_str).await;
        
        if all_pools.is_empty() {
            info!("  No pools found in cache for token {}", token_address_str);
            return vec![];  // Return empty vector, no pools to simulate
        }
        
        info!("  Found {} pools for token", all_pools.len());
        
        // Filter to only V2 pools (V3/V4 not supported yet)
        let v2_pools: Vec<_> = all_pools.into_iter()
            .filter(|pool_state| {
                use crate::token_tracking::PoolType;
                let is_v2 = matches!(pool_state.pool_type, PoolType::UniswapV2 | PoolType::Unknown);
                if !is_v2 {
                    info!("  Skipping {:?} pool {} (not supported)", pool_state.pool_type, pool_state.address);
                }
                is_v2
            })
            .collect();
        
        if v2_pools.is_empty() {
            info!("  No V2 pools found for token");
            return vec![];  // Return empty vector, no V2 pools to simulate
        }
        
        info!("  Will simulate {} V2 pools", v2_pools.len());
        
        // Results vector to collect all pool simulations
        let mut results = Vec::new();
        
        // CRITICAL LOOP: Simulate each pool INDEPENDENTLY
        // Each iteration produces a separate result for signal generation
        for (pool_idx, pool_state) in v2_pools.into_iter().enumerate() {
            let pool_address = match pool_state.address.trim_start_matches("0x")
                .parse::<alloy_primitives::Address>() {
                Ok(addr) => addr,
                Err(e) => {
                    results.push(Err((format!("Invalid pool address {}: {}", pool_state.address, e), None)));
                    continue;
                }
            };
            
            let pool_type = format!("{:?}", pool_state.pool_type);
            info!("  [Pool {}] Simulating pool: {:?} (Type: {}, ETH: {:.6})", 
                pool_idx, pool_address, &pool_type, pool_state.eth_reserve);
        
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
            
            let mut tx_call_request = match crate::common::convert::ipc_to_call_request(&full_tx.tx_data) {
                Ok(req) => req,
                Err(e) => {
                    results.push(Err((format!("Failed to convert transaction: {}", e), None)));
                    continue;
                }
            };
            
            // Log the parsed call request for debugging
            info!("  [Pool {}] Parsed CallRequest:", pool_idx);
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
                results.push(Err(("Gas limit missing from transaction - parsing error".to_string(), None)));
                continue;
            }
            
            // Remove nonce to let the sequential simulator manage it automatically
            tx_call_request.nonce = None;
            
            // Store original gas prices for retry logic
            let original_gas_price = tx_call_request.gas_price;
            let original_max_fee = tx_call_request.max_fee_per_gas;
            
            info!("  [Pool {}] Using original gas prices from transaction", pool_idx);
            
            // Store request details for error reporting
            let _tx_details = format!(
                "Original TX: from={:?}, to={:?}",
                tx_call_request.from, tx_call_request.to
            );
            
            info!("  [Pool {}] Calling buy_sell_simulator.simulate_sequence_with_tx()...", pool_idx);
            info!("    Token: {:?}", token_address);
            info!("    Pool: {:?}", pool_address);
            info!("    from: {:?}", tx_call_request.from);
            info!("    to: {:?}", tx_call_request.to);
            
            // Try simulation with original gas price first
            // Use pool_type string
            let simulation_result = match self.unified_simulator.simulate_sequence_with_tx_and_pool_type(
                Some(tx_call_request.clone()),
                token_address,
                pool_address,
                &pool_type,
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
                        self.unified_simulator.simulate_sequence_with_tx_and_pool_type(
                            Some(tx_call_request.clone()),
                            token_address,
                            pool_address,
                            &pool_type,
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
                    info!("  [Pool {}] Buy/sell simulation completed successfully:", pool_idx);
                    info!("    Given TX: {:?}", result.given_tx_result.as_ref().map(|r| r.success));
                    info!("    Buy TX: {}", result.buy_result.success);
                    info!("    Sell TX: {}", result.sell_result.success);
                    
                    // Extract transaction state changes
                    let tx_state_changes = result.given_tx_result.as_ref().map(|tx| tx.state_changes.clone());
                    
                    // Get buyer address from simulator
                    let buyer_address = self.unified_simulator.get_buyer_address();
                    
                    // Calculate buy tax immediately after buy simulation
                    let (buy_tax, buy_tax_error) = if result.buy_result.success {
                        match calculate_buy_tax(&result.buy_result.state_changes, &pool_address, &buyer_address, &token_address) {
                            TaxCalculationResult::Calculated(tax) => {
                                info!("    Buy tax calculated: {:.2}%", tax);
                                (Some(tax), None)
                            }
                            TaxCalculationResult::InvalidSimulation { reason } => {
                                warn!("    Buy tax calculation failed: {}", reason);
                                (None, Some(reason))
                            }
                        }
                    } else {
                        (None, Some("Buy simulation failed".to_string()))
                    };
                    
                    // Calculate sell tax immediately after sell simulation
                    let (sell_tax, sell_tax_error) = if result.sell_result.success {
                        match calculate_sell_tax(&result.sell_result.state_changes, &pool_address, &buyer_address) {
                            TaxCalculationResult::Calculated(tax) => {
                                info!("    Sell tax calculated: {:.2}%", tax);
                                (Some(tax), None)
                            }
                            TaxCalculationResult::InvalidSimulation { reason } => {
                                warn!("    Sell tax calculation failed: {}", reason);
                                (None, Some(reason))
                            }
                        }
                    } else {
                        (None, Some("Sell simulation failed".to_string()))
                    };
                    
                    // Create buy/sell result with tax calculation results
                    let bs_result = BuySellResult {
                        can_buy: result.buy_result.success,
                        can_sell: result.sell_result.success,
                        buy_state_changes: Some(result.buy_result.state_changes.clone()),
                        sell_state_changes: Some(result.sell_result.state_changes.clone()),
                        buy_tax,
                        sell_tax,
                        buy_tax_error,
                        sell_tax_error,
                    };
                    
                    results.push(Ok((tx_state_changes, bs_result, token_address, if pool_address.is_zero() { None } else { Some(pool_address) }, Some(pool_type.clone()), Some(result))));
                }
                Err(e) => {
                    info!("  [Pool {}] ERROR in sequence simulation: {}", pool_idx, e);
                    info!("  Error details: {:?}", e);                    
                    let error_msg = format!("Pool {}: {}", pool_idx, e);
                    results.push(Err((error_msg, None)));
                }
            }
        }  // End of for loop
        
        results
    }

    /// Get manager statistics
    pub async fn get_stats(&self) -> ManagerStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }
    
}