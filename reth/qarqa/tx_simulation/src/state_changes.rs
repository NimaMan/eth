//! State change analysis for addresses

use qarqa_core_types::*;
use alloy_primitives::Address;
use std::collections::HashMap;
use tracing::{debug, info};

/// State change analyzer
#[derive(Clone)]
pub struct StateChangeAnalyzer {
    /// USD exchange rates for tokens
    usd_rates: HashMap<Address, f64>,
    /// ETH price in USD
    eth_usd_rate: f64,
}

impl StateChangeAnalyzer {
    /// Create a new state change analyzer
    pub fn new() -> Self {
        Self {
            usd_rates: HashMap::new(),
            eth_usd_rate: 2500.0, // Default ETH price
        }
    }
    
    /// Set ETH USD rate
    pub fn with_eth_rate(mut self, eth_usd_rate: f64) -> Self {
        self.eth_usd_rate = eth_usd_rate;
        self
    }
    
    /// Add token USD rate
    pub fn with_token_rate(mut self, token: Address, usd_rate: f64) -> Self {
        self.usd_rates.insert(token, usd_rate);
        self
    }
    
    /// Set multiple token rates
    pub fn with_rates(mut self, rates: HashMap<Address, f64>) -> Self {
        self.usd_rates.extend(rates);
        self
    }
    
    /// Calculate state changes from fund flows
    pub fn calculate_state_changes(&self, fund_flows: &[FundFlow]) -> HashMap<Address, AddressStateChange> {
        debug!("Calculating state changes for {} fund flows", fund_flows.len());
        
        let mut state_changes: HashMap<Address, AddressStateChange> = HashMap::new();
        
        for flow in fund_flows {
            // Update sender state
            let sender_state = state_changes.entry(flow.from).or_insert_with(|| {
                AddressStateChange::new(flow.from)
            });
            sender_state.subtract_flow(flow, self.eth_usd_rate);
            
            // Update receiver state
            let receiver_state = state_changes.entry(flow.to).or_insert_with(|| {
                AddressStateChange::new(flow.to)
            });
            receiver_state.add_flow(flow, self.eth_usd_rate);
        }
        
        // Calculate final USD values
        for state_change in state_changes.values_mut() {
            state_change.calculate_usd_values(self.eth_usd_rate);
        }
        
        info!("Calculated state changes for {} addresses", state_changes.len());
        state_changes
    }
    
    /// Filter significant state changes
    pub fn filter_significant_changes(
        &self,
        state_changes: &HashMap<Address, AddressStateChange>,
        min_usd_change: f64,
    ) -> HashMap<Address, AddressStateChange> {
        debug!("Filtering state changes with min USD change: {}", min_usd_change);
        
        let significant_changes: HashMap<Address, AddressStateChange> = state_changes
            .iter()
            .filter(|(_, change)| change.usd_value_change.abs() >= min_usd_change)
            .map(|(addr, change)| (*addr, change.clone()))
            .collect();
        
        info!(
            "Found {} significant changes out of {} total",
            significant_changes.len(),
            state_changes.len()
        );
        
        significant_changes
    }
    
    /// Get top gainers and losers
    pub fn get_top_movers(
        &self,
        state_changes: &HashMap<Address, AddressStateChange>,
        count: usize,
    ) -> (Vec<(Address, f64)>, Vec<(Address, f64)>) {
        let mut gainers: Vec<(Address, f64)> = state_changes
            .iter()
            .filter(|(_, change)| change.usd_value_change > 0.0)
            .map(|(addr, change)| (*addr, change.usd_value_change))
            .collect();
        
        let mut losers: Vec<(Address, f64)> = state_changes
            .iter()
            .filter(|(_, change)| change.usd_value_change < 0.0)
            .map(|(addr, change)| (*addr, change.usd_value_change))
            .collect();
        
        // Sort gainers descending, losers ascending
        gainers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        losers.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top count
        gainers.truncate(count);
        losers.truncate(count);
        
        (gainers, losers)
    }
}

impl Default for StateChangeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// State change for a specific address
#[derive(Debug, Clone)]
pub struct AddressStateChange {
    pub address: Address,
    pub eth_change: i128, // In wei, can be negative
    pub token_changes: HashMap<Address, i128>, // Token address -> amount change
    pub usd_value_change: f64,
    pub transaction_count: u64,
    pub first_block: u64,
    pub last_block: u64,
}

impl AddressStateChange {
    /// Create a new address state change
    pub fn new(address: Address) -> Self {
        Self {
            address,
            eth_change: 0,
            token_changes: HashMap::new(),
            usd_value_change: 0.0,
            transaction_count: 0,
            first_block: u64::MAX,
            last_block: 0,
        }
    }
    
