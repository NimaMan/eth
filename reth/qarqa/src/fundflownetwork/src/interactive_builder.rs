//! Interactive fund flow network builder with database integration
//! This module allows building networks incrementally by fetching data from eth_db

use crate::fund_flow_analyzer::FundFlowAnalyzer;
use crate::network_builder::NetworkBuilder;
use crate::network_types::*;
use crate::tx_processor_integration::{extract_fund_flows_from_processed_tx, ProcessedTxConverter};
use qarqa_eth_db_fetcher::{AddressFetcher, TransactionFetcher};
use qarqa_core_types::{CompleteFundFlows, FundFlow};
use tx_processor::tx_processor::TxProcessor;
use alloy_primitives::{Address, B256, U256};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};
use eyre::Result;

/// Interactive fund flow network that can be expanded incrementally
pub struct InteractiveFundFlowNetwork {
    /// Current network state
    pub network: FundFlowNetwork,
    /// Network builder
    builder: NetworkBuilder,
    /// Database connection pool
    db_pool: PgPool,
    /// Transaction processor
    tx_processor: TxProcessor,
    /// Cache of processed transactions
    tx_cache: HashMap<String, CompleteFundFlows>,
    /// Addresses that have been fully explored
    explored_addresses: HashSet<Address>,
    /// Configuration
    config: InteractiveConfig,
}

/// Configuration for interactive network building
#[derive(Clone)]
pub struct InteractiveConfig {
    /// Maximum transactions to fetch per address
    pub max_txs_per_address: i64,
    /// Minimum ETH flow to include
    pub min_eth_flow: f64,
    /// Whether to fetch address metadata from db
    pub fetch_metadata: bool,
    /// Whether to use cached transactions
    pub use_cache: bool,
}

impl Default for InteractiveConfig {
    fn default() -> Self {
        Self {
            max_txs_per_address: 100,
            min_eth_flow: 0.01,
            fetch_metadata: true,
            use_cache: true,
        }
    }
}

/// Candidate for network expansion
#[derive(Debug, Clone)]
pub struct ExpansionCandidate {
    pub address: Address,
    pub total_flow: f64,
    pub transaction_count: u64,
    pub depth_from_center: u32,
    pub reason: String,
}

impl InteractiveFundFlowNetwork {
    /// Create a new interactive network
    pub async fn new(
        db_pool: PgPool,
        tx_processor: TxProcessor,
        config: InteractiveConfig,
    ) -> Result<Self> {
        let mut builder = NetworkBuilder::new()
            .with_min_eth_amount(config.min_eth_flow);
        
        Ok(Self {
            network: FundFlowNetwork::new(),
            builder,
            db_pool,
            tx_processor,
            tx_cache: HashMap::new(),
            explored_addresses: HashSet::new(),
            config,
        })
    }
    
    /// Initialize network from a center address
    pub async fn initialize_from_address(&mut self, center: Address) -> Result<()> {
        info!("Initializing network from center address: {:?}", center);
        
        self.network = FundFlowNetwork::new();
        self.network.metadata.center_address = Some(center);
        self.explored_addresses.clear();
        
        // Expand the center address
        self.expand_address(center, 0).await?;
        
        Ok(())
    }
    
    /// Expand network by exploring an address
    pub async fn expand_address(&mut self, address: Address, depth: u32) -> Result<Vec<Address>> {
        if self.explored_addresses.contains(&address) {
            debug!("Address already explored: {:?}", address);
            return Ok(Vec::new());
        }
        
        info!("Expanding network from address: {:?} at depth {}", address, depth);
        
        // Fetch transactions for this address
        let fund_flows = self.fetch_address_fund_flows(address).await?;
        
        // Add to network
        let new_addresses = self.add_flows_to_network(&fund_flows, depth);
        
        // Mark as explored
        self.explored_addresses.insert(address);
        
        // Update network depth
        if depth > self.network.metadata.network_depth {
            self.network.metadata.network_depth = depth;
        }
        
        info!("Added {} new addresses from expansion", new_addresses.len());
        Ok(new_addresses)
    }
    
