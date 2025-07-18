//! Types for graph discovery

use alloy_primitives::{Address, TxHash, U256};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Undirected graph node representing an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub address: Address,
    pub entity_type: Option<String>,
    pub is_contract: bool,
    pub name: Option<String>,
    pub explored: bool,           // Have we explored transactions from this address?
    pub exploration_depth: u32,   // How many hops from seed
    pub first_seen_block: u64,    // When we first encountered this address
}

/// Undirected edge representing a connection via transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub node1: Address,          // One end of connection
    pub node2: Address,          // Other end of connection  
    pub tx_hash: TxHash,
    pub value: U256,             // For prioritization only
    pub block_number: u64,
}

impl GraphEdge {
    /// Check if this edge connects two addresses
    pub fn connects(&self, addr1: &Address, addr2: &Address) -> bool {
        (self.node1 == *addr1 && self.node2 == *addr2) ||
        (self.node1 == *addr2 && self.node2 == *addr1)
    }
}

/// Preliminary undirected graph from discovery
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PreliminaryGraph {
    pub nodes: HashMap<Address, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl PreliminaryGraph {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.address, node);
    }
    
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }
    
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    
    pub fn has_node(&self, address: &Address) -> bool {
        self.nodes.contains_key(address)
    }
}

/// Transaction candidate for deep analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxCandidate {
    pub tx_hash: TxHash,
    pub from_address: Address,
    pub to_address: Address,
    pub value: U256,
    pub block_number: u64,
    pub priority_score: f64,      // Calculated priority for processing
    pub involves_unknown: bool,   // Involves addresses with unknown entity type
    pub involves_router: bool,    // Involves DEX router or aggregator
}

/// Configuration for graph discovery
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub max_depth: u32,                // Maximum hops from seed
    pub max_nodes: usize,              // Maximum nodes to discover
    pub min_value_wei: U256,           // Minimum transaction value
    pub max_txs_per_address: usize,    // Limit transactions per address
    pub time_window_blocks: Option<u64>, // Optional: only recent blocks
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_nodes: 500,
            min_value_wei: U256::from(10).pow(U256::from(17)), // 0.1 ETH
            max_txs_per_address: 100,
            time_window_blocks: None,
        }
    }
}

/// Output from graph discovery
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryOutput {
    pub graph: PreliminaryGraph,
    pub priority_transactions: Vec<TxCandidate>,
    pub stop_reason: StopReason,
    pub discovery_stats: DiscoveryStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StopReason {
    DepthReached,
    MaxNodesReached,
    NoMoreCandidates,
    TimeWindowExceeded,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_txs_examined: usize,
    pub addresses_explored: usize,
    pub cex_addresses_found: usize,
    pub dex_addresses_found: usize,
    pub unknown_contracts: usize,
    pub time_taken_ms: u64,
}

/// Item in the exploration frontier (for BFS)
#[derive(Debug, Clone)]
pub struct ExplorationItem {
    pub tx_hash: TxHash,
    pub from: Address,
    pub to: Address,
    pub counterparty: Address,  // The address we're exploring towards
    pub value: U256,
    pub depth: u32,
    pub block_number: u64,
}

impl Ord for ExplorationItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher value = higher priority
        self.value.cmp(&other.value)
    }
}

impl PartialOrd for ExplorationItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for ExplorationItem {}

impl PartialEq for ExplorationItem {
    fn eq(&self, other: &Self) -> bool {
        self.tx_hash == other.tx_hash
    }
}