    /// Add a fund flow as income
    fn add_flow(&mut self, flow: &FundFlow, _eth_usd_rate: f64) {
        let eth_wei = eth_to_wei(flow.amount_eth);
        self.eth_change += eth_wei as i128;
        self.transaction_count += flow.transaction_count;
        self.update_block_range(flow.first_block, flow.last_block);
    }
    
    /// Subtract a fund flow as outgoing
    fn subtract_flow(&mut self, flow: &FundFlow, _eth_usd_rate: f64) {
        let eth_wei = eth_to_wei(flow.amount_eth);
        self.eth_change -= eth_wei as i128;
        self.transaction_count += flow.transaction_count;
        self.update_block_range(flow.first_block, flow.last_block);
    }
    
    /// Update block range
    fn update_block_range(&mut self, first: u64, last: u64) {
        if first < self.first_block {
            self.first_block = first;
        }
        if last > self.last_block {
            self.last_block = last;
        }
    }
    
    /// Calculate USD value of changes
    fn calculate_usd_values(&mut self, eth_usd_rate: f64) {
        // ETH change in USD
        let eth_change_eth = wei_to_eth_signed(self.eth_change);
        self.usd_value_change = eth_change_eth * eth_usd_rate;
        
        // TODO: Add token changes when we have token rates
    }
    
    /// Get net ETH change in ETH units
    pub fn net_eth_change(&self) -> f64 {
        wei_to_eth_signed(self.eth_change)
    }
    
    /// Check if this is a significant change
    pub fn is_significant(&self, min_usd_threshold: f64) -> bool {
        self.usd_value_change.abs() >= min_usd_threshold
    }
}

/// Convert ETH to wei
fn eth_to_wei(eth: f64) -> u64 {
    (eth * 1e18) as u64
}

/// Convert wei to ETH (signed)
fn wei_to_eth_signed(wei: i128) -> f64 {
    wei as f64 / 1e18
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};
    use std::str::FromStr;
    
    #[test]
    fn test_state_change_analyzer() {
        let analyzer = StateChangeAnalyzer::new()
            .with_eth_rate(2500.0);
        
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
        
        let state_changes = analyzer.calculate_state_changes(&fund_flows);
        
        assert_eq!(state_changes.len(), 2);
        
        let sender_change = &state_changes[&Address::from_str("0x1111111111111111111111111111111111111111").unwrap()];
        assert_eq!(sender_change.net_eth_change(), -1.0);
        assert_eq!(sender_change.usd_value_change, -2500.0);
        
        let receiver_change = &state_changes[&Address::from_str("0x2222222222222222222222222222222222222222").unwrap()];
        assert_eq!(receiver_change.net_eth_change(), 1.0);
        assert_eq!(receiver_change.usd_value_change, 2500.0);
    }
    
    #[test]
    fn test_filter_significant_changes() {
        let analyzer = StateChangeAnalyzer::new();
        
        let mut state_changes = HashMap::new();
        
        // Small change
        let mut small_change = AddressStateChange::new(
            Address::from_str("0x1111111111111111111111111111111111111111").unwrap()
        );
        small_change.usd_value_change = 5.0;
        state_changes.insert(small_change.address, small_change);
        
        // Large change
        let mut large_change = AddressStateChange::new(
            Address::from_str("0x2222222222222222222222222222222222222222").unwrap()
        );
        large_change.usd_value_change = 1000.0;
        state_changes.insert(large_change.address, large_change);
        
        let significant = analyzer.filter_significant_changes(&state_changes, 100.0);
        
        assert_eq!(significant.len(), 1);
        assert!(significant.contains_key(&Address::from_str("0x2222222222222222222222222222222222222222").unwrap()));
    }
    
    #[test]
    fn test_top_movers() {
        let analyzer = StateChangeAnalyzer::new();
        
        let mut state_changes = HashMap::new();
        
        // Gainer
        let mut gainer = AddressStateChange::new(
            Address::from_str("0x1111111111111111111111111111111111111111").unwrap()
        );
        gainer.usd_value_change = 1000.0;
        state_changes.insert(gainer.address, gainer);
        
        // Loser
        let mut loser = AddressStateChange::new(
            Address::from_str("0x2222222222222222222222222222222222222222").unwrap()
        );
        loser.usd_value_change = -500.0;
        state_changes.insert(loser.address, loser);
        
        let (gainers, losers) = analyzer.get_top_movers(&state_changes, 10);
        
        assert_eq!(gainers.len(), 1);
        assert_eq!(losers.len(), 1);
        assert_eq!(gainers[0].1, 1000.0);
        assert_eq!(losers[0].1, -500.0);
    }
}