    /// Fetch and process transactions for an address
    async fn fetch_address_fund_flows(&mut self, address: Address) -> Result<Vec<CompleteFundFlows>> {
        let address_fetcher = AddressFetcher::new(self.db_pool.clone());
        let tx_fetcher = TransactionFetcher::new(self.db_pool.clone());
        
        // Fetch address metadata if configured
        if self.config.fetch_metadata {
            if let Ok(Some(addr_record)) = address_fetcher.get_address(&format!("{:?}", address)).await {
                self.builder.add_node_metadata(
                    address,
                    addr_record.name.clone(),
                    addr_record.is_contract,
                    addr_record.entity_category.clone(),
                );
            }
        }
        
        // Get transactions
        let addr_txs = address_fetcher.get_address_transactions(
            &format!("{:?}", address),
            Some(self.config.max_txs_per_address),
            None,
        ).await?;
        
        info!("Found {} transactions for address", addr_txs.tx_hashes.len());
        
        let mut fund_flows = Vec::new();
        
        for tx_hash in &addr_txs.tx_hashes {
            // Check cache first
            if self.config.use_cache {
                if let Some(cached) = self.tx_cache.get(tx_hash) {
                    fund_flows.push(cached.clone());
                    continue;
                }
            }
            
            // Fetch transaction details
            match tx_fetcher.get_transaction(tx_hash).await? {
                Some(tx_record) => {
                    // Process transaction - convert types
                    let tx_hash_bytes = B256::from_slice(&hex::decode(&tx_hash[2..]).unwrap_or_default());
                    let from_addr = Address::from_slice(&hex::decode(&tx_record.from_address[2..]).unwrap_or_default());
                    let to_addr = tx_record.to_address.as_ref()
                        .map(|addr| Address::from_slice(&hex::decode(&addr[2..]).unwrap_or_default()));
                    let value = U256::from_str_radix(&tx_record.value, 10).unwrap_or_default();
                    let gas_price = U256::from_str_radix(&tx_record.gas_price, 10).unwrap_or_default();
                    let input_data = hex::decode(&tx_record.input_data[2..]).unwrap_or_default();
                    
                    // For now, we'll use dummy values for missing fields - this should be improved
                    match self.tx_processor.process_transaction(
                        tx_hash_bytes,
                        tx_record.block_number as u64,
                        0, // block_timestamp - not in our DB model
                        tx_record.position_in_block as u64,
                        from_addr,
                        to_addr,
                        value,
                        input_data,
                        gas_price,
                        tx_record.gas_used.unwrap_or(21000) as u64,
                        tx_record.status.unwrap_or_else(|| "1".to_string()),
                        0, // nonce - not in our DB model
                        Vec::new(), // logs - not in our simple model
                    ).await {
                        Ok(processed) => {
                            match extract_fund_flows_from_processed_tx(&processed) {
                                Ok(flows) => {
                                    if self.config.use_cache {
                                        self.tx_cache.insert(tx_hash.clone(), flows.clone());
                                    }
                                    fund_flows.push(flows);
                                }
                                Err(e) => warn!("Failed to extract fund flows from tx {}: {}", tx_hash, e),
                            }
                        }
                        Err(e) => warn!("Failed to process transaction {}: {}", tx_hash, e),
                    }
                }
                None => warn!("Transaction not found in db: {}", tx_hash),
            }
        }
        
        Ok(fund_flows)
    }
    
    /// Add fund flows to network and return new addresses
    fn add_flows_to_network(&mut self, complete_flows: &[CompleteFundFlows], _depth: u32) -> Vec<Address> {
        let mut new_addresses = Vec::new();
        
        // Convert to fund flows
        let analyzer = FundFlowAnalyzer::new()
            .with_min_value(alloy_primitives::U256::from((self.config.min_eth_flow * 1e18) as u128));
        let fund_flows = analyzer.analyze_fund_flows(complete_flows);
        
        // Track which addresses are new
        for flow in &fund_flows {
            if !self.network.nodes.contains_key(&flow.from) {
                new_addresses.push(flow.from);
            }
            if !self.network.nodes.contains_key(&flow.to) {
                new_addresses.push(flow.to);
            }
        }
        
        // Build partial network and merge
        let partial = self.builder.build_from_flows(&fund_flows);
        
        // Merge nodes
        for (addr, node) in partial.nodes {
            if !self.network.nodes.contains_key(&addr) {
                self.network.add_node(node);
            }
        }
        
        // Merge edges
        for edge in partial.edges {
            self.network.add_edge(edge);
        }
        
        // Deduplicate new addresses
        new_addresses.sort();
        new_addresses.dedup();
        
        new_addresses
    }
    
