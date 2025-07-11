//! Fund flow network data structures

use qarqa_core_types::*;
use alloy_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete fund flow network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundFlowNetwork {
    /// Network nodes (addresses)
    pub nodes: HashMap<Address, NetworkNode>,
    /// Network edges (fund flows between addresses)
    pub edges: Vec<NetworkEdge>,
    /// Network statistics
    pub stats: NetworkStatistics,
    /// Network metadata
    pub metadata: NetworkMetadata,
}

/// Network node representing an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    /// Node address
    pub address: Address,
    /// Node type (EOA, contract, etc.)
    pub node_type: NodeType,
    /// Total ETH inflow
    pub total_eth_in: U256,
    /// Total ETH outflow  
    pub total_eth_out: U256,
    /// Net ETH change
    pub net_eth_change: i128,
    /// USD value of net change
    pub usd_value_change: f64,
    /// Incoming transaction count
    pub incoming_tx_count: u64,
    /// Outgoing transaction count
    pub outgoing_tx_count: u64,
    /// First block this address appeared
    pub first_block: u64,
    /// Last block this address appeared
    pub last_block: u64,
    /// Node labels and metadata
    pub labels: Vec<String>,
}

/// Network edge representing fund flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEdge {
    /// Source address
    pub from: Address,
    /// Target address
    pub to: Address,
    /// Total ETH transferred
    pub total_eth: U256,
    /// USD value of transfers
    pub total_usd: f64,
    /// Number of transactions
    pub transaction_count: u64,
    /// First block of transfer
    pub first_block: u64,
    /// Last block of transfer
    pub last_block: u64,
    /// Types of flows in this edge
    pub flow_types: Vec<FlowType>,
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatistics {
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Total ETH volume
    pub total_eth_volume: U256,
    /// Total USD volume
    pub total_usd_volume: f64,
    /// Network density (edges / max_possible_edges)
    pub network_density: f64,
    /// Block range covered
    pub block_range: (u64, u64),
    /// Total transaction count
    pub total_transactions: u64,
}

/// Network metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetadata {
    /// Center address (if network is address-centric)
    pub center_address: Option<Address>,
    /// Network creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Analysis parameters used
    pub analysis_params: AnalysisParams,
    /// Currency used for USD calculations
    pub currency: NetworkCurrency,
}

/// Analysis parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisParams {
    /// Minimum USD value threshold
    pub min_usd_threshold: f64,
    /// Maximum network depth
    pub max_depth: u32,
    /// Whether gas payments are included
    pub include_gas: bool,
    /// Whether WETH is treated as ETH
    pub treat_weth_as_eth: bool,
}

/// Network currency for calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkCurrency {
    ETH,
    USD,
    USDC,
    USDT,
}

/// Node type classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    /// Externally owned account
    ExternalAccount,
    /// Smart contract
    Contract(ContractType),
    /// Known exchange
    Exchange,
    /// Mining pool/validator
    Miner,
    /// Bridge contract
    Bridge,
    /// Unknown type
    Unknown,
}

/// Contract type classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContractType {
    Token,
    DEXRouter,
    DEXPool,
    LendingProtocol,
    NFTMarketplace,
    Bridge,
    Multisig,
    Unknown,
}

