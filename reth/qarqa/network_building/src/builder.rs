//! Network builder - constructs networks from fund flows

use qarqa_core_types::*;
use qarqa_tx_simulation::{FundFlowAnalyzer, StateChangeAnalyzer, AddressStateChange};
use crate::network::*;
use alloy_primitives::{Address, U256};
use tracing::{debug, info};

/// Network builder
#[derive(Clone)]
pub struct NetworkBuilder {
    analyzer: FundFlowAnalyzer,
    state_analyzer: StateChangeAnalyzer,
    params: AnalysisParams,
}

impl NetworkBuilder {
    /// Create a new network builder
    pub fn new() -> Self {
        Self {
            analyzer: FundFlowAnalyzer::new(),
            state_analyzer: StateChangeAnalyzer::new(),
            params: AnalysisParams::default(),
        }
    }
    
    /// Set analysis parameters
    pub fn with_params(mut self, params: AnalysisParams) -> Self {
        // Configure analyzers based on params
        self.analyzer = FundFlowAnalyzer::new()
            .with_gas_inclusion(params.include_gas)
            .with_weth_as_eth(params.treat_weth_as_eth);
        
        self.params = params;
        self
    }
    
    /// Build network from fund flows
    pub fn build_from_fund_flows(
        &self,
        fund_flows: &[FundFlow],
        center_address: Option<Address>,
    ) -> QarqaResult<FundFlowNetwork> {
        debug!("Building network from {} fund flows", fund_flows.len());
        
        let mut network = FundFlowNetwork::new();
        
        // Set metadata
        network.metadata.center_address = center_address;
        network.metadata.analysis_params = self.params.clone();
        
        // Calculate state changes
        let state_changes = self.state_analyzer.calculate_state_changes(fund_flows);
        
        // Build nodes
        for (address, state_change) in &state_changes {
            if state_change.is_significant(self.params.min_usd_threshold) {
                let node = self.create_network_node(*address, state_change);
                network.add_node(node);
            }
        }
        
        // Build edges
        for flow in fund_flows {
            if should_include_flow(flow, &self.params) {
                let edge = self.create_network_edge(flow);
                network.add_edge(edge);
            }
        }
        
        // Calculate statistics
        network.calculate_statistics();
        
        info!("Built network with {} nodes and {} edges", 
              network.stats.node_count, network.stats.edge_count);
        
        Ok(network)
    }
    
    /// Create network node from state change
    fn create_network_node(&self, address: Address, state_change: &AddressStateChange) -> NetworkNode {
        // Determine node type (simplified)
        let node_type = classify_address(address);
        
        // Calculate ETH values
        let (total_eth_in, total_eth_out) = if state_change.eth_change >= 0 {
            (U256::from(state_change.eth_change as u128), U256::ZERO)
        } else {
            (U256::ZERO, U256::from((-state_change.eth_change) as u128))
        };
        
        NetworkNode {
            address,
            node_type,
            total_eth_in,
            total_eth_out,
            net_eth_change: state_change.eth_change,
            usd_value_change: state_change.usd_value_change,
            incoming_tx_count: 0, // Will be calculated during network analysis
            outgoing_tx_count: 0, // Will be calculated during network analysis
            first_block: state_change.first_block,
            last_block: state_change.last_block,
            labels: generate_labels(address, &node_type),
        }
    }
    
    /// Create network edge from fund flow
    fn create_network_edge(&self, flow: &FundFlow) -> NetworkEdge {
        let total_eth = U256::from((flow.amount_eth * 1e18) as u64);
        
        NetworkEdge {
            from: flow.from,
            to: flow.to,
            total_eth,
            total_usd: flow.amount_tokens_usd,
            transaction_count: flow.transaction_count,
            first_block: flow.first_block,
            last_block: flow.last_block,
            flow_types: vec![flow.flow_type.clone()],
        }
    }
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a flow should be included based on parameters
fn should_include_flow(flow: &FundFlow, params: &AnalysisParams) -> bool {
    // Check gas inclusion
    if !params.include_gas && matches!(flow.flow_type, FlowType::GasPayment) {
        return false;
    }
    
    // Check minimum threshold
    let usd_value = flow.amount_eth * 2500.0; // Simplified ETH rate
    if usd_value < params.min_usd_threshold {
        return false;
    }
    
    true
}

/// Classify address type (simplified implementation)
fn classify_address(address: Address) -> NodeType {
    // This would normally use a comprehensive classification service
    // For now, just basic heuristics
    
    // Check for known addresses (simplified)
    let address_str = format!("{:?}", address);
    
    if is_known_exchange(&address_str) {
        NodeType::Exchange
    } else if is_known_contract(&address_str) {
        NodeType::Contract(ContractType::Unknown)
    } else {
        NodeType::ExternalAccount
    }
}

/// Check if address is a known exchange (simplified)
fn is_known_exchange(_address: &str) -> bool {
    // Would normally check against a database of known exchanges
    false
}

/// Check if address is a contract (simplified)
fn is_known_contract(_address: &str) -> bool {
    // Would normally check contract code
    false
}

/// Generate labels for an address
fn generate_labels(address: Address, node_type: &NodeType) -> Vec<String> {
    let mut labels = Vec::new();
    
    // Add type-based labels
    match node_type {
        NodeType::ExternalAccount => labels.push("EOA".to_string()),
        NodeType::Contract(contract_type) => {
            labels.push("Contract".to_string());
            labels.push(format!("{:?}", contract_type));
        }
        NodeType::Exchange => labels.push("Exchange".to_string()),
        NodeType::Miner => labels.push("Miner".to_string()),
        NodeType::Bridge => labels.push("Bridge".to_string()),
        NodeType::Unknown => labels.push("Unknown".to_string()),
    }
    
    // Add address short form
    let addr_str = format!("{:?}", address);
    labels.push(format!("{}...{}", &addr_str[0..6], &addr_str[addr_str.len()-4..]));
    
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;
    use std::str::FromStr;
    
    #[test]
    fn test_network_builder() {
        let builder = NetworkBuilder::new();
        
        let fund_flows = vec![
            FundFlow {
                from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
                to: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
                amount_eth: 1.0,
                amount_tokens_usd: 2500.0,
                transaction_count: 1,
                first_block: 1,
                last_block: 1,
                flow_type: FlowType::DirectTransfer,
            }
        ];
        
        let network = builder.build_from_fund_flows(&fund_flows, None).unwrap();
        
        assert_eq!(network.stats.edge_count, 1);
        // Nodes are filtered by significance, might be 0 if below threshold
    }
}