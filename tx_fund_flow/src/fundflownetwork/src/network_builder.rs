//! Network builder for constructing fund flow networks

use crate::fund_flow_analyzer::{FundFlowAnalyzer, NetBalance};
use crate::network_types::*;
use alloy_primitives::Address;
use std::collections::HashMap;
use tracing::{debug, info};
use tx_fund_flow_core_types::{FlowType, FundFlow};

/// Builder for constructing fund flow networks
pub struct NetworkBuilder {
    /// Fund flow analyzer
    analyzer: FundFlowAnalyzer,
    /// Minimum ETH amount for including edges
    min_eth_amount: f64,
    /// Whether to include gas flows
    include_gas: bool,
    /// Node metadata cache
    node_metadata: HashMap<Address, NodeMetadata>,
}

#[derive(Clone)]
struct NodeMetadata {
    label: Option<String>,
    is_contract: bool,
    entity_category: Option<String>,
}

impl NetworkBuilder {
    /// Create a new network builder
    pub fn new() -> Self {
        Self {
            analyzer: FundFlowAnalyzer::new(),
            min_eth_amount: 0.01, // 0.01 ETH minimum
            include_gas: false,
            node_metadata: HashMap::new(),
        }
    }

    /// Set minimum ETH amount for edges
    pub fn with_min_eth_amount(mut self, min_eth: f64) -> Self {
        self.min_eth_amount = min_eth;
        self
    }

    /// Set whether to include gas flows
    pub fn with_gas_inclusion(mut self, include_gas: bool) -> Self {
        self.include_gas = include_gas;
        self.analyzer = self.analyzer.with_gas_inclusion(include_gas);
        self
    }

    /// Add node metadata
    pub fn add_node_metadata(
        &mut self,
        address: Address,
        label: Option<String>,
        is_contract: bool,
        entity_category: Option<String>,
    ) {
        self.node_metadata.insert(
            address,
            NodeMetadata {
                label,
                is_contract,
                entity_category,
            },
        );
    }

    /// Build network from fund flows
    pub fn build_from_flows(&self, fund_flows: &[FundFlow]) -> FundFlowNetwork {
        info!("Building network from {} fund flows", fund_flows.len());

        let mut network = FundFlowNetwork::new();
        let net_balances = self.analyzer.calculate_net_balances(fund_flows);

        // Create nodes from all addresses in flows
        let mut all_addresses = std::collections::HashSet::new();
        for flow in fund_flows {
            if flow.amount_eth >= self.min_eth_amount {
                all_addresses.insert(flow.from);
                all_addresses.insert(flow.to);
            }
        }

        // Add nodes
        for address in all_addresses {
            let node = self.create_node(address, net_balances.get(&address));
            network.add_node(node);
        }

        // Add edges
        for flow in fund_flows {
            if flow.amount_eth >= self.min_eth_amount {
                let edge = self.create_edge(flow);
                network.add_edge(edge);
            }
        }

        info!(
            "Built network with {} nodes and {} edges",
            network.nodes.len(),
            network.edges.len()
        );

        network
    }

    /// Build network centered on a specific address
    pub fn build_centered_network(
        &self,
        center: Address,
        fund_flows: &[FundFlow],
        max_depth: u32,
    ) -> FundFlowNetwork {
        info!(
            "Building network centered on {:?} with max depth {}",
            center, max_depth
        );

        let mut network = FundFlowNetwork::new();
        network.metadata.center_address = Some(center);
        network.metadata.network_depth = max_depth;

        let net_balances = self.analyzer.calculate_net_balances(fund_flows);

        // Build address connections map
        let mut connections: HashMap<Address, Vec<Address>> = HashMap::new();
        for flow in fund_flows {
            if flow.amount_eth >= self.min_eth_amount {
                connections.entry(flow.from).or_default().push(flow.to);
                connections.entry(flow.to).or_default().push(flow.from);
            }
        }

        // BFS to find addresses within max_depth
        let mut visited = std::collections::HashSet::new();
        let mut current_level = vec![center];
        visited.insert(center);

        for depth in 0..=max_depth {
            let mut next_level = Vec::new();

            for addr in current_level {
                // Add node
                let node = self.create_node(addr, net_balances.get(&addr));
                network.add_node(node);

                // Find connected addresses
                if depth < max_depth {
                    if let Some(connected) = connections.get(&addr) {
                        for &next_addr in connected {
                            if visited.insert(next_addr) {
                                next_level.push(next_addr);
                            }
                        }
                    }
                }
            }

            current_level = next_level;
            if current_level.is_empty() {
                break;
            }
        }

        // Add edges between included nodes
        for flow in fund_flows {
            if flow.amount_eth >= self.min_eth_amount
                && network.nodes.contains_key(&flow.from)
                && network.nodes.contains_key(&flow.to)
            {
                let edge = self.create_edge(flow);
                network.add_edge(edge);
            }
        }

        info!(
            "Built centered network with {} nodes and {} edges",
            network.nodes.len(),
            network.edges.len()
        );

        network
    }

