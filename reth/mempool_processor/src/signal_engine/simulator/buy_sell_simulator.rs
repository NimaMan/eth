/// Buy/Sell Simulator for Token Safety Analysis
/// 
/// This module provides a reusable interface for simulating buy and sell transactions
/// to detect honeypots, calculate taxes, and verify token safety before trading.
/// 
/// Key features:
/// - Sequential buy -> approve -> sell simulation
/// - Honeypot detection (can buy but can't sell)
/// - Tax calculation (buy and sell taxes)
/// - MEV opportunity detection (tax changes)
/// - Contract creation validation

use async_trait::async_trait;
use eyre::Result;
use std::time::Instant;
use std::sync::Arc;
use alloy_primitives::{Address, Bytes, U256};
use crate::tx_simulator::{DirectTxSimulator, CallRequest};
use reth_tx_simulator::SequentialSimulationOptions;

/// Result of a buy/sell simulation sequence
#[derive(Debug, Clone)]
pub struct BuySellResult {
    /// Whether the buy transaction succeeded
    pub can_buy: bool,
    /// Whether the sell transaction succeeded
    pub can_sell: bool,
    /// Calculated buy tax percentage (0-100)
    pub buy_tax: f64,
    /// Calculated sell tax percentage (0-100), -1 if sell failed
    pub sell_tax: f64,
    /// Whether this token is a honeypot (can buy but can't sell)
    pub is_honeypot: bool,
    /// Amount of tokens received from buy
    pub tokens_received: U256,
    /// Amount of tokens attempted to sell
    pub tokens_sold: U256,
    /// Total simulation time in milliseconds
    pub simulation_time_ms: f64,
    /// Optional error message if simulation failed
    pub error: Option<String>,
}

impl BuySellResult {
    /// Create a failed result with error message
    pub fn failed(error: String) -> Self {
        Self {
            can_buy: false,
            can_sell: false,
            buy_tax: -1.0,
            sell_tax: -1.0,
            is_honeypot: false,
            tokens_received: U256::ZERO,
            tokens_sold: U256::ZERO,
            simulation_time_ms: 0.0,
            error: Some(error),
        }
    }
}

