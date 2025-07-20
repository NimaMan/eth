//! Main graph explorer that orchestrates the discovery process

use crate::graph_discovery::{
    types::*,
    db_queries::{GraphDiscoveryQueries, AddressInfo},
    routing_rules::RoutingRules,
    tx_selector::TxSelector,
};
use alloy_primitives::Address;
use std::collections::{BinaryHeap, HashSet};
use std::time::Instant;
use sqlx::PgPool;
use eyre::Result;
use tracing::{info, debug};

pub struct GraphExplorer {
    queries: GraphDiscoveryQueries,
}

impl GraphExplorer {
    pub fn new(pool: PgPool) -> Self {
        Self {
            queries: GraphDiscoveryQueries::new(pool),
        }
    }
    
    /// Main exploration function - discovers graph structure from a seed address
    pub async fn explore(
        &self,
        seed: Address,
        config: DiscoveryConfig,
        already_explored: &HashSet<Address>,
    ) -> Result<DiscoveryOutput> {
        let start_time = Instant::now();
        let mut stats = DiscoveryStats::default();
        
        info!("Starting graph discovery from seed: {:?}", seed);
        info!("Config: max_depth={}, max_nodes={}, min_value={}", 
              config.max_depth, config.max_nodes, config.min_value_wei);
        
        let mut graph = UndirectedGraph::new();
        let priority_txs;
        let mut tx_candidates = Vec::new();
        let mut frontier = BinaryHeap::new();
        let mut visited = already_explored.clone();
        
        // Add seed node
        let seed_info = self.queries.get_address_info(&seed).await?;
        graph.add_node(GraphNode {
            address: seed,
            entity_type: seed_info.entity_type.clone(),
            is_contract: seed_info.is_contract,
            name: seed_info.name.clone(),
            explored: false,
            exploration_depth: 0,
            first_seen_block: 0, // Will be updated
        });
        
        info!("Seed address type: {:?}", seed_info.entity_type);
        
        // Get seed transactions
        let seed_txs = self.queries
            .get_address_transactions(
                &seed,
                &config.min_value_wei,
                config.max_txs_per_address,
                config.max_block_number,
            )
            .await?;
            
        info!("Found {} transactions for seed address", seed_txs.len());
        stats.total_txs_examined += seed_txs.len();
        
        // Initialize frontier
        for tx in seed_txs {
            let counterparty = if tx.from_address == seed {
                tx.to_address
            } else {
                tx.from_address
            };
            
            frontier.push(FrontierItem {
                tx_hash: tx.tx_hash,
                from: tx.from_address,
                to: tx.to_address,
                counterparty,
                value: tx.value,
                depth: 1,
                block_number: tx.block_number,
            });
            
            // Update first seen block
            if let Some(node) = graph.nodes.get_mut(&seed) {
                if node.first_seen_block == 0 || tx.block_number < node.first_seen_block {
                    node.first_seen_block = tx.block_number;
                }
            }
        }
        
        // Mark seed as being explored
        if let Some(node) = graph.nodes.get_mut(&seed) {
            node.explored = true;
        }
        stats.addresses_explored += 1;
        
        // BFS exploration
        let stop_reason = self.explore_bfs(
            &mut graph,
            &mut tx_candidates,
            &mut frontier,
            &mut visited,
            &config,
            &mut stats,
        ).await?;
        
        // Select priority transactions for deep analysis
        priority_txs = TxSelector::select_top_transactions(
            tx_candidates,
            100, // Max transactions to deeply analyze
        );
        
        info!("Discovery complete. Found {} nodes, {} edges, selected {} priority txs",
              graph.node_count(), graph.edge_count(), priority_txs.len());
        
        stats.time_taken_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(DiscoveryOutput {
            graph,
            priority_transactions: priority_txs,
            stop_reason,
            discovery_stats: stats,
        })
    }
    
