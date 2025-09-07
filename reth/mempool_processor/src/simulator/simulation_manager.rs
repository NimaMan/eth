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
use crate::tx_router::{TransactionCategory, SimulationPriority, CreatorFunctionType};
use crate::signal_detector::{SignalManager, SignalManagerConfig};
use crate::token_tracking::TokenTrackingCache;
use tokio::sync::Mutex as TokioMutex;
use super::{SimulationQueue, MempoolSimulator, LiquidityRemovalSimulator, LiquidityRemovalResult};
use tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::types::PoolViabilityResult;
use std::collections::HashMap;
use alloy_primitives::U256;

// AddressStateChange is now AddressBalanceChange in tx_processor
use tx_processor::tx_processor::data_models::AddressBalanceChange as AddressStateChange;

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
    pub pool_viability_result: Option<PoolViabilityResult>,
    pub error: Option<String>,
    pub simulation_time_ms: f64,
    // Addresses needed for compatibility
    pub token_address: Option<alloy_primitives::Address>,
    pub pool_address: Option<alloy_primitives::Address>,
    pub pool_type: Option<String>,  // Pool type (V2, V3, V4)
    // Debug info for error analysis
    pub debug_info: Option<String>,
    // Liquidity removal result (only populated for liquidity removal transactions)
    pub liquidity_removal_result: Option<LiquidityRemovalResult>,
}

impl SimulationResult {
    /// Get buy_sell_result for backward compatibility
    pub fn buy_sell_result(&self) -> Option<BuySellResult> {
        self.pool_viability_result.as_ref().map(BuySellResult::from)
    }
}

/// Buy/sell simulation result - now using PoolViabilityResult
/// This struct is kept for backward compatibility but delegates to PoolViabilityResult
#[derive(Debug, Clone)]
pub struct BuySellResult {
    pub can_buy: bool,
    pub can_sell: bool,
    pub buy_tax: Option<f64>,      // 0-100% or None if calculation failed
    pub sell_tax: Option<f64>,     // 0-100% or None if calculation failed
    pub buy_tax_error: Option<String>,   // Error message if buy tax calculation failed
    pub sell_tax_error: Option<String>,  // Error message if sell tax calculation failed
}

impl From<&PoolViabilityResult> for BuySellResult {
    fn from(result: &PoolViabilityResult) -> Self {
        Self {
            can_buy: result.can_buy,
            can_sell: result.can_sell,
            buy_tax: if result.buy_tax_percent >= 0.0 {
                Some(result.buy_tax_percent)
            } else {
                None
            },
            sell_tax: if result.sell_tax_percent >= 0.0 {
                Some(result.sell_tax_percent)
            } else {
                None
            },
            buy_tax_error: if result.buy_tax_percent < 0.0 {
                Some("Failed to calculate buy tax".to_string())
            } else {
                None
            },
            sell_tax_error: if result.sell_tax_percent < 0.0 {
                Some("Failed to calculate sell tax".to_string())
            } else {
                None
            },
        }
    }
}

