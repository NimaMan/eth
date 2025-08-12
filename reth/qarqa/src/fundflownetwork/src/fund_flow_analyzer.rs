//! Fund flow analysis and extraction utilities
//! This module analyzes fund flows from processed transactions to build network relationships

use qarqa_core_types::*;
use alloy_primitives::{Address, U256};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, info};

/// Fund flow analyzer for extracting and aggregating fund movements
#[derive(Clone)]
pub struct FundFlowAnalyzer {
    /// Minimum value threshold for including flows (in wei)
    min_value_wei: U256,
    /// Whether to include gas payments
    include_gas: bool,
    /// Whether to treat WETH as ETH
    treat_weth_as_eth: bool,
}

impl FundFlowAnalyzer {
    /// Create a new fund flow analyzer
    pub fn new() -> Self {
        Self {
            min_value_wei: U256::from(1000), // 1000 wei minimum
            include_gas: true,
            treat_weth_as_eth: true,
        }
    }
    
    /// Set minimum value threshold
    pub fn with_min_value(mut self, min_value_wei: U256) -> Self {
        self.min_value_wei = min_value_wei;
        self
    }
    
    /// Set whether to include gas payments
    pub fn with_gas_inclusion(mut self, include_gas: bool) -> Self {
        self.include_gas = include_gas;
        self
    }
    
    /// Set whether to treat WETH as ETH
    pub fn with_weth_as_eth(mut self, treat_weth_as_eth: bool) -> Self {
        self.treat_weth_as_eth = treat_weth_as_eth;
        self
    }
    
