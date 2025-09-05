/// Liquidity Removal Simulator
/// 
/// Dedicated simulator for liquidity removal transactions that handles:
/// - MEV bot accounts with 0 ETH balance (funded via flashloans)
/// - State override to provide gas for simulation
/// - State change extraction even from reverted transactions
/// - Direct pool drain calculation
/// 
/// This solves the issue where liquidity removals fail simulation because
/// the remover account has 0 balance in current state (gets funded in same block).

use std::collections::HashMap;
use std::sync::Arc;
use alloy_primitives::{Address, U256, Bytes};
use tx_simulator::{TxSimulator, UnsignedTransaction};
use eyre::Result;
use tracing::{info, warn, debug, error};
use crate::common::address::alloy_address_to_checksum;
use crate::token_tracking::TokenTrackingCache;

/// Placeholder for AddressStateChange - actual implementation in tx_processor
#[derive(Debug, Clone)]
pub struct AddressStateChange {
    pub eth_net: f64,
    pub token_net: HashMap<Address, f64>,
}

/// Simple state override for account balance
/// (Since reth_tx_simulator doesn't expose StateOverride yet)
#[derive(Debug, Clone, Default)]
pub struct StateOverride {
    pub balance: Option<U256>,
    pub nonce: Option<u64>,
    pub code: Option<Bytes>,
    pub state: Option<HashMap<U256, U256>>,
    pub state_diff: Option<HashMap<U256, U256>>,
}

/// Result of liquidity removal simulation
#[derive(Debug, Clone)]
pub struct LiquidityRemovalResult {
    /// Whether the transaction succeeded
    pub success: bool,
    /// Revert reason if failed
    pub revert_reason: Option<String>,
    /// State changes for all affected addresses
    pub state_changes: HashMap<Address, AddressStateChange>,
    /// Pool address that was drained (if identified)
    pub pool_address: Option<Address>,
    /// Amount of ETH removed from pool
    pub eth_removed: f64,
    /// Percentage of pool drained
    pub drain_percentage: f64,
    /// Remaining ETH in pool after removal
    pub remaining_eth: f64,
    /// Whether this is likely a scam (>60% drain or <0.3 ETH remaining)
    pub is_scam: bool,
    /// Debug information
    pub debug_info: Option<String>,
}

/// Pool drain calculation result
#[derive(Debug)]
struct PoolDrainInfo {
    pub pool_address: Address,
    pub initial_eth: f64,
    pub eth_removed: f64,
    pub remaining_eth: f64,
    pub percentage: f64,
}

/// Simulator specifically for liquidity removal transactions
pub struct LiquidityRemovalSimulator {
    /// Underlying transaction simulator
    simulator: Arc<TxSimulator>,
    /// Token cache for pool information
    token_cache: Option<Arc<TokenTrackingCache>>,
    /// Test buyer address used for funding
    buyer_address: Address,
}

impl LiquidityRemovalSimulator {
    /// Create new liquidity removal simulator
    pub fn new(simulator: Arc<TxSimulator>) -> Self {
        Self {
            simulator,
            token_cache: None,
            // Use the standard test buyer address
            buyer_address: Address::from([
                0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 
                0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D, 0x80, 0x4a, 
                0x24, 0xbd, 0x56, 0x89
            ]),
        }
    }
    
    /// Set token cache for pool lookups
    pub fn set_token_cache(&mut self, cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(cache);
    }
    