/// Manager for transaction simulations
pub struct SimulationManager {
    mempool_simulator: Arc<MempoolSimulator>,
    liquidity_removal_simulator: Arc<super::LiquidityRemovalSimulator>,
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
        mempool_simulator: Arc<MempoolSimulator>,
        token_cache: Arc<TokenTrackingCache>,
        signal_config: SignalManagerConfig,
        publisher: Arc<TokioMutex<crate::signal_publisher::SignalPublisher>>,
        max_concurrent: usize,
    ) -> Self {
        let mut signal_manager = SignalManager::new(signal_config);
        signal_manager.set_token_cache(token_cache.clone());
        signal_manager.set_publisher(publisher);
        
        // Create liquidity removal simulator with the same underlying simulator
        let mut liquidity_removal_simulator = super::LiquidityRemovalSimulator::new(
            mempool_simulator.get_tx_simulator()
        );
        liquidity_removal_simulator.set_token_cache(token_cache.clone());
        
        Self {
            mempool_simulator,
            liquidity_removal_simulator: Arc::new(liquidity_removal_simulator),
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
            
            if result.buy_sell_result().is_some() {
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
            pool_viability_result: None,
            error: None,
            simulation_time_ms: 0.0,
            token_address: None,
            pool_address: None,
            pool_type: None,
            debug_info: None,
            liquidity_removal_result: None,
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
            TransactionCategory::CreatorTransaction { function_type, .. } => {
                // Check if this is a liquidity removal - handle specially
                let all_results = if matches!(function_type, CreatorFunctionType::LiquidityRemoval) {
                    info!("💧 Detected liquidity removal transaction, using dedicated simulator");
                    // Use dedicated liquidity removal simulator
                    self.simulate_liquidity_removal(&request).await
                } else {
                    // CRITICAL: Run simulation for EACH pool independently
                    // Each pool will generate its own signal
                    self.simulate_tx_with_buy_sell_all_pools(&request).await
                };
                
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
                    for (pool_idx, pool_specific_result) in all_results.into_iter().enumerate() {
                        // Debug logging for liquidity removal transactions
                        if matches!(pool_specific_result.request.category, TransactionCategory::CreatorTransaction { function_type: CreatorFunctionType::LiquidityRemoval, .. }) {
                            if let Some(ref removal_result) = pool_specific_result.liquidity_removal_result {
                                info!("  [DEBUG] Liquidity removal TX {} pool {} has removal result",
                                    pool_specific_result.request.tx.hash,
                                    pool_idx
                                );
                            }
                        }
                        
                        // Send each pool's results to signal manager separately
                        info!("📤 Sending pool-specific result to signal manager for TX {} (pool {})", 
                            pool_specific_result.request.tx.hash, pool_idx);
                        info!("  Pool address: {:?}", pool_specific_result.pool_address);
                        info!("  Result has error: {}, has buy_sell: {}", 
                            pool_specific_result.error.is_some(), 
                            pool_specific_result.pool_viability_result.is_some()
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
                    result.buy_sell_result().is_some()
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
    async fn simulate_tx_with_buy_sell_all_pools(&self, request: &SimulationRequest) -> Vec<SimulationResult> {
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
                            return vec![SimulationResult {
                                request: request.clone(),
                                pool_viability_result: None,
                                liquidity_removal_result: None,
                                error: Some(format!("No token address found for creator {}", creator)),
                                token_address: None,
                                pool_address: None,
                                pool_type: None,
                                debug_info: None,
                                simulation_time_ms: 0.0,
                            }];
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
                    return vec![SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        liquidity_removal_result: None,
                        error: Some("Contract creation: address calculation not implemented yet".to_string()),
                        token_address: None,
                        pool_address: None,
                        pool_type: None,
                        debug_info: None,
                        simulation_time_ms: 0.0,
                    }];
                } else {
                    contract_address.clone()
                }
            }
            _ => return vec![SimulationResult {
                request: request.clone(),
                pool_viability_result: None,
                liquidity_removal_result: None,
                error: Some("Category doesn't support buy/sell simulation".to_string()),
                token_address: None,
                pool_address: None,
                pool_type: None,
                debug_info: None,
                simulation_time_ms: 0.0,
            }],
        };
        
        // Convert string addresses to alloy Address type
        let token_address = match token_address_str.trim_start_matches("0x")
            .parse::<alloy_primitives::Address>() {
            Ok(addr) => addr,
            Err(e) => return vec![SimulationResult {
                request: request.clone(),
                pool_viability_result: None,
                liquidity_removal_result: None,
                error: Some(format!("Invalid token address: {}", e)),
                token_address: None,
                pool_address: None,
                pool_type: None,
                debug_info: None,
                simulation_time_ms: 0.0,
            }],
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
        let mut results: Vec<SimulationResult> = Vec::new();
        
        // CRITICAL LOOP: Simulate each pool INDEPENDENTLY
        // Each iteration produces a separate result for signal generation
        for (pool_idx, pool_state) in v2_pools.into_iter().enumerate() {
            let pool_address = match pool_state.address.trim_start_matches("0x")
                .parse::<alloy_primitives::Address>() {
                Ok(addr) => addr,
                Err(e) => {
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(format!("Invalid pool address {}: {}", pool_state.address, e)),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: None,
                        pool_type: Some("V2".to_string()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                    continue;
                }
            };
            
            let pool_type = format!("{:?}", pool_state.pool_type);
            info!("  [Pool {}] Simulating pool: {:?} (Type: {}, ETH: {:.6})", 
                pool_idx, pool_address, &pool_type, pool_state.eth_reserve);
        
            // Get current block number (simulate at latest)
            let block_number = None; // Use latest block
            info!("  Using block number: {:?}", block_number);
            
            // Use the MempoolTransaction directly (no need to convert to FullTransaction)
            let mut tx_call_request = match crate::common::convert::ipc_to_call_request(&request.tx.data) {
                Ok(req) => req,
                Err(e) => {
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(format!("Failed to convert transaction: {}", e)),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: Some(pool_address),
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
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
            
            // Log first 4 bytes of calldata to verify function selector
            if let Some(ref data) = tx_call_request.data {
                if data.len() >= 4 {
                    let selector = format!("0x{:02x}{:02x}{:02x}{:02x}", data[0], data[1], data[2], data[3]);
                    info!("    function selector: {}", selector);
                    if selector == "0x02751cec" {
                        info!("    ✓ This is removeLiquidityETH!");
                    }
                }
            }
            
            // Check if gas is missing - this should never happen for mined transactions
            if tx_call_request.gas.is_none() {
                error!("WARNING: Gas limit is None for mined transaction!");
                error!("Raw IPC data: {}", serde_json::to_string_pretty(&request.tx.data).unwrap_or_default());
                results.push(SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    error: Some("Gas limit missing from transaction - parsing error".to_string()),
                    simulation_time_ms: 0.0,
                    token_address: Some(token_address),
                    pool_address: Some(pool_address),
                    pool_type: Some(pool_type.clone()),
                    debug_info: None,
                    liquidity_removal_result: None,
                });
                continue;
            }
            
            // Remove nonce to let the sequential simulator manage it automatically
            tx_call_request.nonce = None;
            
            // Store original gas prices for retry logic
            let original_gas_price = tx_call_request.gas_price;
            let original_max_fee = tx_call_request.max_fee_per_gas;
            
            info!("  [Pool {}] Using original gas prices from transaction", pool_idx);
            info!("    Block: {:?} (None = latest)", block_number);
            
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
            // Create pool viability config
            let pool_type_enum = match pool_type.as_str() {
                "V2" => tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::types::PoolType::UniswapV2,
                "V3" => tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::types::PoolType::UniswapV3 { fee_tier: 3000 }, // Default to 0.3% fee
                _ => tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::types::PoolType::UniswapV2,
            };
            
            let config = tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::config::PoolViabilityConfig {
                token_address,
                pool_address,
                pool_type: pool_type_enum,
                test_amount: U256::from(10_000_000_000_000_000u64), // 0.01 ETH
                buyer_address: tx_call_request.from.unwrap_or_default(),
                block_number,
                gas_limit: tx_call_request.gas.unwrap_or(500_000) as u64,
                gas_price: tx_call_request.gas_price.unwrap_or(30_000_000_000) as u128,
                prior_tx: None, // TODO: Convert tx_call_request to ProcessedTransaction
                block_delay: 0,
                slippage_tolerance: 0.5, // 0.5% default slippage
                weth_address: alloy_primitives::Address::from([
                    0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D,
                    0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08,
                    0x3C, 0x75, 0x6C, 0xc2
                ]), // Mainnet WETH
                token_decimals: 18, // Default to 18 decimals
            };
            
            let simulation_result = match self.mempool_simulator.simulate_pool_buy_sell(config.clone()).await {
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
                        let retry_config = tx_processor::simulator::erc20_token_buy_approve_sell_tx_simulator::config::PoolViabilityConfig {
                            token_address,
                            pool_address,
                            pool_type: pool_type_enum,
                            test_amount: U256::from(10_000_000_000_000_000u64), // 0.01 ETH
                            buyer_address: tx_call_request.from.unwrap_or_default(),
                            block_number,
                            gas_limit: tx_call_request.gas.unwrap_or(500_000) as u64,
                            gas_price: new_gas_price.unwrap_or(90_000_000_000) as u128, // 3x default
                            prior_tx: None, // TODO: Convert tx_call_request to ProcessedTransaction
                            block_delay: 0,
                            slippage_tolerance: 0.5, // 0.5% default slippage
                            weth_address: alloy_primitives::Address::from([
                                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D,
                                0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08,
                                0x3C, 0x75, 0x6C, 0xc2
                            ]), // Mainnet WETH
                            token_decimals: 18, // Default to 18 decimals
                        };
                        
                        self.mempool_simulator.simulate_pool_buy_sell(retry_config).await
                    } else {
                        Err(e)
                    }
                }
            };
            
            // Process the result
            match simulation_result {
                Ok(result) => {
                    info!("  [Pool {}] Buy/sell simulation completed successfully:", pool_idx);
                    info!("    Can Buy: {}", result.can_buy);
                    info!("    Can Sell: {}", result.can_sell);
                    info!("    Is Tradeable: {}", result.is_tradeable);
                    info!("    Buy Tax: {:.2}%", result.buy_tax_percent);
                    info!("    Sell Tax: {:.2}%", result.sell_tax_percent);
                    
                    // Log failure reason if provided
                    if let Some(ref reason) = result.failure_reason {
                        warn!("    Simulation failure: {}", reason);
                        // Try to extract more details from the revert
                        if reason.contains("output:") {
                            if let Some(output_start) = reason.find("output: ") {
                                let output = &reason[output_start + 8..];
                                if let Some(end) = output.find(' ').or_else(|| output.find('}')) {
                                    let hex_output = &output[..end];
                                    warn!("    Revert output hex: {}", hex_output);
                                    if hex_output == "0x" {
                                        warn!("    Empty revert - likely require() without message");
                                    }
                                }
                            }
                        }
                    }
                    
                    // State changes are in buy_transaction.address_balance_changes
                    let tx_state_changes = Some(result.buy_transaction.address_balance_changes.clone());
                    
                    // Debug logging for liquidity removal transactions
                    if matches!(request.category, TransactionCategory::CreatorTransaction { function_type: CreatorFunctionType::LiquidityRemoval, .. }) {
                        info!("  [DEBUG] Liquidity removal TX {} prior_tx present: {}",
                            request.tx.hash,
                            result.prior_transaction.is_some()
                        );
                        
                        // Log state changes from buy transaction
                        info!("  [DEBUG] State changes count: {}",
                            result.buy_transaction.address_balance_changes.len()
                        );
                        
                        if let Some(ref revert_reason) = result.failure_reason {
                            warn!("  [DEBUG] Liquidity removal FAILED: {}", revert_reason);
                        }
                        
                        for (addr, changes) in result.buy_transaction.address_balance_changes.iter().take(3) {
                            let eth_change = changes.currency_net.get("ETH").cloned().unwrap_or(U256::ZERO);
                            info!("  [DEBUG] Address {} ETH change: {:?}", addr, eth_change);
                        }
                    }
                    
                    // No need to create BuySellResult - SimulationResult has buy_sell_result() method
                    
                    // Store the result with all the data we need
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: Some(result),
                        error: None,
                        simulation_time_ms: 0.0, // Will be calculated elsewhere
                        token_address: Some(token_address),
                        pool_address: if pool_address.is_zero() { None } else { Some(pool_address) },
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                }
                Err(e) => {
                    info!("  [Pool {}] ERROR in sequence simulation: {}", pool_idx, e);
                    info!("  Error details: {:?}", e);                    
                    let error_msg = format!("Pool {}: {}", pool_idx, e);
                    results.push(SimulationResult {
                        request: request.clone(),
                        pool_viability_result: None,
                        error: Some(error_msg),
                        simulation_time_ms: 0.0,
                        token_address: Some(token_address),
                        pool_address: Some(pool_address),
                        pool_type: Some(pool_type.clone()),
                        debug_info: None,
                        liquidity_removal_result: None,
                    });
                }
            }
        }  // End of for loop
        
        // Return all simulation results
        results
    }

    /// Get manager statistics
    pub async fn get_stats(&self) -> ManagerStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }
    
    /// Simulate liquidity removal transaction using dedicated simulator
    /// This handles MEV bot accounts with 0 balance by using state override
    async fn simulate_liquidity_removal(&self, request: &SimulationRequest) -> Vec<SimulationResult> {
        info!("💧 Starting liquidity removal simulation for TX {}", request.tx.hash);
        
        // Convert mempool transaction to call request
        let call_request = match crate::common::convert::ipc_to_call_request(&request.tx.data) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to convert transaction to call request: {}", e);
                let mut result = SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    error: Some(format!("Failed to convert transaction: {}", e)),
                    simulation_time_ms: 0.0,
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    liquidity_removal_result: None,
                };
                return vec![result];
            }
        };
        
        // Get current block number (simulate at latest)
        let block_number = None; // Use latest block
        
        let sim_start = std::time::Instant::now();
        
        // Run liquidity removal simulation with state override if needed
        let removal_result = match self.liquidity_removal_simulator
            .simulate_removal(call_request, block_number)
            .await {
            Ok(result) => result,
            Err(e) => {
                error!("Liquidity removal simulation failed: {}", e);
                return vec![SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    liquidity_removal_result: None,
                    error: Some(format!("Simulation failed: {}", e)),
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    simulation_time_ms: 0.0,
                }];
            }
        };
        
        let simulation_time = sim_start.elapsed().as_millis() as f64;
        
        info!("  Simulation complete: success={}, is_scam={}, drain={}%", 
            removal_result.success, 
            removal_result.is_scam,
            removal_result.drain_percentage
        );
        
        // Extract token address from the transaction category
        let token_address = match &request.category {
            TransactionCategory::CreatorTransaction { target_token, creator, .. } => {
                if let Some(token) = target_token {
                    match token.trim_start_matches("0x").parse::<alloy_primitives::Address>() {
                        Ok(addr) => Some(addr),
                        Err(e) => {
                            error!("Invalid token address: {}", e);
                            let result = SimulationResult {
                                request: request.clone(),
                                pool_viability_result: None,
                                error: Some(format!("Invalid token address: {}", e)),
                                simulation_time_ms: simulation_time,
                                token_address: None,
                                pool_address: None,
                                pool_type: None,
                                debug_info: None,
                                liquidity_removal_result: None,
                            };
                            return vec![result];
                        }
                    }
                } else {
                    // Try to get from creator's tokens
                    if let Some(token_info) = self.token_cache.get_token_for_creator(creator).await {
                        match token_info.address.trim_start_matches("0x").parse::<alloy_primitives::Address>() {
                            Ok(addr) => Some(addr),
                            Err(e) => {
                                let result = SimulationResult {
                                    request: request.clone(),
                                    pool_viability_result: None,
                                    error: Some(format!("Invalid token address: {}", e)),
                                    simulation_time_ms: simulation_time,
                                    token_address: None,
                                    pool_address: None,
                                    pool_type: None,
                                    debug_info: None,
                                    liquidity_removal_result: None,
                                };
                                return vec![result];
                            }
                        }
                    } else {
                        let result = SimulationResult {
                            request: request.clone(),
                            pool_viability_result: None,
                            error: Some("No token found for creator".to_string()),
                            simulation_time_ms: simulation_time,
                            token_address: None,
                            pool_address: None,
                            pool_type: None,
                            debug_info: None,
                            liquidity_removal_result: None,
                        };
                        return vec![result];
                    }
                }
            }
            _ => {
                let result = SimulationResult {
                    request: request.clone(),
                    pool_viability_result: None,
                    error: Some("Not a creator transaction".to_string()),
                    simulation_time_ms: simulation_time,
                    token_address: None,
                    pool_address: None,
                    pool_type: None,
                    debug_info: None,
                    liquidity_removal_result: None,
                };
                return vec![result];
            }
        };
        
        // Create and return SimulationResult for liquidity removal
        let result = SimulationResult {
            request: request.clone(),
            pool_viability_result: None,  // No buy/sell testing for liquidity removal
            error: None,
            simulation_time_ms: simulation_time,
            token_address,
            pool_address: removal_result.pool_address,
            pool_type: Some("V2".to_string()),
            debug_info: None,
            liquidity_removal_result: Some(removal_result),  // Include the liquidity removal result
        };
        
        vec![result]
    }
    
}
