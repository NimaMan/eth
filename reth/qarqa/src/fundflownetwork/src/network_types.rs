//! Network types for fund flow network representation

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fund flow network graph structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundFlowNetwork {
    pub nodes: HashMap<Address, NetworkNode>,
    pub edges: Vec<NetworkEdge>,
    pub metadata: NetworkMetadata,
}

/// Node in the fund flow network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    pub address: Address,
    pub node_type: NodeType,
    pub label: Option<String>,
    pub balance_change: f64, // ETH balance change
    pub usd_value_change: f64,
    pub transaction_count: u64,
    pub is_contract: bool,
    pub metadata: HashMap<String, String>,
}

/// Edge in the fund flow network (directed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEdge {
    pub from: Address,
    pub to: Address,
    pub eth_amount: f64,
    pub usd_amount: f64,
    pub transaction_count: u64,
    pub edge_type: EdgeType,
    pub first_block: u64,
    pub last_block: u64,
}

/// Type of node in the network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// Externally owned account
    EOA,
    /// Smart contract
    Contract,
    /// Centralized exchange
    CEX,
    /// Decentralized exchange
    DEX,
    /// DeFi protocol
    DeFi,
    /// Token contract
    Token,
    /// Unknown/unclassified
    Unknown,
}

/// Type of edge/flow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Direct ETH transfer
    Direct,
    /// Internal transaction
    Internal,
    /// Token transfer (treated as ETH)
    TokenAsEth,
    /// Gas payment
    Gas,
    /// Aggregated flows
    Aggregated,
}

/// Network metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetadata {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub total_eth_volume: f64,
    pub total_usd_volume: f64,
    pub block_range: (u64, u64),
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub network_depth: u32,
    pub center_address: Option<Address>,
}

impl FundFlowNetwork {
    /// Create a new empty network
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            metadata: NetworkMetadata {
                total_nodes: 0,
                total_edges: 0,
                total_eth_volume: 0.0,
                total_usd_volume: 0.0,
                block_range: (0, 0),
                created_at: chrono::Utc::now(),
                network_depth: 0,
                center_address: None,
            },
        }
    }
    
    /// Add a node to the network
    pub fn add_node(&mut self, node: NetworkNode) {
        self.nodes.insert(node.address, node);
        self.metadata.total_nodes = self.nodes.len();
    }
    
    /// Add an edge to the network
    pub fn add_edge(&mut self, edge: NetworkEdge) {
        self.metadata.total_eth_volume += edge.eth_amount;
        self.metadata.total_usd_volume += edge.usd_amount;
        
        // Update block range
        if self.metadata.block_range.0 == 0 || edge.first_block < self.metadata.block_range.0 {
            self.metadata.block_range.0 = edge.first_block;
        }
        if edge.last_block > self.metadata.block_range.1 {
            self.metadata.block_range.1 = edge.last_block;
        }
        
        self.edges.push(edge);
        self.metadata.total_edges = self.edges.len();
    }
    
    /// Get node by address
    pub fn get_node(&self, address: &Address) -> Option<&NetworkNode> {
        self.nodes.get(address)
    }
    
    /// Get all edges from a specific address
    pub fn get_outgoing_edges(&self, address: &Address) -> Vec<&NetworkEdge> {
        self.edges.iter()
            .filter(|edge| &edge.from == address)
            .collect()
    }
    
    /// Get all edges to a specific address
    pub fn get_incoming_edges(&self, address: &Address) -> Vec<&NetworkEdge> {
        self.edges.iter()
            .filter(|edge| &edge.to == address)
            .collect()
    }
    
    /// Calculate network statistics
    pub fn calculate_stats(&self) -> NetworkStats {
        let mut total_in_degree = 0;
        let mut total_out_degree = 0;
        let mut max_in_degree = 0;
        let mut max_out_degree = 0;
        let mut max_balance_change: f64 = 0.0;
        let mut min_balance_change: f64 = 0.0;
        
        for node in self.nodes.values() {
            let in_degree = self.get_incoming_edges(&node.address).len();
            let out_degree = self.get_outgoing_edges(&node.address).len();
            
            total_in_degree += in_degree;
            total_out_degree += out_degree;
            max_in_degree = max_in_degree.max(in_degree);
            max_out_degree = max_out_degree.max(out_degree);
            
            max_balance_change = max_balance_change.max(node.balance_change);
            min_balance_change = min_balance_change.min(node.balance_change);
        }
        
        let avg_degree = if self.nodes.is_empty() {
            0.0
        } else {
            (total_in_degree + total_out_degree) as f64 / (2.0 * self.nodes.len() as f64)
        };
        
        NetworkStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
            avg_degree,
            max_in_degree,
            max_out_degree,
            total_volume_eth: self.metadata.total_eth_volume,
            total_volume_usd: self.metadata.total_usd_volume,
            max_balance_change,
            min_balance_change,
        }
    }
}

impl Default for FundFlowNetwork {
    fn default() -> Self {
        Self::new()
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub avg_degree: f64,
    pub max_in_degree: usize,
    pub max_out_degree: usize,
    pub total_volume_eth: f64,
    pub total_volume_usd: f64,
    pub max_balance_change: f64,
    pub min_balance_change: f64,
}

impl NodeType {
    /// Determine node type from metadata
    pub fn from_metadata(is_contract: bool, entity_category: Option<&str>, name: Option<&str>) -> Self {
        if let Some(category) = entity_category {
            match category.to_uppercase().as_str() {
                "CEX" => return NodeType::CEX,
                "DEX" => return NodeType::DEX,
                "DEFI" => return NodeType::DeFi,
                "TOKEN" => return NodeType::Token,
                _ => {}
            }
        }
        
        if let Some(name) = name {
            let name_upper = name.to_uppercase();
            if name_upper.contains("EXCHANGE") || name_upper.contains("BINANCE") || 
               name_upper.contains("COINBASE") || name_upper.contains("KRAKEN") {
                return NodeType::CEX;
            }
            if name_upper.contains("UNISWAP") || name_upper.contains("SUSHISWAP") || 
               name_upper.contains("CURVE") {
                return NodeType::DEX;
            }
            if name_upper.contains("AAVE") || name_upper.contains("COMPOUND") || 
               name_upper.contains("MAKER") {
                return NodeType::DeFi;
            }
        }
        
        if is_contract {
            NodeType::Contract
        } else {
            NodeType::EOA
        }
    }
}