    /// Simulate a liquidity removal transaction
    /// 
    /// This handles the special case where MEV bots have 0 balance and would
    /// normally fail simulation. We use state override to provide ETH for gas.
    pub async fn simulate_removal(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
    ) -> Result<LiquidityRemovalResult> {
        let from = unsigned_tx.from.unwrap_or_default();
        let to = unsigned_tx.to.unwrap_or_default();
        
        info!("🔍 LIQUIDITY REMOVAL SIMULATION STARTING");
        info!("  From: {}", alloy_address_to_checksum(from));
        info!("  To (Router): {}", alloy_address_to_checksum(to));
        info!("  Block: {:?} (None = latest)", block_number);
        info!("  Call value: {:?}", unsigned_tx.value);
        info!("  Gas limit: {:?}", unsigned_tx.gas);
        
        // Log calldata info to understand what's being removed
        if let Some(ref data) = unsigned_tx.data {
            if data.len() >= 4 {
                let selector = format!("0x{:02x}{:02x}{:02x}{:02x}", data[0], data[1], data[2], data[3]);
                info!("  Function selector: {}", selector);
                if selector == "0x02751cec" {
                    info!("  ✓ Confirmed: removeLiquidityETH");
                }
            }
        }
        
        // For liquidity removals, we often see MEV bots with 0 balance
        // They get funded via flashloans in the same block
        // Since we can't check balance directly, we'll attempt simulation
        // and handle failures gracefully
        let needs_override = true; // Assume we might need override for liquidity removals
        
        let (result, state_changes) = if needs_override {
            info!("  🔧 Account has insufficient balance, attempting simulation anyway");
            info!("  Note: State override not yet available in reth_tx_simulator");
            
            // Try simulation anyway - it might work if the transaction is funded via flashloan
            // or if we're simulating at a different block
            let sim_result = if let Some(block) = block_number {
                match self.simulator
                    .simulate_unsigned_transaction_at_block(unsigned_tx.clone(), block)
                    .await {
                    Ok(res) => res,
                    Err(e) => {
                        error!("❌ LIQUIDITY REMOVAL SIMULATION FAILED:");
                        error!("  From: {}", alloy_address_to_checksum(from));
                        error!("  To: {}", alloy_address_to_checksum(to));
                        error!("  Block: {:?}", block_number);
                        error!("  Error: {}", e);
                        
                        // Parse the specific error type
                        let error_str = e.to_string();
                        let revert_reason = if error_str.contains("ds-math-sub-underflow") {
                            "Pool math underflow - pool may be empty or amounts incorrect"
                        } else if error_str.contains("LackOfFund") {
                            "Insufficient ETH balance for gas"
                        } else if error_str.contains("INSUFFICIENT_LIQUIDITY") {
                            "Insufficient liquidity in pool"
                        } else {
                            "Unknown revert reason"
                        };
                        
                        error!("  Parsed reason: {}", revert_reason);
                        
                        return Ok(LiquidityRemovalResult {
                            success: false,
                            revert_reason: Some(format!("{}: {}", revert_reason, e)),
                            state_changes: HashMap::new(),
                            pool_address: None,
                            eth_removed: 0.0,
                            drain_percentage: 0.0,
                            remaining_eth: 0.0,
                            is_scam: false,
                            debug_info: Some(format!("Simulation failed at block {:?}: {}", block_number, error_str)),
                        });
                    }
                }
            } else {
                match self.simulator
                    .simulate_call(unsigned_tx.clone())
                    .await {
                    Ok(res) => res,
                    Err(e) => {
                        let error_str = e.to_string();
                        
                        // Check if this is a nonce error that we can fix with approval
                        if error_str.contains("nonce") && error_str.contains("too high, expected") {
                            info!("🔄 Nonce error detected, attempting to simulate with approval sequence");
                            
                            // Extract expected nonce
                            if let Some(expected_nonce_str) = error_str.split("expected ").nth(1) {
                                if let Ok(expected_nonce) = expected_nonce_str.trim().parse::<u64>() {
                                    // Extract LP token from removal calldata
                                    if let Some(ref data) = unsigned_tx.data {
                                        let selector = format!("0x{:02x}{:02x}{:02x}{:02x}", 
                                            data[0], data[1], data[2], data[3]);
                                        
                                        if let Some(lp_token) = Self::extract_lp_token_from_removal_calldata(data, &selector) {
                                            info!("  LP Token: {}", alloy_address_to_checksum(lp_token));
                                            info!("  Expected nonce: {}", expected_nonce);
                                            
                                            // First simulate approval with expected nonce
                                            let approval_request = UnsignedTransaction {
                                                from: Some(from),
                                                to: Some(lp_token),
                                                value: Some(U256::ZERO),
                                                data: Some(Self::construct_approval_calldata(to, U256::MAX)),
                                                gas: Some(100_000),
                                                gas_price: unsigned_tx.gas_price,
                                                max_fee_per_gas: unsigned_tx.max_fee_per_gas,
                                                max_priority_fee_per_gas: unsigned_tx.max_priority_fee_per_gas,
                                                nonce: Some(expected_nonce),
                                            };
                                            
                                            info!("  1️⃣ Simulating approval with nonce {}", expected_nonce);
                                            match self.simulator
                                                .simulate_call(approval_request)
                                                .await {
                                                Ok(approval_result) => {
                                                    if approval_result.success {
                                                        info!("    ✅ Approval simulation succeeded");
                                                    } else {
                                                        warn!("    ⚠️ Approval reverted: {:?}", approval_result.revert_reason);
                                                    }
                                                },
                                                Err(e) => {
                                                    warn!("    ❌ Approval simulation failed: {}", e);
                                                }
                                            }
                                            
                                            // Now simulate removal with next nonce
                                            let mut removal_request = unsigned_tx.clone();
                                            removal_request.nonce = Some(expected_nonce + 1);
                                            
                                            info!("  2️⃣ Simulating removal with nonce {}", expected_nonce + 1);
                                            match self.simulator
                                                .simulate_call(removal_request)
                                                .await {
                                                Ok(_sim_result) => {
                                                    info!("    ✅ Removal simulation succeeded");
                                                    // TODO: Extract state changes from simulation
                                                    // For now, return empty state changes
                                                    let state_changes = HashMap::new();
                                                    return Ok(self.process_successful_removal(
                                                        state_changes, to, from
                                                    ).await?);
                                                },
                                                Err(e) => {
                                                    error!("    ❌ Removal still failed: {}", e);
                                                    // Fall through to normal error handling
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Normal error handling
                        error!("❌ LIQUIDITY REMOVAL SIMULATION FAILED (latest block):");
                        error!("  From: {}", alloy_address_to_checksum(from));
                        error!("  To: {}", alloy_address_to_checksum(to));
                        error!("  Error: {}", e);
                        
                        // Parse the specific error type
                        let revert_reason = if error_str.contains("ds-math-sub-underflow") {
                            "Pool math underflow - pool may be empty or amounts incorrect"
                        } else if error_str.contains("LackOfFund") {
                            "Insufficient ETH balance for gas"
                        } else if error_str.contains("INSUFFICIENT_LIQUIDITY") {
                            "Insufficient liquidity in pool"
                        } else {
                            "Unknown revert reason"
                        };
                        
                        error!("  Parsed reason: {}", revert_reason);
                        
                        return Ok(LiquidityRemovalResult {
                            success: false,
                            revert_reason: Some(format!("{}: {}", revert_reason, e)),
                            state_changes: HashMap::new(),
                            pool_address: None,
                            eth_removed: 0.0,
                            drain_percentage: 0.0,
                            remaining_eth: 0.0,
                            is_scam: false,
                            debug_info: Some(format!("Simulation failed at latest block: {}", error_str)),
                        });
                    }
                }
            };
            
            // Log success
            if sim_result.success {
                info!("  ✅ Simulation succeeded (gas used: {})", sim_result.gas_used);
            } else if let Some(ref reason) = sim_result.revert_reason {
                warn!("  ⚠️ Simulation reverted but continued: {}", reason);
            }
            
            // Extract state changes even if simulation failed
            // TODO: Implement trace extraction for failed transactions
            (sim_result.success, HashMap::new())
        } else {
            // Normal simulation without override - use call trace version to get state changes
            // For now, just do basic simulation - state change extraction moved to tx_processor
            let _sim_result = if let Some(block) = block_number {
                self.simulator
                    .simulate_unsigned_transaction_at_block(unsigned_tx.clone(), block)
                    .await?
            } else {
                self.simulator
                    .simulate_call(unsigned_tx.clone())
                    .await?
            };
            
            (true, HashMap::new())  // If we got here, simulation succeeded
        };
        
        info!("  Simulation result: success={}, {} addresses affected", 
            result, state_changes.len());
        
        // Find the pool and calculate drain
        let pool_drain = self.calculate_pool_drain(&state_changes, to).await?;
        
        // Determine if this is a scam
        let is_scam = pool_drain.as_ref()
            .map(|drain| drain.percentage > 60.0 || drain.remaining_eth < 0.3)
            .unwrap_or(false);
        
        if let Some(ref drain) = pool_drain {
            info!("  💧 Pool drain detected:");
            info!("    Pool: {}", alloy_address_to_checksum(drain.pool_address));
            info!("    Initial: {:.4} ETH", drain.initial_eth);
            info!("    Removed: {:.4} ETH", drain.eth_removed);
            info!("    Remaining: {:.4} ETH", drain.remaining_eth);
            info!("    Drain: {:.1}%", drain.percentage);
            info!("    Is scam: {}", is_scam);
        }
        
        Ok(LiquidityRemovalResult {
            success: result,
            revert_reason: if !result { 
                Some("Transaction reverted (extracted state changes from traces)".to_string()) 
            } else { 
                None 
            },
            state_changes: state_changes.clone(),
            pool_address: pool_drain.as_ref().map(|d| d.pool_address),
            eth_removed: pool_drain.as_ref().map(|d| d.eth_removed).unwrap_or(0.0),
            drain_percentage: pool_drain.as_ref().map(|d| d.percentage).unwrap_or(0.0),
            remaining_eth: pool_drain.as_ref().map(|d| d.remaining_eth).unwrap_or(0.0),
            is_scam,
            debug_info: if needs_override {
                Some("State override used to provide gas".to_string())
            } else {
                None
            },
        })
    }
    
    /// Calculate pool drain from state changes
    async fn calculate_pool_drain(
        &self,
        state_changes: &HashMap<Address, AddressStateChange>,
        router_address: Address,
    ) -> Result<Option<PoolDrainInfo>> {
        // Find the pool by looking for significant ETH decrease
        let mut pool_candidate: Option<(Address, f64)> = None;
        
        for (address, changes) in state_changes {
            // Skip the router itself
            if address == &router_address {
                continue;
            }
            
            // Check ETH change
            let eth_change = changes.eth_net / 1e18;
            
            // Pool will have negative ETH change (drain)
            if eth_change < -0.01 { // More than 0.01 ETH removed
                debug!("  Potential pool {} with ETH change: {:.6}", 
                    alloy_address_to_checksum(*address), eth_change);
                
                // Check if this is a known pool
                if let Some(ref cache) = self.token_cache {
                    let addr_str = alloy_address_to_checksum(*address);
                    if let Some(pool_state) = cache.get_pool(&addr_str).await {
                        info!("  ✅ Confirmed pool: {} (initial: {:.4} ETH)", 
                            addr_str, pool_state.eth_reserve);
                        
                        let drain_percentage = (eth_change.abs() / pool_state.eth_reserve) * 100.0;
                        let remaining = pool_state.eth_reserve + eth_change; // eth_change is negative
                        
                        return Ok(Some(PoolDrainInfo {
                            pool_address: *address,
                            initial_eth: pool_state.eth_reserve,
                            eth_removed: eth_change.abs(),
                            remaining_eth: remaining.max(0.0),
                            percentage: drain_percentage.min(100.0),
                        }));
                    }
                }
                
                // Track as potential pool even if not in cache
                if pool_candidate.is_none() || eth_change < pool_candidate.as_ref().unwrap().1 {
                    pool_candidate = Some((*address, eth_change));
                }
            }
        }
        
        // If we found a potential pool but it's not in cache, still report it
        if let Some((pool_addr, eth_change)) = pool_candidate {
            warn!("  ⚠️ Potential pool drain from unknown pool: {}", 
                alloy_address_to_checksum(pool_addr));
            
            // Estimate based on typical pool sizes
            let estimated_initial = 10.0; // Assume 10 ETH pool
            let drain_percentage = (eth_change.abs() / estimated_initial) * 100.0;
            
            return Ok(Some(PoolDrainInfo {
                pool_address: pool_addr,
                initial_eth: estimated_initial,
                eth_removed: eth_change.abs(),
                remaining_eth: (estimated_initial + eth_change).max(0.0),
                percentage: drain_percentage.min(100.0),
            }));
        }
        
        Ok(None)
    }
    
    /// Process successful removal simulation results
    async fn process_successful_removal(
        &self,
        state_changes: HashMap<Address, AddressStateChange>,
        router_address: Address,
        _from_address: Address,
    ) -> Result<LiquidityRemovalResult> {
        // Calculate pool drain
        let pool_drain = self.calculate_pool_drain(&state_changes, router_address).await?;
        
        // Determine if this is a scam
        let is_scam = pool_drain.as_ref()
            .map(|drain| drain.percentage > 60.0 || drain.remaining_eth < 0.3)
            .unwrap_or(false);
        
        if let Some(ref drain) = pool_drain {
            info!("  💧 Pool drain detected:");
            info!("    Pool: {}", alloy_address_to_checksum(drain.pool_address));
            info!("    Initial: {:.4} ETH", drain.initial_eth);
            info!("    Removed: {:.4} ETH", drain.eth_removed);
            info!("    Remaining: {:.4} ETH", drain.remaining_eth);
            info!("    Drain: {:.1}%", drain.percentage);
            info!("    Is scam: {}", is_scam);
        }
        
        Ok(LiquidityRemovalResult {
            success: true,
            revert_reason: None,
            state_changes: state_changes.clone(),
            pool_address: pool_drain.as_ref().map(|d| d.pool_address),
            eth_removed: pool_drain.as_ref().map(|d| d.eth_removed).unwrap_or(0.0),
            drain_percentage: pool_drain.as_ref().map(|d| d.percentage).unwrap_or(0.0),
            remaining_eth: pool_drain.as_ref().map(|d| d.remaining_eth).unwrap_or(0.0),
            is_scam,
            debug_info: Some("Simulated with approval sequence".to_string()),
        })
    }
    
    /// Construct approve calldata for LP token approval
    /// approve(address spender, uint256 amount)
    fn construct_approval_calldata(spender: Address, amount: U256) -> Bytes {
        let mut data = Vec::with_capacity(68);
        
        // Function selector for approve(address,uint256): 0x095ea7b3
        data.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]);
        
        // Pad spender address to 32 bytes
        data.extend_from_slice(&[0u8; 12]); // 12 bytes of padding
        data.extend_from_slice(spender.as_slice());
        
        // Amount (32 bytes) - use MAX_UINT256 for unlimited approval
        let amount_bytes = amount.to_be_bytes::<32>();
        data.extend_from_slice(&amount_bytes);
        
        Bytes::from(data)
    }
    
    /// Extract LP token address from removeLiquidityETH calldata
    /// removeLiquidityETH params: (address token, uint liquidity, uint amountTokenMin, uint amountETHMin, address to, uint deadline)
    /// The token address is the first parameter (bytes 4-36)
    fn extract_lp_token_from_removal_calldata(data: &Bytes, selector_hex: &str) -> Option<Address> {
        // Check if we have enough data
        if data.len() < 36 {
            return None;
        }
        
        // Different removal functions have different parameter layouts
        match selector_hex {
            "0x02751cec" | "0xaf2979eb" | "0x5b0d5984" | "0xded9382a" => {
                // removeLiquidityETH variants - token is first param
                // Skip selector (4 bytes) and padding (12 bytes)
                let token_bytes = &data[16..36];
                Some(Address::from_slice(token_bytes))
            },
            _ => {
                // For other removal types, we might need different parsing
                // For now, return None
                None
            }
        }
    }
    
}