    async fn explore_bfs(
        &self,
        graph: &mut UndirectedGraph,
        tx_candidates: &mut Vec<TxCandidate>,
        frontier: &mut BinaryHeap<FrontierItem>,
        visited: &mut HashSet<Address>,
        config: &DiscoveryConfig,
        stats: &mut DiscoveryStats,
    ) -> Result<StopReason> {
        while let Some(item) = frontier.pop() {
            // Check depth limit
            if item.depth > config.max_depth {
                debug!("Depth limit reached for {:?}", item.counterparty);
                continue;
            }
            
            // Check node limit
            if graph.node_count() >= config.max_nodes {
                return Ok(StopReason::MaxNodesReached);
            }
            
            // Skip if already visited
            if visited.contains(&item.counterparty) {
                continue;
            }
            
            debug!("Exploring {:?} at depth {}", item.counterparty, item.depth);
            
            // Get counterparty info
            let counterparty_info = self.queries
                .get_address_info(&item.counterparty)
                .await?;
            
            // Update stats
            match counterparty_info.entity_type.as_deref() {
                Some(t) if t.contains("CEX") => stats.cex_addresses_found += 1,
                Some(t) if t.contains("DEX") => stats.dex_addresses_found += 1,
                _ if counterparty_info.is_contract && counterparty_info.entity_type.is_none() => {
                    stats.unknown_contracts += 1;
                }
                _ => {}
            }
            
            // Add node if new
            if !graph.has_node(&item.counterparty) {
                graph.add_node(GraphNode {
                    address: item.counterparty,
                    entity_type: counterparty_info.entity_type.clone(),
                    is_contract: counterparty_info.is_contract,
                    name: counterparty_info.name.clone(),
                    explored: false,
                    exploration_depth: item.depth,
                    first_seen_block: item.block_number,
                });
            }
            
            // Add undirected edge
            graph.add_edge(GraphEdge {
                node1: item.from,
                node2: item.to,
                tx_hash: item.tx_hash,
                value: item.value,
                block_number: item.block_number,
            });
            
            // Get info for both sides of transaction
            let from_info = if item.from == item.counterparty {
                counterparty_info.clone()
            } else {
                self.queries.get_address_info(&item.from).await?
            };
            
            let to_info = if item.to == item.counterparty {
                counterparty_info.clone()
            } else {
                self.queries.get_address_info(&item.to).await?
            };
            
            // Check if we should analyze this transaction deeply
            if RoutingRules::should_analyze_deeply(&from_info, &to_info, &item.value) {
                tx_candidates.push(TxCandidate {
                    tx_hash: item.tx_hash,
                    from_address: item.from,
                    to_address: item.to,
                    value: item.value,
                    block_number: item.block_number,
                    priority_score: 0.0, // Will be calculated later
                    involves_unknown: from_info.entity_type.is_none() || to_info.entity_type.is_none(),
                    involves_router: RoutingRules::is_router(&from_info) || RoutingRules::is_router(&to_info),
                });
            }
            
            // Check if we should expand from this address
            if RoutingRules::should_expand(&counterparty_info) && item.depth < config.max_depth {
                debug!("Expanding from {:?} (type: {:?})", 
                       item.counterparty, counterparty_info.entity_type);
                
                // Get more transactions
                let new_txs = self.queries
                    .get_address_transactions(
                        &item.counterparty,
                        &config.min_value_wei,
                        config.max_txs_per_address,
                        config.max_block_number,
                    )
                    .await?;
                
                stats.total_txs_examined += new_txs.len();
                
                for tx in new_txs {
                    let next_counterparty = if tx.from_address == item.counterparty {
                        tx.to_address
                    } else {
                        tx.from_address
                    };
                    
                    // Skip if already visited
                    if visited.contains(&next_counterparty) {
                        continue;
                    }
                    
                    frontier.push(FrontierItem {
                        tx_hash: tx.tx_hash,
                        from: tx.from_address,
                        to: tx.to_address,
                        counterparty: next_counterparty,
                        value: tx.value,
                        depth: item.depth + 1,
                        block_number: tx.block_number,
                    });
                }
                
                // Mark as explored
                if let Some(node) = graph.nodes.get_mut(&item.counterparty) {
                    node.explored = true;
                }
                stats.addresses_explored += 1;
            } else {
                debug!("Not expanding from {:?} (type: {:?})", 
                       item.counterparty, counterparty_info.entity_type);
            }
            
            visited.insert(item.counterparty);
        }
        
        Ok(StopReason::NoMoreCandidates)
    }
}

// Note: is_router method is already defined in routing_rules.rs