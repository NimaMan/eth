/// Tax Calculator
/// 
/// Uses sequential buy/sell simulation to measure token taxes
/// 
/// Algorithm:
/// 1. Simulate buy: 0.1 ETH -> tokens
/// 2. Simulate sell: ALL tokens from buy -> ETH
/// 3. Calculate taxes from actual token/ETH movements

use eyre::Result;
use alloy_primitives::{Address, U256, I256, Bytes};
use reth_tx_simulator::{RethTxSimulator, CallRequest, state_change_calculator::AddressStateChange, SequentialSimulationOptions};
use std::collections::HashMap;

pub struct TokenInfo {
    pub address: Address,
    pub decimals: u8,
    pub total_supply: U256,
}

pub struct PoolReserves {
    pub token_reserve: U256,  // In token's smallest unit
    pub eth_reserve: U256,    // In wei
}

pub struct TaxCalculator {
    simulator: RethTxSimulator,
}

impl TaxCalculator {
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let simulator = RethTxSimulator::new(reth_datadir)?;
        Ok(Self { simulator })
    }
    
    
    /// Calculate taxes via independent buy/sell simulations
    /// 
    /// This method:
    /// 1. Simulates buy: 0.1 ETH -> tokens (measures buy tax)
    /// 2. Simulates sell: ALL tokens received -> ETH (measures sell tax)
    /// 3. Calculates taxes from actual token/ETH movements
    /// 
    /// Fixed buyer address: 0x0C96c602b1b332B8AB2093E5d72D804a24bd5689
    pub async fn calculate_taxes_via_simulation(
        &self,
        buyer_address: Address,
        token_info: &TokenInfo,
        pool_address: Address,
        router_address: Address,
        _reserves: &PoolReserves,
        block_number: u64,
    ) -> Result<(f64, f64, U256, U256)> {  // Returns (buy_tax, sell_tax, tokens_bought, tokens_sold)
        // Create buy transaction: 0.1 ETH -> tokens
        let buy_amount = U256::from(100_000_000_000_000_000u64); // 0.1 ETH
        let buy_calldata = self.encode_swap_exact_eth_for_tokens(
            U256::ZERO, // min tokens out
            vec![
                Address::from([0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2]), // WETH
                token_info.address,
            ],
            buyer_address,
            U256::from(9999999999u64),
        );
        
        let buy_request = CallRequest {
            from: Some(buyer_address),
            to: Some(router_address),
            value: Some(buy_amount),
            data: Some(buy_calldata),
            gas: Some(300000),
            gas_price: Some(20_000_000_000),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // First, determine how many tokens the buyer receives from the buy
        println!("🔄 Simulating sequential buy/sell to calculate taxes...");
        println!("   Block: {}", block_number);
        println!("   Router: {:?}", router_address);
        println!("   Token: {:?}", token_info.address);
        println!("   Pool: {:?}", pool_address);
        
        let buy_result = self.simulator.simulate_transaction_detailed(
            buy_request.clone(),
            Some(block_number)
        ).await?;
        
        if !buy_result.success {
            return Err(eyre::eyre!("Buy simulation failed: {:?}", buy_result.revert_reason));
        }
        
        println!("\n   🔍 Buy simulation result:");
        println!("      Success: {}", buy_result.success);
        println!("      Gas used: {}", buy_result.gas_used);
        println!("      State changes count: {}", buy_result.state_changes.len());
        println!("      Logs count: {}", buy_result.logs.len());
        
        // Check if there are any swap events
        for log in &buy_result.logs {
            println!("      Log from: {:?}", log.address);
            if log.topics().len() > 0 {
                println!("        Topic[0]: {:?}", log.topics()[0]);
            }
        }
        
        // Extract EXACT tokens from state changes - this is critical!
        let mut tokens_received = U256::ZERO;
        
        if let Some(buyer_changes) = buy_result.state_changes.get(&buyer_address) {
            for (token_key, amount) in &buyer_changes.token_net {
                let token_addr_str = format!("{:#x}", token_info.address);
                if token_key == &token_addr_str && *amount > I256::ZERO {
                    // Convert I256 to U256 (positive value)
                    tokens_received = amount.unsigned_abs();
                    break;
                }
            }
        }
        
        if tokens_received == U256::ZERO {
            return Err(eyre::eyre!("Failed to extract tokens received by buyer"));
        }
        
        // Create approve transaction: allow router to spend tokens
        let approve_calldata = self.encode_approve(router_address, U256::MAX);
        
        let approve_request = CallRequest {
            from: Some(buyer_address),
            to: Some(token_info.address),
            value: Some(U256::ZERO),
            data: Some(approve_calldata),
            gas: Some(100000),
            gas_price: Some(20_000_000_000),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // For sell simulation, try to sell all tokens received to measure actual tax
        let sell_amount = tokens_received;
        println!("\n   🔍 Setting up sell transaction:");
        println!("      Buyer has: {} raw tokens ({} with {} decimals)", 
            tokens_received, 
            tokens_received / Self::pow10(token_info.decimals),
            token_info.decimals
        );
        println!("      Trying to sell: {} raw tokens ({} with {} decimals)", 
            sell_amount,
            sell_amount / Self::pow10(token_info.decimals),
            token_info.decimals
        );
        
        let sell_calldata = self.encode_swap_exact_tokens_for_eth(
            sell_amount,
            U256::ZERO, // min ETH out
            vec![
                token_info.address,
                Address::from([0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2]), // WETH
            ],
            buyer_address,
            U256::from(9999999999u64),
        );
        
        let sell_request = CallRequest {
            from: Some(buyer_address),
            to: Some(router_address),
            value: Some(U256::ZERO),
            data: Some(sell_calldata),
            gas: Some(300000),
            gas_price: Some(20_000_000_000),
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            nonce: None,
        };
        
        // Run sequential simulation: buy -> approve -> sell
        let sequence = vec![buy_request, approve_request, sell_request];
        let options = SequentialSimulationOptions {
            at_block: Some(block_number),
            stop_on_failure: true,
            auto_increment_nonces: true,
            gas_limit_per_tx: None,
        };
        
        let sequence_result = self.simulator.simulate_transaction_sequence(
            sequence,
            options
        ).await?;
        
        // Check results
        println!("   📊 Sequence results: {} successful, {} failed", 
            sequence_result.successful_transactions, 
            sequence_result.failed_transactions
        );
        
        for (i, result) in sequence_result.results.iter().enumerate() {
            let tx_type = match i {
                0 => "Buy",
                1 => "Approve", 
                2 => "Sell",
                _ => "Unknown"
            };
            if !result.success {
                println!("   ❌ {} transaction failed: {:?}", tx_type, result.revert_reason);
            } else {
                println!("   ✅ {} transaction succeeded (gas used: {})", tx_type, result.gas_used);
            }
        }
        
        // Calculate buy tax from first transaction
        let buy_tax = if let Some(buy_tx) = sequence_result.results.get(0) {
            if buy_tx.success {
                // Debug all state changes
                println!("\n   🔍 All state changes after sequential buy:");
                println!("      Total addresses with changes: {}", buy_tx.state_changes.len());
                
                // Check specific addresses
                let addresses_to_check = vec![
                    (buyer_address, "Buyer"),
                    (router_address, "Router"),
                    (pool_address, "Pool"),
                    (token_info.address, "Token"),
                ];
                
                for (addr, name) in addresses_to_check {
                    if let Some(changes) = buy_tx.state_changes.get(&addr) {
                        println!("      {} ({:#x}):", name, addr);
                        if !changes.eth_net.is_zero() {
                            let eth_f64 = changes.eth_net.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                            println!("        ETH: {:+.6}", eth_f64);
                        }
                        for (token_key, amount) in &changes.token_net {
                            println!("        Token {}: {:+.6}", token_key, amount);
                        }
                    } else {
                        println!("      {} ({:#x}): No changes", name, addr);
                    }
                }
                
                // Check if buyer got any tokens
                if let Some(buyer_changes) = buy_tx.state_changes.get(&buyer_address) {
                    let token_addr_str = format!("{:#x}", token_info.address);
                    let tokens = buyer_changes.token_net.get(&token_addr_str);
                    println!("\n      Buyer token balance change: {:?}", tokens);
                }
                
                self.calculate_buy_tax_from_state_changes(
                    &buy_tx.state_changes,
                    &pool_address,
                    &buyer_address,
                ).unwrap_or(0.0)
            } else {
                return Err(eyre::eyre!("Buy transaction failed"));
            }
        } else {
            return Err(eyre::eyre!("No buy transaction result"));
        };
        
        // Calculate sell tax from third transaction (after approve)
        let sell_tax = if let Some(sell_tx) = sequence_result.results.get(2) {
            if sell_tx.success {
                self.calculate_sell_tax_from_state_changes(
                    &sell_tx.state_changes,
                    &pool_address,
                    &buyer_address,
                ).unwrap_or(0.0)
            } else {
                // If sell fails, it might be due to transfer restrictions
                println!("   ⚠️  Sell transaction failed - token may have transfer restrictions");
                println!("   ℹ️  This could indicate:");
                println!("      - Trading cooldown period");
                println!("      - Holder restrictions");
                println!("      - Or very high sell tax (honeypot)");
                -1.0 // Indicate failure
            }
        } else {
            return Err(eyre::eyre!("No sell transaction result"));
        };
        
        println!("   💸 Buy Tax: {:.1}%", buy_tax);
        if sell_tax >= 0.0 {
            println!("   💸 Sell Tax: {:.1}%", sell_tax);
        } else {
            println!("   💸 Sell Tax: Unable to determine (transfer failed)");
        }
        
        // Return tokens_received and sell_amount along with taxes
        Ok((buy_tax, sell_tax, tokens_received, sell_amount))
    }
    
    /// Helper to power of 10
    fn pow10(decimals: u8) -> U256 {
        U256::from(10).pow(U256::from(decimals))
    }
    
    /// Extract how many tokens the buyer received from a buy transaction
    fn extract_tokens_received_by_buyer(
        &self,
        state_changes: &HashMap<Address, AddressStateChange>,
        buyer_address: &Address,
        token_address: &Address,
    ) -> Option<U256> {
        if let Some(buyer_changes) = state_changes.get(buyer_address) {
            // Find the token balance change
            for (token_key, amount) in &buyer_changes.token_net {
                // The key format is the token address in hex format (0x...)
                let token_addr_str = format!("{:#x}", token_address);
                if token_key == &token_addr_str {
                    if *amount > I256::ZERO {
                        // Now we have the EXACT U256 amount - no conversion needed!
                        println!("      🔍 Found token balance change: {} (exact U256)", amount);
                        println!("      🔍 Token key: {}", token_key);
                        println!("      🔍 Is this the token we're looking for? {}", token_key == &token_addr_str);
                        
                        // Use the amount directly - no conversion needed
                        println!("      Using EXACT amount: {}", amount);
                        return Some(amount.unsigned_abs());
                    }
                }
            }
        }
        None
    }
    
    /// Calculate buy tax from state changes using movements data
    /// 
    /// Buy Tax Formula: (1 - tokens_received_by_buyer / tokens_sent_by_pool) × 100
    /// 
    /// We track:
    /// 1. How many tokens leave the pool (outgoing from pool)
    /// 2. How many tokens the buyer receives (incoming to buyer)
    /// 3. The difference is the tax
    /// 
    /// Example: Pool sends 100 tokens, buyer receives 95 tokens = 5% buy tax
    fn calculate_buy_tax_from_state_changes(
        &self,
        state_changes: &HashMap<Address, AddressStateChange>,
        pool_address: &Address,
        buyer_address: &Address,
    ) -> Option<f64> {
        let pool_changes = state_changes.get(pool_address)?;
        let buyer_changes = state_changes.get(buyer_address)?;
        
        // Find the token address by checking buyer's incoming tokens
        for (token_addr, buyer_token_movements) in &buyer_changes.movements.tokens {
            // Calculate tokens received by buyer
            let tokens_to_buyer: U256 = buyer_token_movements.incoming
                .values()
                .map(|e| e.amount)
                .fold(U256::ZERO, |acc, x| acc + x);
            
            if tokens_to_buyer > U256::ZERO {
                // Check pool's outgoing tokens for the same token
                if let Some(pool_token_movements) = pool_changes.movements.tokens.get(token_addr) {
                    let tokens_from_pool: U256 = pool_token_movements.outgoing
                        .values()
                        .map(|e| e.amount)
                        .fold(U256::ZERO, |acc, x| acc + x);
                    
                    if tokens_from_pool > U256::ZERO {
                        // Calculate tax percentage
                        // Convert to f64 for percentage calculation
                        let from_pool_f64 = tokens_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
                        let to_buyer_f64 = tokens_to_buyer.to_string().parse::<f64>().unwrap_or(0.0);
                        
                        if from_pool_f64 > 0.0 {
                            let tax_percent = (1.0 - (to_buyer_f64 / from_pool_f64)) * 100.0;
                            return Some(tax_percent.max(0.0));
                        }
                    }
                }
            }
        }
        
        None // Unable to calculate tax from state changes
    }
    
    /// Calculate sell tax from state changes using movements data
    /// 
    /// Sell Tax Formula: (1 - eth_received_by_seller / eth_sent_by_pool) × 100
    /// 
    /// We track:
    /// 1. How much ETH leaves the pool (outgoing from pool)
    /// 2. How much ETH the seller receives (incoming to seller)
    /// 3. The difference is the tax
    /// 
    /// Example: Pool sends 1 ETH, seller receives 0.1 ETH = 90% sell tax (honeypot!)
    fn calculate_sell_tax_from_state_changes(
        &self,
        state_changes: &HashMap<Address, AddressStateChange>,
        pool_address: &Address,
        seller_address: &Address,
    ) -> Option<f64> {
        let pool_changes = state_changes.get(pool_address)?;
        let seller_changes = state_changes.get(seller_address)?;
        
        // Calculate ETH received by seller
        let eth_to_seller: U256 = seller_changes.movements.denom.incoming
            .values()
            .map(|e| e.amount)
            .fold(U256::ZERO, |acc, x| acc + x);
        
        if eth_to_seller > U256::ZERO {
            // Calculate ETH sent from pool
            let eth_from_pool: U256 = pool_changes.movements.denom.outgoing
                .values()
                .map(|e| e.amount)
                .fold(U256::ZERO, |acc, x| acc + x);
            
            if eth_from_pool > U256::ZERO {
                // Calculate tax percentage
                // Convert to f64 for percentage calculation
                let from_pool_f64 = eth_from_pool.to_string().parse::<f64>().unwrap_or(0.0);
                let to_seller_f64 = eth_to_seller.to_string().parse::<f64>().unwrap_or(0.0);
                
                if from_pool_f64 > 0.0 {
                    let tax_percent = (1.0 - (to_seller_f64 / from_pool_f64)) * 100.0;
                    return Some(tax_percent.max(0.0));
                }
            }
        }
        
        None // Unable to calculate tax from state changes
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
    fn encode_approve(
        &self,
        spender: Address,
        amount: U256,
    ) -> Bytes {
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