    /// Get expansion candidates ranked by importance
    pub fn get_expansion_candidates(&self, limit: usize) -> Vec<ExpansionCandidate> {
        let mut candidates = Vec::new();
        
        // Find addresses that are in the network but not explored
        for (address, node) in &self.network.nodes {
            if self.explored_addresses.contains(address) {
                continue;
            }
            
            // Skip CEX addresses - don't expand beyond centralized exchanges
            if node.node_type == NodeType::CEX {
                debug!("Skipping CEX address for expansion: {:?}", address);
                continue;
            }
            
            // Calculate total flow through this address
            let incoming_flow: f64 = self.network.get_incoming_edges(address)
                .iter()
                .map(|e| e.eth_amount)
                .sum();
            
            let outgoing_flow: f64 = self.network.get_outgoing_edges(address)
                .iter()
                .map(|e| e.eth_amount)
                .sum();
            
            let total_flow = incoming_flow + outgoing_flow;
            
            // Determine depth from center
            let depth = self.calculate_depth_from_center(*address);
            
            // Create candidate
            let reason = match node.node_type {
                NodeType::CEX => "Centralized Exchange".to_string(),
                NodeType::DEX => "Decentralized Exchange".to_string(),
                NodeType::DeFi => "DeFi Protocol".to_string(),
                _ if total_flow > 100.0 => "High Volume".to_string(),
                _ if node.transaction_count > 50 => "High Activity".to_string(),
                _ => "Connected Address".to_string(),
            };
            
            candidates.push(ExpansionCandidate {
                address: *address,
                total_flow,
                transaction_count: node.transaction_count,
                depth_from_center: depth,
                reason,
            });
        }
        
        // Sort by total flow (descending)
        candidates.sort_by(|a, b| {
            b.total_flow.partial_cmp(&a.total_flow)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        candidates.truncate(limit);
        candidates
    }
    
    /// Calculate depth from center address using BFS
    fn calculate_depth_from_center(&self, target: Address) -> u32 {
        if let Some(center) = self.network.metadata.center_address {
            if center == target {
                return 0;
            }
            
            // BFS to find shortest path
            let mut visited = HashSet::new();
            let mut queue = vec![(center, 0u32)];
            visited.insert(center);
            
            while let Some((current, depth)) = queue.pop() {
                // Check all connected addresses
                for edge in &self.network.edges {
                    let next = if edge.from == current {
                        Some(edge.to)
                    } else if edge.to == current {
                        Some(edge.from)
                    } else {
                        None
                    };
                    
                    if let Some(next_addr) = next {
                        if next_addr == target {
                            return depth + 1;
                        }
                        
                        if visited.insert(next_addr) {
                            queue.push((next_addr, depth + 1));
                        }
                    }
                }
            }
        }
        
        // Not connected or no center
        u32::MAX
    }
    
    /// Expand network by exploring top candidates
    pub async fn expand_top_candidates(&mut self, count: usize) -> Result<Vec<Address>> {
        let candidates = self.get_expansion_candidates(count);
        let mut all_new_addresses = Vec::new();
        
        for candidate in candidates {
            info!("Expanding candidate: {:?} ({})", candidate.address, candidate.reason);
            let new_addresses = self.expand_address(
                candidate.address, 
                candidate.depth_from_center + 1
            ).await?;
            all_new_addresses.extend(new_addresses);
        }
        
        // Deduplicate
        all_new_addresses.sort();
        all_new_addresses.dedup();
        
        Ok(all_new_addresses)
    }
    
    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        self.network.calculate_stats()
    }
    
    /// Export current network state
    pub fn export_network(&self) -> &FundFlowNetwork {
        &self.network
    }
}