/// Trait for buy/sell simulation implementations
#[async_trait]
pub trait BuySellSimulator {
    /// Simulate a buy/sell sequence for a token
    async fn simulate_buy_sell_sequence(
        &self,
        token_address: Address,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<BuySellResult>;
    
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
            test_buy_amount: U256::from(100_000_000_000_000_000u64), // 0.1 ETH
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
    simulator: DirectTxSimulator,
    config: BuySellSimulatorConfig,
    /// Optional token cache for faster lookups
    token_cache: Option<Arc<crate::token_tracking::TokenTrackingCache>>,
}

impl SequentialBuySellSimulator {
    /// Create a new buy/sell simulator
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let simulator = DirectTxSimulator::new(reth_datadir)?;
        Ok(Self {
            simulator,
            config: BuySellSimulatorConfig::default(),
            token_cache: None,
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(reth_datadir: &str, config: BuySellSimulatorConfig) -> Result<Self> {
        let simulator = DirectTxSimulator::new(reth_datadir)?;
        Ok(Self {
            simulator,
            config,
            token_cache: None,
        })
    }
    
    /// Set token cache for optimized lookups
    pub fn set_token_cache(&mut self, cache: Arc<crate::token_tracking::TokenTrackingCache>) {
        self.token_cache = Some(cache);
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
    
    /// Calculate buy tax from state changes
    fn calculate_buy_tax(
        &self,
        _pool_eth_in: U256,
        buyer_tokens_received: U256,
        pool_tokens_out: U256,
    ) -> f64 {
        if pool_tokens_out > U256::ZERO && buyer_tokens_received > U256::ZERO {
            // Convert to f64 for percentage calculation
            let from_pool_f64 = pool_tokens_out.to_string().parse::<f64>().unwrap_or(0.0);
            let to_buyer_f64 = buyer_tokens_received.to_string().parse::<f64>().unwrap_or(0.0);
            
            if from_pool_f64 > 0.0 {
                let tax_percent = (1.0 - (to_buyer_f64 / from_pool_f64)) * 100.0;
                return tax_percent.max(0.0);
            }
        }
        0.0
    }
    
    /// Calculate sell tax from state changes
    fn calculate_sell_tax(
        &self,
        _seller_tokens_sent: U256,
        seller_eth_received: U256,
        pool_eth_out: U256,
    ) -> f64 {
        if pool_eth_out > U256::ZERO && seller_eth_received > U256::ZERO {
            // Convert to f64 for percentage calculation
            let from_pool_f64 = pool_eth_out.to_string().parse::<f64>().unwrap_or(0.0);
            let to_seller_f64 = seller_eth_received.to_string().parse::<f64>().unwrap_or(0.0);
            
            if from_pool_f64 > 0.0 {
                let tax_percent = (1.0 - (to_seller_f64 / from_pool_f64)) * 100.0;
                return tax_percent.max(0.0);
            }
        }
        0.0
    }
}

#[async_trait]
impl BuySellSimulator for SequentialBuySellSimulator {
    async fn simulate_buy_sell_sequence(
        &self,
        token_address: Address,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<BuySellResult> {
        let start_time = Instant::now();
        
        // Build buy transaction
        let buy_calldata = self.encode_swap_exact_eth_for_tokens(
            U256::ZERO, // min tokens out
            vec![self.config.weth_address, token_address],
            self.config.buyer_address,
            U256::from(9999999999u64),
        );
        
        let buy_request = CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(self.config.router_address),
            value: Some(self.config.test_buy_amount),
            data: Some(buy_calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // First simulate just the buy to get tokens received
        let buy_only_result = self.simulator.simulate_transaction_sequence(
            vec![buy_request.clone()],
            SequentialSimulationOptions {
                at_block: block_number,
                stop_on_failure: true,
                auto_increment_nonces: true,
                gas_limit_per_tx: None,
            }
        ).await?;
        
        if !buy_only_result.results[0].success {
            return Ok(BuySellResult {
                can_buy: false,
                can_sell: false,
                buy_tax: -1.0,
                sell_tax: -1.0,
                is_honeypot: false,
                tokens_received: U256::ZERO,
                tokens_sold: U256::ZERO,
                simulation_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
                error: Some("Buy transaction failed".to_string()),
            });
        }
        
        // Extract tokens received from buy
        let tokens_received = buy_only_result.results[0].state_changes
            .get(&self.config.buyer_address)
            .and_then(|changes| {
                let token_addr_str = format!("{:#x}", token_address);
                changes.token_net.get(&token_addr_str).copied()
            })
            .unwrap_or(U256::ZERO);
        
        // Build approve transaction
        let approve_calldata = self.encode_approve(self.config.router_address, tokens_received);
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
        
        // Build sell transaction
        let sell_calldata = self.encode_swap_exact_tokens_for_eth(
            tokens_received,
            U256::ZERO, // min ETH out
            vec![token_address, self.config.weth_address],
            self.config.buyer_address,
            U256::from(9999999999u64),
        );
        
        let sell_request = CallRequest {
            from: Some(self.config.buyer_address),
            to: Some(self.config.router_address),
            value: Some(U256::ZERO),
            data: Some(sell_calldata),
            gas: Some(self.config.gas_limit),
            gas_price: Some(self.config.gas_price),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // Run full sequence: buy -> approve -> sell
        let sequence_result = self.simulator.simulate_transaction_sequence(
            vec![buy_request, approve_request, sell_request],
            SequentialSimulationOptions {
                at_block: block_number,
                stop_on_failure: false, // Don't stop on failure to see which tx fails
                auto_increment_nonces: true,
                gas_limit_per_tx: None,
            }
        ).await?;
        
        // Analyze results
        let buy_success = sequence_result.results[0].success;
        let approve_success = sequence_result.results.len() > 1 && sequence_result.results[1].success;
        let sell_success = sequence_result.results.len() > 2 && sequence_result.results[2].success;
        
        // Calculate taxes from state changes
        let buy_tax = if buy_success {
            // Extract pool and buyer changes from buy transaction
            let buy_changes = &sequence_result.results[0].state_changes;
            
            let pool_eth_in = buy_changes.get(&pool_address)
                .map(|c| c.eth_net)
                .unwrap_or(U256::ZERO);
                
            let pool_tokens_out = buy_changes.get(&pool_address)
                .and_then(|c| {
                    let token_addr_str = format!("{:#x}", token_address);
                    c.token_net.get(&token_addr_str).map(|v| {
                        // Pool loses tokens, so negate the negative value
                        if *v > U256::from(0) { U256::ZERO } else { U256::ZERO - *v }
                    })
                })
                .unwrap_or(U256::ZERO);
                
            self.calculate_buy_tax(pool_eth_in, tokens_received, pool_tokens_out)
        } else {
            -1.0
        };
        
        let sell_tax = if sell_success {
            // Extract pool and seller changes from sell transaction
            let sell_changes = &sequence_result.results[2].state_changes;
            
            let pool_eth_out = sell_changes.get(&pool_address)
                .map(|c| {
                    // Pool loses ETH, so we need the negative value
                    if c.eth_net > U256::from(0) { U256::ZERO } else { U256::ZERO - c.eth_net }
                })
                .unwrap_or(U256::ZERO);
                
            let seller_eth_received = sell_changes.get(&self.config.buyer_address)
                .map(|c| c.eth_net)
                .unwrap_or(U256::ZERO);
                
            self.calculate_sell_tax(tokens_received, seller_eth_received, pool_eth_out)
        } else {
            -1.0
        };
        
        let is_honeypot = buy_success && !sell_success;
        
        Ok(BuySellResult {
            can_buy: buy_success,
            can_sell: sell_success,
            buy_tax,
            sell_tax,
            is_honeypot,
            tokens_received,
            tokens_sold: if sell_success { tokens_received } else { U256::ZERO },
            simulation_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
            error: if !buy_success || !sell_success {
                Some(format!("Buy: {}, Approve: {}, Sell: {}", 
                    if buy_success { "OK" } else { "FAILED" },
                    if approve_success { "OK" } else { "FAILED" },
                    if sell_success { "OK" } else { "FAILED" }
                ))
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_buy_sell_simulator() {
        // This would be a unit test
    }
}