use eyre::Result;
use std::time::Instant;
use std::sync::Arc;
use std::collections::HashMap;
use alloy_primitives::{Address, Bytes, U256, I256};
use reth_tx_simulator::{RethTxSimulator, CallRequest, AddressStateChange};
use crate::common::address::alloy_address_to_checksum;
use reth_tx_simulator::SequentialSimulationOptions;

/// Result of a single transaction in the sequence
#[derive(Debug, Clone)]
pub struct TransactionSimulationResult {
    /// Whether the transaction succeeded
    pub success: bool,
    /// Gas used by the transaction
    pub gas_used: u64,
    /// Revert reason if transaction failed
    pub revert_reason: Option<String>,
    /// State changes for all affected addresses
    pub state_changes: HashMap<Address, AddressStateChange>,
}

/// Result of a buy/sell simulation sequence
#[derive(Debug, Clone)]
pub struct SequenceSimulationResult {
    /// Result of the given transaction (if provided)
    pub given_tx_result: Option<TransactionSimulationResult>,
    /// Result of the buy transaction
    pub buy_result: TransactionSimulationResult,
    /// Result of the approve transaction (not usually needed by detectors)
    pub approve_result: TransactionSimulationResult,
    /// Result of the sell transaction
    pub sell_result: TransactionSimulationResult,
    /// Total simulation time in milliseconds
    pub simulation_time_ms: f64,
    /// Block number used for simulation
    pub block_number: u64,
}

/// Configuration for buy/sell simulation
#[derive(Debug, Clone)]
pub struct BuySellSimulatorConfig {
    /// Amount of ETH to use for test buy (default: 0.1 ETH)
    pub test_buy_amount: U256,
    /// Router address for swaps (default: Uniswap V2)
    pub router_address: Address,
    /// WETH address (default: mainnet WETH)
    pub weth_address: Address,
    /// Gas limit for transactions
    pub gas_limit: u64,
    /// Gas price in wei
    pub gas_price: u128,
    /// Buyer address for simulations
    pub buyer_address: Address,
}

impl Default for BuySellSimulatorConfig {
    fn default() -> Self {
        Self {
            test_buy_amount: U256::from(10_000_000_000_000_000u64), // 0.01 ETH (reduced from 0.1)
            router_address: Address::from([0x7a, 0x25, 0x0d, 0x56, 0x30, 0xB4, 0xcF, 0x53, 0x97, 0x39, 0xdF, 0x2C, 0x5d, 0xAc, 0xb4, 0xc6, 0x59, 0xF2, 0x48, 0x8D]), // Uniswap V2
            weth_address: Address::from([0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2]), // WETH
            gas_limit: 300_000,
            gas_price: 20_000_000_000u128, // 20 gwei
            buyer_address: Address::from([0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D, 0x80, 0x4a, 0x24, 0xbd, 0x56, 0x89]), // Fixed test address
        }
    }
}

/// Sequential buy/sell simulator using transaction simulation
pub struct SequentialBuySellSimulator {
    simulator: Arc<RethTxSimulator>,
    config: BuySellSimulatorConfig,
    /// Optional token cache for faster lookups
    token_cache: Option<Arc<crate::token_tracking::TokenTrackingCache>>,
}