impl FundFlowNetwork {
    /// Create a new empty network
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            stats: NetworkStatistics::default(),
            metadata: NetworkMetadata::default(),
        }
    }
    
    /// Add a node to the network
    pub fn add_node(&mut self, node: NetworkNode) {
        self.nodes.insert(node.address, node);
    }
    
    /// Add an edge to the network
    pub fn add_edge(&mut self, edge: NetworkEdge) {
        self.edges.push(edge);
    }
    
    /// Get node by address
    pub fn get_node(&self, address: &Address) -> Option<&NetworkNode> {
        self.nodes.get(address)
    }
    
    /// Get edges from an address
    pub fn get_edges_from(&self, address: &Address) -> Vec<&NetworkEdge> {
        self.edges.iter().filter(|e| e.from == *address).collect()
    }
    
    /// Get edges to an address
    pub fn get_edges_to(&self, address: &Address) -> Vec<&NetworkEdge> {
        self.edges.iter().filter(|e| e.to == *address).collect()
    }
    
    /// Calculate network statistics
    pub fn calculate_statistics(&mut self) {
        let node_count = self.nodes.len();
        let edge_count = self.edges.len();
        
        let total_eth_volume = self.edges.iter()
            .map(|e| e.total_eth)
            .fold(U256::ZERO, |acc, val| acc + val);
        
        let total_usd_volume = self.edges.iter()
            .map(|e| e.total_usd)
            .sum();
        
        let max_edges = if node_count > 1 {
            node_count * (node_count - 1)
        } else {
            1
        };
        let network_density = edge_count as f64 / max_edges as f64;
        
        let block_range = if !self.edges.is_empty() {
            let min_block = self.edges.iter().map(|e| e.first_block).min().unwrap_or(0);
            let max_block = self.edges.iter().map(|e| e.last_block).max().unwrap_or(0);
            (min_block, max_block)
        } else {
            (0, 0)
        };
        
        let total_transactions = self.edges.iter()
            .map(|e| e.transaction_count)
            .sum();
        
        self.stats = NetworkStatistics {
            node_count,
            edge_count,
            total_eth_volume,
            total_usd_volume,
            network_density,
            block_range,
            total_transactions,
        };
    }
    
    /// Filter network by minimum value
    pub fn filter_by_min_value(&self, min_eth: f64, min_usd: f64) -> Self {
        let mut filtered = self.clone();
        
        // Filter edges
        filtered.edges.retain(|edge| {
            let eth_value = wei_to_eth(edge.total_eth);
            eth_value >= min_eth || edge.total_usd >= min_usd
        });
        
        // Remove nodes with no edges
        let mut connected_addresses = std::collections::HashSet::new();
        for edge in &filtered.edges {
            connected_addresses.insert(edge.from);
            connected_addresses.insert(edge.to);
        }
        
        filtered.nodes.retain(|addr, _| connected_addresses.contains(addr));
        
        // Recalculate statistics
        filtered.calculate_statistics();
        
        filtered
    }
    
    /// Convert to Cytoscape format for visualization
    pub fn to_cytoscape_format(&self) -> serde_json::Value {
        use crate::visualization::convert_to_visualization_data;
        
        let viz_data = convert_to_visualization_data(
            self, 
            self.metadata.center_address,
            NetworkCurrency::USD
        );
        
        serde_json::to_value(viz_data).unwrap_or_default()
    }
}

impl Default for FundFlowNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for NetworkStatistics {
    fn default() -> Self {
        Self {
            node_count: 0,
            edge_count: 0,
            total_eth_volume: U256::ZERO,
            total_usd_volume: 0.0,
            network_density: 0.0,
            block_range: (0, 0),
            total_transactions: 0,
        }
    }
}

impl Default for NetworkMetadata {
    fn default() -> Self {
        Self {
            center_address: None,
            created_at: chrono::Utc::now(),
            analysis_params: AnalysisParams::default(),
            currency: NetworkCurrency::USD,
        }
    }
}

impl Default for AnalysisParams {
    fn default() -> Self {
        Self {
            min_usd_threshold: 10.0,
            max_depth: 2,
            include_gas: false,
            treat_weth_as_eth: true,
        }
    }
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
    fn test_network_creation() {
        let mut network = FundFlowNetwork::new();
        
        // Add a node
        let node = NetworkNode {
            address: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
            node_type: NodeType::ExternalAccount,
            total_eth_in: U256::ZERO,
            total_eth_out: U256::ZERO,
            net_eth_change: 0,
            usd_value_change: 0.0,
            incoming_tx_count: 0,
            outgoing_tx_count: 0,
            first_block: 0,
            last_block: 0,
            labels: Vec::new(),
        };
        
        network.add_node(node);
        
        assert_eq!(network.nodes.len(), 1);
        assert!(network.get_node(&Address::from_str("0x1111111111111111111111111111111111111111").unwrap()).is_some());
    }
    
    #[test]
    fn test_network_statistics() {
        let mut network = FundFlowNetwork::new();
        
        // Add edge
        let edge = NetworkEdge {
            from: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
            to: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
            total_eth: U256::from_str("1000000000000000000").unwrap(), // 1 ETH
            total_usd: 2500.0,
            transaction_count: 1,
            first_block: 1,
            last_block: 1,
            flow_types: vec![FlowType::DirectTransfer],
        };
        
        network.add_edge(edge);
        network.calculate_statistics();
        
        assert_eq!(network.stats.edge_count, 1);
        assert_eq!(network.stats.total_usd_volume, 2500.0);
        assert_eq!(network.stats.block_range, (1, 1));
    }
}