    /// Analyze fund flows from complete fund flows
    pub fn analyze_fund_flows(&self, flows: &[CompleteFundFlows]) -> Vec<FundFlow> {
        debug!("Analyzing fund flows from {} transactions", flows.len());
        
        let mut flow_map: HashMap<(Address, Address), FundFlowAccumulator> = HashMap::new();
        
        for complete_flows in flows {
            // Process ETH movements
            for eth_movement in &complete_flows.eth_movements {
                if !self.include_gas && matches!(eth_movement.movement_type, EthMovementType::Gas) {
                    continue;
                }
                
                if eth_movement.amount < self.min_value_wei {
                    continue;
                }
                
                let key = (eth_movement.from, eth_movement.to);
                let accumulator = flow_map.entry(key).or_insert_with(|| {
                    FundFlowAccumulator::new(eth_movement.from, eth_movement.to)
                });
                
                accumulator.add_eth_movement(eth_movement, complete_flows.block_number);
            }
            
            // Process token movements
            for token_movement in &complete_flows.token_movements {
                // Convert WETH to ETH if configured
                if self.treat_weth_as_eth && is_weth_address(token_movement.token_address) {
                    let eth_movement = EthMovement {
                        from: token_movement.from,
                        to: token_movement.to,
                        amount: token_movement.amount,
                        movement_type: EthMovementType::Internal,
                    };
                    
                    if eth_movement.amount >= self.min_value_wei {
                        let key = (eth_movement.from, eth_movement.to);
                        let accumulator = flow_map.entry(key).or_insert_with(|| {
                            FundFlowAccumulator::new(eth_movement.from, eth_movement.to)
                        });
                        accumulator.add_eth_movement(&eth_movement, complete_flows.block_number);
                    }
                } else {
                    // Handle as regular token movement
                    let key = (token_movement.from, token_movement.to);
                    let accumulator = flow_map.entry(key).or_insert_with(|| {
                        FundFlowAccumulator::new(token_movement.from, token_movement.to)
                    });
                    accumulator.add_token_movement(token_movement, complete_flows.block_number);
                }
            }
        }
        
        // Convert accumulators to fund flows
        let mut fund_flows: Vec<FundFlow> = flow_map.into_iter()
            .map(|(_, accumulator)| accumulator.into_fund_flow())
            .collect();
        
        // Sort by ETH amount descending
        fund_flows.sort_by(|a, b| {
            b.amount_eth.partial_cmp(&a.amount_eth).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        info!("Extracted {} unique fund flows", fund_flows.len());
        fund_flows
    }
    
    /// Get net balances from fund flows
    pub fn calculate_net_balances(&self, fund_flows: &[FundFlow]) -> HashMap<Address, NetBalance> {
        debug!("Calculating net balances for {} fund flows", fund_flows.len());
        
        let mut balances: HashMap<Address, NetBalance> = HashMap::new();
        
        for flow in fund_flows {
            // Update sender balance
            let sender_balance = balances.entry(flow.from).or_insert_with(NetBalance::new);
            sender_balance.eth_out += flow.amount_eth;
            sender_balance.usd_out += flow.amount_tokens_usd;
            sender_balance.tx_count += flow.transaction_count;
            
            // Update receiver balance
            let receiver_balance = balances.entry(flow.to).or_insert_with(NetBalance::new);
            receiver_balance.eth_in += flow.amount_eth;
            receiver_balance.usd_in += flow.amount_tokens_usd;
            receiver_balance.tx_count += flow.transaction_count;
        }
        
        // Calculate net values
        for balance in balances.values_mut() {
            balance.net_eth = balance.eth_in - balance.eth_out;
            balance.net_usd = balance.usd_in - balance.usd_out;
        }
        
        info!("Calculated net balances for {} addresses", balances.len());
        balances
    }
}

impl Default for FundFlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Fund flow accumulator for combining multiple movements
#[derive(Debug, Clone)]
struct FundFlowAccumulator {
    from: Address,
    to: Address,
    total_eth: U256,
    total_usd: f64,
    transaction_count: u64,
    first_block: Option<u64>,
    last_block: Option<u64>,
    flow_types: Vec<FlowType>,
}

impl FundFlowAccumulator {
    fn new(from: Address, to: Address) -> Self {
        Self {
            from,
            to,
            total_eth: U256::ZERO,
            total_usd: 0.0,
            transaction_count: 0,
            first_block: None,
            last_block: None,
            flow_types: Vec::new(),
        }
    }
    
    fn add_eth_movement(&mut self, movement: &EthMovement, block_number: u64) {
        self.total_eth += movement.amount;
        self.transaction_count += 1;
        self.update_block_range(block_number);
        
        let flow_type = match movement.movement_type {
            EthMovementType::Direct => FlowType::DirectTransfer,
            EthMovementType::Internal => FlowType::InternalTransfer,
            EthMovementType::Gas => FlowType::GasPayment,
            EthMovementType::Refund => FlowType::GasPayment, // Treat refunds as gas-related
            EthMovementType::SelfDestruct => FlowType::InternalTransfer, // Treat as internal transfer
        };
        
        if !self.flow_types.contains(&flow_type) {
            self.flow_types.push(flow_type);
        }
    }
    
    fn add_token_movement(&mut self, movement: &TokenMovement, block_number: u64) {
        // Convert token amount to USD (simplified - would need price oracle)
        // For now, just count the movement
        self.transaction_count += 1;
        self.update_block_range(block_number);
        
        let flow_type = FlowType::TokenTransfer(movement.token_address);
        if !self.flow_types.contains(&flow_type) {
            self.flow_types.push(flow_type);
        }
    }
    
    fn update_block_range(&mut self, block_number: u64) {
        match self.first_block {
            None => self.first_block = Some(block_number),
            Some(ref mut first) => {
                if block_number < *first {
                    *first = block_number;
                }
            }
        }
        
        match self.last_block {
            None => self.last_block = Some(block_number),
            Some(ref mut last) => {
                if block_number > *last {
                    *last = block_number;
                }
            }
        }
    }
    
    fn into_fund_flow(self) -> FundFlow {
        let primary_flow_type = self.flow_types.into_iter().next()
            .unwrap_or(FlowType::DirectTransfer);
        
        FundFlow {
            from: self.from,
            to: self.to,
            amount_eth: wei_to_eth(self.total_eth),
            amount_tokens_usd: self.total_usd,
            transaction_count: self.transaction_count,
            first_block: self.first_block.unwrap_or(0),
            last_block: self.last_block.unwrap_or(0),
            flow_type: primary_flow_type,
        }
    }
}

/// Net balance for an address
#[derive(Debug, Clone, Default)]
pub struct NetBalance {
    pub eth_in: f64,
    pub eth_out: f64,
    pub net_eth: f64,
    pub usd_in: f64,
    pub usd_out: f64,
    pub net_usd: f64,
    pub tx_count: u64,
}

impl NetBalance {
    fn new() -> Self {
        Self::default()
    }
}

/// Check if an address is WETH
fn is_weth_address(address: Address) -> bool {
    // WETH contract address on mainnet
    address == Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap_or_default()
}

/// Convert wei to ETH
fn wei_to_eth(wei: U256) -> f64 {
    let wei_per_eth = U256::from(10).pow(U256::from(18));
    let eth_part = wei / wei_per_eth;
    let wei_remainder = wei % wei_per_eth;
    
    eth_part.to::<u64>() as f64 + (wei_remainder.to::<u64>() as f64 / 1e18)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};
    use std::str::FromStr;
    
    #[test]
    fn test_fund_flow_analyzer_basic() {
        let analyzer = FundFlowAnalyzer::new();
        
        // Create test data
        let eth_movement = EthMovement {
            from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
            to: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
            amount: U256::from_str("1000000000000000000").unwrap(), // 1 ETH
            movement_type: EthMovementType::Direct,
        };
        
        let complete_flows = CompleteFundFlows {
            tx_hash: "0x0".to_string(),
            block_number: 1,
            from_address: eth_movement.from,
            to_address: Some(eth_movement.to),
            eth_movements: vec![eth_movement],
            token_movements: Vec::new(),
        };
        
        let fund_flows = analyzer.analyze_fund_flows(&[complete_flows]);
        
        assert_eq!(fund_flows.len(), 1);
        assert_eq!(fund_flows[0].amount_eth, 1.0);
        assert_eq!(fund_flows[0].transaction_count, 1);
    }
    
    #[test]
    fn test_net_balance_calculation() {
        let analyzer = FundFlowAnalyzer::new();
        
        let fund_flows = vec![
            FundFlow {
                from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
                to: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
                amount_eth: 1.0,
                amount_tokens_usd: 0.0,
                transaction_count: 1,
                first_block: 1,
                last_block: 1,
                flow_type: FlowType::DirectTransfer,
            }
        ];
        
        let balances = analyzer.calculate_net_balances(&fund_flows);
        
        assert_eq!(balances.len(), 2);
        
        let sender_balance = &balances[&Address::from_str("0x1111111111111111111111111111111111111111").unwrap()];
        assert_eq!(sender_balance.eth_out, 1.0);
        assert_eq!(sender_balance.net_eth, -1.0);
        
        let receiver_balance = &balances[&Address::from_str("0x2222222222222222222222222222222222222222").unwrap()];
        assert_eq!(receiver_balance.eth_in, 1.0);
        assert_eq!(receiver_balance.net_eth, 1.0);
    }
    
    #[test]
    fn test_wei_to_eth_conversion() {
        assert_eq!(wei_to_eth(U256::from_str("1000000000000000000").unwrap()), 1.0);
        assert_eq!(wei_to_eth(U256::from_str("500000000000000000").unwrap()), 0.5);
        assert_eq!(wei_to_eth(U256::ZERO), 0.0);
    }
}