impl SequentialBuySellSimulator {
    /// Create a new buy/sell simulator
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let simulator = RethTxSimulator::new(reth_datadir)?;
        Ok(Self {
            simulator: Arc::new(simulator),
            config: BuySellSimulatorConfig::default(),
            token_cache: None,
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(reth_datadir: &str, config: BuySellSimulatorConfig) -> Result<Self> {
        let simulator = RethTxSimulator::new(reth_datadir)?;
        Ok(Self {
            simulator: Arc::new(simulator),
            config,
            token_cache: None,
        })
    }
    
    /// Create with an existing simulator instance (avoids database lock conflicts)
    pub fn with_existing_simulator(simulator: Arc<RethTxSimulator>, config: BuySellSimulatorConfig) -> Self {
        Self {
            simulator,
            config,
            token_cache: None,
        }
    }
    
    /// Set token cache for optimized lookups
    pub fn set_token_cache(&mut self, cache: Arc<crate::token_tracking::TokenTrackingCache>) {
        self.token_cache = Some(cache);
    }
    
    /// Get the buyer address used for simulations
    pub fn get_buyer_address(&self) -> Address {
        self.config.buyer_address
    }
    
    /// Simulate a buy/sell sequence for a token
    /// Returns raw simulation results with state changes
    pub async fn simulate_sequence(
        &self,
        token_address: Address,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<SequenceSimulationResult> {
        // Default to V2 for backward compatibility
        self.simulate_sequence_with_pool_type(token_address, pool_address, "V2", block_number).await
    }
    
    /// Simulate a buy/sell sequence with specific pool type
    pub async fn simulate_sequence_with_pool_type(
        &self,
        token_address: Address,
        pool_address: Address,
        pool_type: &str,
        block_number: Option<u64>,
    ) -> Result<SequenceSimulationResult> {
        self.simulate_sequence_with_tx_and_pool_type(None, token_address, pool_address, pool_type, block_number).await
    }
    
    /// Simulate a sequence with an optional given transaction first
    /// Sequence: [Given TX] → Buy → Approve → Sell
    pub async fn simulate_sequence_with_tx(
        &self,
        given_tx: Option<CallRequest>,
        token_address: Address,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<SequenceSimulationResult> {
        // Default to V2 for backward compatibility
        self.simulate_sequence_with_tx_and_pool_type(given_tx, token_address, pool_address, "V2", block_number).await
    }
    
    /// Simulate a sequence with an optional given transaction first and specific pool type
    /// Sequence: [Given TX] → Buy → Approve → Sell
    pub async fn simulate_sequence_with_tx_and_pool_type(
        &self,
        given_tx: Option<CallRequest>,
        token_address: Address,
        pool_address: Address,
        pool_type: &str,
        block_number: Option<u64>,
    ) -> Result<SequenceSimulationResult> {
        let start_time = Instant::now();
        
        // Select router based on pool type
        let router_address = self.get_router_for_pool_type(pool_type);
        
        // Build buy transaction based on pool type
        let buy_calldata = match pool_type {
            "V3" | "V4" => {
                // For V3/V4, we still use V2 router for now as V3 requires more complex encoding
                // TODO: Implement proper V3 exactInputSingle encoding
                self.encode_swap_exact_eth_for_tokens(
                    U256::ZERO, // min tokens out
                    vec![self.config.weth_address, token_address],
                    self.config.buyer_address,
                    U256::from(9999999999u64),
                )
            },
            _ => {
                // V2 or unknown types use V2 method
                self.encode_swap_exact_eth_for_tokens(
                    U256::ZERO, // min tokens out
                    vec![self.config.weth_address, token_address],
                    self.config.buyer_address,
                    U256::from(9999999999u64),
                )
            }
        };
        
        // Try with default amount first, then retry with smaller amount if TRANSFER_FAILED
        let mut buy_amount = self.config.test_buy_amount;
        let mut attempts = 0;
        
        let (initial_result, buy_index) = loop {
            attempts += 1;
            
            let buy_request = CallRequest {
                from: Some(self.config.buyer_address),
                to: Some(router_address),
                value: Some(buy_amount),
                data: Some(buy_calldata.clone()),
                gas: Some(self.config.gas_limit),
                gas_price: Some(self.config.gas_price),
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                nonce: None,
            };
            
            // Build initial sequence with given tx (if any) and buy
            let mut initial_sequence = Vec::new();
            if let Some(tx) = given_tx.clone() {
                initial_sequence.push(tx);
            }
            initial_sequence.push(buy_request.clone());
            
            // Run initial simulation with stop_on_failure: false to get all results
            let result = self.simulator.simulate_transaction_sequence(
                initial_sequence.clone(),
                SequentialSimulationOptions {
                    at_block: block_number,
                    stop_on_failure: false, // Continue even if transactions fail
                    auto_increment_nonces: true,
                    gas_limit_per_tx: None,
                }
            ).await?;
            
            // Check if buy failed with TRANSFER_FAILED and we can retry
            let buy_idx = if given_tx.is_some() { 1 } else { 0 };
            if let Some(buy_result) = result.results.get(buy_idx) {
                if !buy_result.success && attempts == 1 {
                    if let Some(ref reason) = buy_result.revert_reason {
                        if reason.contains("TRANSFER_FAILED") {
                            // Try with smaller amount (0.001 ETH)
                            buy_amount = U256::from(1_000_000_000_000_000u64);
                            continue;
                        }
                    }
                }
            }
            
            // Return the result and buy index
            break (result, buy_idx);
        };
        
        // Check if buy succeeded and extract tokens received
        let (buy_succeeded, tokens_received) = if let Some(buy_result) = initial_result.results.get(buy_index) {
            if buy_result.success {
                let tokens = buy_result.state_changes
                    .get(&self.config.buyer_address)
                    .and_then(|changes| {
                        let token_addr_str = alloy_address_to_checksum(token_address);
                        changes.token_net.get(&token_addr_str).copied()
                    })
                    .unwrap_or(I256::ZERO);
                (!tokens.is_zero(), tokens)
            } else {
                (false, I256::ZERO)
            }
        } else {
            (false, I256::ZERO)
        };
        
        // If buy failed or no tokens received, return partial results
        if !buy_succeeded {
            let given_tx_result = if given_tx.is_some() {
                Some(TransactionSimulationResult {
                    success: initial_result.results[0].success,
                    gas_used: initial_result.results[0].gas_used,
                    revert_reason: initial_result.results[0].revert_reason.clone(),
                    state_changes: initial_result.results[0].state_changes.clone(),
                })
            } else {
                None
            };
            
            let buy_result = if let Some(buy_res) = initial_result.results.get(buy_index) {
                TransactionSimulationResult {
                    success: buy_res.success,
                    gas_used: buy_res.gas_used,
                    revert_reason: buy_res.revert_reason.clone(),
                    state_changes: buy_res.state_changes.clone(),
                }
            } else {
                TransactionSimulationResult {
                    success: false,
                    gas_used: 0,
                    revert_reason: Some("Buy transaction not executed".to_string()),
                    state_changes: HashMap::new(),
                }
            };
            
            // Return with empty approve/sell results
            return Ok(SequenceSimulationResult {
                given_tx_result,
                buy_result,
                approve_result: TransactionSimulationResult {
                    success: false,
                    gas_used: 0,
                    revert_reason: Some("Skipped due to buy failure".to_string()),
                    state_changes: HashMap::new(),
                },
                sell_result: TransactionSimulationResult {
                    success: false,
                    gas_used: 0,
                    revert_reason: Some("Skipped due to buy failure".to_string()),
                    state_changes: HashMap::new(),
                },
                simulation_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
                block_number: block_number.unwrap_or(0), // 0 indicates mempool tx (no block yet)
            });
        }
        
        // Build approve transaction
        // Convert I256 to U256 for approve - tokens received should be positive
        let tokens_received_u256 = if tokens_received >= I256::ZERO {
            tokens_received.unsigned_abs()
        } else {
            return Err(eyre::eyre!("Negative tokens received, this should not happen"));
        };
        let approve_calldata = self.encode_approve(router_address, tokens_received_u256);
        let approve_request = CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(token_address),
            value: Some(U256::ZERO),
            data: Some(approve_calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // Build sell transaction based on pool type
        let sell_calldata = match pool_type {
            "V3" | "V4" => {
                // For V3/V4, we still use V2 router for now as V3 requires more complex encoding
                // TODO: Implement proper V3 exactOutputSingle encoding
                self.encode_swap_exact_tokens_for_eth(
                    tokens_received_u256,
                    U256::ZERO, // min ETH out
                    vec![token_address, self.config.weth_address],
                    self.config.buyer_address,
                    U256::from(9999999999u64),
                )
            },
            _ => {
                // V2 or unknown types use V2 method
                self.encode_swap_exact_tokens_for_eth(
                    tokens_received_u256,
                    U256::ZERO, // min ETH out
                    vec![token_address, self.config.weth_address],
                    self.config.buyer_address,
                    U256::from(9999999999u64),
                )
            }
        };
        
        let sell_request = CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(router_address),
            value: Some(U256::ZERO),
            data: Some(sell_calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // Recreate buy request with successful amount (from the loop above)
        let buy_request = CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(router_address),
            value: Some(buy_amount), // Use the amount that worked
            data: Some(buy_calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // Build full sequence
        let mut full_sequence = Vec::new();
        let has_given_tx = given_tx.is_some();
        if let Some(tx) = given_tx {
            full_sequence.push(tx);
        }
        full_sequence.extend(vec![buy_request, approve_request, sell_request]);
        
        // Run full sequence
        let sequence_result = self.simulator.simulate_transaction_sequence(
            full_sequence,
            SequentialSimulationOptions {
                at_block: block_number,
                stop_on_failure: false, // Don't stop on failure to see all results
                auto_increment_nonces: true,
                gas_limit_per_tx: None,
            }
        ).await?;
        
        // Extract results based on whether we have a given tx
        let (given_tx_result, buy_result, approve_result, sell_result) = if has_given_tx {
            (
                Some(TransactionSimulationResult {
                    success: sequence_result.results[0].success,
                    gas_used: sequence_result.results[0].gas_used,
                    revert_reason: sequence_result.results[0].revert_reason.clone(),
                    state_changes: sequence_result.results[0].state_changes.clone(),
                }),
                TransactionSimulationResult {
                    success: sequence_result.results[1].success,
                    gas_used: sequence_result.results[1].gas_used,
                    revert_reason: sequence_result.results[1].revert_reason.clone(),
                    state_changes: sequence_result.results[1].state_changes.clone(),
                },
                TransactionSimulationResult {
                    success: sequence_result.results[2].success,
                    gas_used: sequence_result.results[2].gas_used,
                    revert_reason: sequence_result.results[2].revert_reason.clone(),
                    state_changes: sequence_result.results[2].state_changes.clone(),
                },
                TransactionSimulationResult {
                    success: sequence_result.results[3].success,
                    gas_used: sequence_result.results[3].gas_used,
                    revert_reason: sequence_result.results[3].revert_reason.clone(),
                    state_changes: sequence_result.results[3].state_changes.clone(),
                },
            )
        } else {
            (
                None,
                TransactionSimulationResult {
                    success: sequence_result.results[0].success,
                    gas_used: sequence_result.results[0].gas_used,
                    revert_reason: sequence_result.results[0].revert_reason.clone(),
                    state_changes: sequence_result.results[0].state_changes.clone(),
                },
                TransactionSimulationResult {
                    success: sequence_result.results[1].success,
                    gas_used: sequence_result.results[1].gas_used,
                    revert_reason: sequence_result.results[1].revert_reason.clone(),
                    state_changes: sequence_result.results[1].state_changes.clone(),
                },
                TransactionSimulationResult {
                    success: sequence_result.results[2].success,
                    gas_used: sequence_result.results[2].gas_used,
                    revert_reason: sequence_result.results[2].revert_reason.clone(),
                    state_changes: sequence_result.results[2].state_changes.clone(),
                },
            )
        };
        
        Ok(SequenceSimulationResult {
            given_tx_result,
            buy_result,
            approve_result,
            sell_result,
            simulation_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            block_number: block_number.unwrap_or(0),
        })
    }
    
    /// Encode swapExactETHForTokens function call
    fn encode_swap_exact_eth_for_tokens(
        &self,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> Bytes {
        // Function selector: 0x7ff36ab5
        let mut data = vec![0x7f, 0xf3, 0x6a, 0xb5];
        
        // amountOutMin
        data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
        
        // path offset
        data.extend_from_slice(&U256::from(128).to_be_bytes::<32>());
        
        // to address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to.as_slice());
        
        // deadline
        data.extend_from_slice(&deadline.to_be_bytes::<32>());
        
        // path array
        data.extend_from_slice(&U256::from(path.len()).to_be_bytes::<32>());
        
        for addr in path {
            data.extend_from_slice(&[0u8; 12]);
            data.extend_from_slice(addr.as_slice());
        }
        
        Bytes::from(data)
    }
    
    /// Encode swapExactTokensForETH function call
    fn encode_swap_exact_tokens_for_eth(
        &self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> Bytes {
        // Function selector: 0x18cbafe5
        let mut data = vec![0x18, 0xcb, 0xaf, 0xe5];
        
        // amountIn
        data.extend_from_slice(&amount_in.to_be_bytes::<32>());
        
        // amountOutMin
        data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
        
        // path offset
        data.extend_from_slice(&U256::from(160).to_be_bytes::<32>());
        
        // to address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to.as_slice());
        
        // deadline
        data.extend_from_slice(&deadline.to_be_bytes::<32>());
        
        // path array
        data.extend_from_slice(&U256::from(path.len()).to_be_bytes::<32>());
        
        for addr in path {
            data.extend_from_slice(&[0u8; 12]);
            data.extend_from_slice(addr.as_slice());
        }
        
        Bytes::from(data)
    }
    
    /// Get the appropriate router address based on pool type
    fn get_router_for_pool_type(&self, pool_type: &str) -> Address {
        match pool_type {
            "V3" | "Uniswap-V3" => {
                // Uniswap V3 SwapRouter
                Address::from([0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D, 0xe3, 0xEd, 0xee, 0x1F, 0x18, 0xE0, 0x15, 0x7C, 0x05, 0x86, 0x15, 0x64])
            },
            "V4" | "Uniswap-V4" => {
                // For V4, we use V3 router for now as V4 is still in development
                // TODO: Update when V4 router is deployed
                Address::from([0xE5, 0x92, 0x42, 0x7A, 0x0A, 0xEc, 0xe9, 0x2D, 0xe3, 0xEd, 0xee, 0x1F, 0x18, 0xE0, 0x15, 0x7C, 0x05, 0x86, 0x15, 0x64])
            },
            _ => {
                // Default to V2 router for "V2", "Uniswap-V2", or any unknown type
                self.config.router_address
            }
        }
    }
    
    /// Encode approve(spender, amount) function call
    fn encode_approve(&self, spender: Address, amount: U256) -> Bytes {
        // Function selector: 0x095ea7b3
        let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];
        
        // spender address
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(spender.as_slice());
        
        // amount
        data.extend_from_slice(&amount.to_be_bytes::<32>());
        
        Bytes::from(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sequence_simulation() {
        // This would be a unit test
    }
}