    /// Create a network node
    fn create_node(&self, address: Address, balance: Option<&NetBalance>) -> NetworkNode {
        let metadata = self.node_metadata.get(&address);
        let (balance_change, usd_change, tx_count) = if let Some(bal) = balance {
            (bal.net_eth, bal.net_usd, bal.tx_count)
        } else {
            (0.0, 0.0, 0)
        };

        let node_type = NodeType::from_metadata(
            metadata.as_ref().map(|m| m.is_contract).unwrap_or(false),
            metadata.as_ref().and_then(|m| m.entity_category.as_deref()),
            metadata.as_ref().and_then(|m| m.label.as_deref()),
        );

        NetworkNode {
            address,
            node_type,
            label: metadata.and_then(|m| m.label.clone()),
            balance_change,
            usd_value_change: usd_change,
            transaction_count: tx_count,
            is_contract: metadata.map(|m| m.is_contract).unwrap_or(false),
            metadata: HashMap::new(),
        }
    }

    /// Create a network edge
    fn create_edge(&self, flow: &FundFlow) -> NetworkEdge {
        let edge_type = match &flow.flow_type {
            FlowType::DirectTransfer => EdgeType::Direct,
            FlowType::InternalTransfer => EdgeType::Internal,
            FlowType::GasPayment => EdgeType::Gas,
            FlowType::TokenTransfer(_) => EdgeType::TokenAsEth,
            FlowType::ContractInteraction => EdgeType::Internal,
        };

        NetworkEdge {
            from: flow.from,
            to: flow.to,
            eth_amount: flow.amount_eth,
            usd_amount: flow.amount_tokens_usd,
            transaction_count: flow.transaction_count,
            edge_type,
            first_block: flow.first_block,
            last_block: flow.last_block,
        }
    }

    /// Contract addresses that should be removed in zero-net analysis
    pub fn contract_zero_net_nodes(&self, network: &FundFlowNetwork) -> Vec<Address> {
        let mut zero_net_contracts = Vec::new();

        for (address, node) in &network.nodes {
            // Only consider contracts
            if !node.is_contract {
                continue;
            }

            // Check if it's a router/intermediary (near-zero balance change)
            if node.balance_change.abs() < 0.001 && node.transaction_count > 2 {
                // Check if it has both incoming and outgoing flows
                let has_incoming = !network.get_incoming_edges(address).is_empty();
                let has_outgoing = !network.get_outgoing_edges(address).is_empty();

                if has_incoming && has_outgoing {
                    zero_net_contracts.push(*address);
                }
            }
        }

        debug!("Found {} zero-net contract nodes", zero_net_contracts.len());
        zero_net_contracts
    }
}

impl Default for NetworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tx_fund_flow_core_types::FlowType;

    #[test]
    fn test_network_builder() {
        let builder = NetworkBuilder::new();

        let fund_flows = vec![
            FundFlow {
                from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
                to: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
                amount_eth: 1.0,
                amount_tokens_usd: 0.0,
                transaction_count: 1,
                first_block: 100,
                last_block: 100,
                flow_type: FlowType::DirectTransfer,
            },
            FundFlow {
                from: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
                to: Address::from_str("0x3333333333333333333333333333333333333333").unwrap(),
                amount_eth: 0.5,
                amount_tokens_usd: 0.0,
                transaction_count: 1,
                first_block: 101,
                last_block: 101,
                flow_type: FlowType::DirectTransfer,
            },
        ];

        let network = builder.build_from_flows(&fund_flows);

        assert_eq!(network.nodes.len(), 3);
        assert_eq!(network.edges.len(), 2);
        assert_eq!(network.metadata.total_eth_volume, 1.5);
    }

    #[test]
    fn test_centered_network() {
        let builder = NetworkBuilder::new();
        let center = Address::from_str("0x2222222222222222222222222222222222222222").unwrap();

        let fund_flows = vec![
            FundFlow {
                from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
                to: center,
                amount_eth: 1.0,
                amount_tokens_usd: 0.0,
                transaction_count: 1,
                first_block: 100,
                last_block: 100,
                flow_type: FlowType::DirectTransfer,
            },
            FundFlow {
                from: center,
                to: Address::from_str("0x3333333333333333333333333333333333333333").unwrap(),
                amount_eth: 0.5,
                amount_tokens_usd: 0.0,
                transaction_count: 1,
                first_block: 101,
                last_block: 101,
                flow_type: FlowType::DirectTransfer,
            },
            FundFlow {
                from: Address::from_str("0x3333333333333333333333333333333333333333").unwrap(),
                to: Address::from_str("0x4444444444444444444444444444444444444444").unwrap(),
                amount_eth: 0.25,
                amount_tokens_usd: 0.0,
                transaction_count: 1,
                first_block: 102,
                last_block: 102,
                flow_type: FlowType::DirectTransfer,
            },
        ];

        // Depth 1 should include center and direct connections
        let network = builder.build_centered_network(center, &fund_flows, 1);
        assert_eq!(network.nodes.len(), 3); // center + 2 direct connections

        // Depth 2 should include all nodes
        let network = builder.build_centered_network(center, &fund_flows, 2);
        assert_eq!(network.nodes.len(), 4); // all nodes
    }
}
