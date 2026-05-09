//! Discovery-driven builder for directed fund-flow networks.
//!
//! Graph discovery stays cheap and broad. Only the selected transaction hashes are
//! replayed through `tx_processor`, then converted into directed fund-flow edges.

use crate::graph_discovery::{DiscoveryConfig, DiscoveryOutput, GraphExplorer};
use crate::{
    extract_fund_flows_from_processed_tx, FundFlowAnalyzer, FundFlowNetwork, NetworkBuilder,
};
use alloy_primitives::{Address, TxHash, U256};
use eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashSet;
use tx_fund_flow_core_types::{CompleteFundFlows, FundFlow};
use tx_processor::ProcessedTxProvider;

pub const DEFAULT_MAX_PROCESSED_TXS: usize = 100;

#[derive(Debug, Clone)]
pub struct FundFlowBuildConfig {
    pub discovery: DiscoveryConfig,
    pub max_processed_txs: usize,
    pub min_edge_eth: f64,
    pub include_gas: bool,
    pub include_tokens: bool,
}

impl Default for FundFlowBuildConfig {
    fn default() -> Self {
        let discovery = DiscoveryConfig::default();
        Self {
            min_edge_eth: wei_to_eth_f64(discovery.min_value_wei),
            discovery,
            max_processed_txs: DEFAULT_MAX_PROCESSED_TXS,
            include_gas: false,
            include_tokens: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveredFundFlowNetwork {
    pub discovery: DiscoveryOutput,
    pub complete_flows: Vec<CompleteFundFlows>,
    pub fund_flows: Vec<FundFlow>,
    pub network: FundFlowNetwork,
    pub processing: FundFlowProcessingStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FundFlowProcessingStats {
    pub selected_transactions: usize,
    pub processed_transactions: usize,
    pub failed_transactions: usize,
    pub extracted_complete_flows: usize,
    pub aggregated_flows: usize,
    pub network_nodes: usize,
    pub network_edges: usize,
    pub failure_samples: Vec<TransactionProcessingFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionProcessingFailure {
    pub tx_hash: TxHash,
    pub stage: String,
    pub error: String,
}

pub struct ProcessedFundFlowNetworkBuilder {
    explorer: GraphExplorer,
    tx_provider: ProcessedTxProvider,
}

impl ProcessedFundFlowNetworkBuilder {
    pub fn new(db_pool: PgPool, reth_datadir: &str) -> Result<Self> {
        let tx_provider = ProcessedTxProvider::new(reth_datadir)
            .wrap_err_with(|| format!("failed to initialize tx_processor from {reth_datadir}"))?;
        Ok(Self::with_provider(db_pool, tx_provider))
    }

    pub fn with_provider(db_pool: PgPool, tx_provider: ProcessedTxProvider) -> Self {
        Self {
            explorer: GraphExplorer::new(db_pool),
            tx_provider,
        }
    }

    pub async fn build_from_address(
        &self,
        seed: Address,
        config: FundFlowBuildConfig,
        already_explored: &HashSet<Address>,
    ) -> Result<DiscoveredFundFlowNetwork> {
        let discovery = self
            .explorer
            .explore(seed, config.discovery.clone(), already_explored)
            .await?;

        let tx_hashes = select_transaction_hashes(&discovery, config.max_processed_txs);
        let mut processing = FundFlowProcessingStats {
            selected_transactions: tx_hashes.len(),
            ..FundFlowProcessingStats::default()
        };
        let mut complete_flows = Vec::new();

        for tx_hash in tx_hashes {
            let processed_tx = match self.tx_provider.process_transaction_by_hash(tx_hash).await {
                Ok(tx) => tx,
                Err(error) => {
                    record_failure(&mut processing, tx_hash, "process_transaction", error);
                    continue;
                }
            };

            let mut flows = match extract_fund_flows_from_processed_tx(&processed_tx) {
                Ok(flows) => flows,
                Err(error) => {
                    record_failure(&mut processing, tx_hash, "extract_fund_flows", error);
                    continue;
                }
            };

            if !config.include_tokens {
                flows.token_movements.clear();
            }

            processing.processed_transactions += 1;
            complete_flows.push(flows);
        }

        processing.extracted_complete_flows = complete_flows.len();

        let analyzer = FundFlowAnalyzer::new()
            .with_min_value(config.discovery.min_value_wei)
            .with_gas_inclusion(config.include_gas)
            .with_weth_as_eth(config.include_tokens);
        let fund_flows = analyzer.analyze_fund_flows(&complete_flows);
        processing.aggregated_flows = fund_flows.len();

        let mut network_builder = NetworkBuilder::new()
            .with_min_eth_amount(config.min_edge_eth)
            .with_gas_inclusion(config.include_gas);
        add_discovery_metadata(&mut network_builder, &discovery);

        let network =
            network_builder.build_centered_network(seed, &fund_flows, config.discovery.max_depth);

        processing.network_nodes = network.nodes.len();
        processing.network_edges = network.edges.len();

        Ok(DiscoveredFundFlowNetwork {
            discovery,
            complete_flows,
            fund_flows,
            network,
            processing,
        })
    }
}

pub fn select_transaction_hashes(discovery: &DiscoveryOutput, max_count: usize) -> Vec<TxHash> {
    if max_count == 0 {
        return Vec::new();
    }

    let mut seen = HashSet::new();
    let mut tx_hashes = Vec::new();

    for tx in &discovery.priority_transactions {
        if seen.insert(tx.tx_hash) {
            tx_hashes.push(tx.tx_hash);
            if tx_hashes.len() == max_count {
                return tx_hashes;
            }
        }
    }

    for edge in &discovery.graph.edges {
        if seen.insert(edge.tx_hash) {
            tx_hashes.push(edge.tx_hash);
            if tx_hashes.len() == max_count {
                break;
            }
        }
    }

    tx_hashes
}

fn add_discovery_metadata(builder: &mut NetworkBuilder, discovery: &DiscoveryOutput) {
    for node in discovery.graph.nodes.values() {
        let label = node.name.clone().or_else(|| node.entity_type.clone());
        builder.add_node_metadata(
            node.address,
            label,
            node.is_contract,
            node.entity_type.clone(),
        );
    }
}

fn record_failure(
    processing: &mut FundFlowProcessingStats,
    tx_hash: TxHash,
    stage: &'static str,
    error: eyre::Report,
) {
    processing.failed_transactions += 1;
    if processing.failure_samples.len() < 10 {
        processing
            .failure_samples
            .push(TransactionProcessingFailure {
                tx_hash,
                stage: stage.to_string(),
                error: error.to_string(),
            });
    }
}

fn wei_to_eth_f64(wei: U256) -> f64 {
    let wei_per_eth = U256::from(10).pow(U256::from(18));
    let eth_part = wei / wei_per_eth;
    let wei_remainder = wei % wei_per_eth;

    eth_part.to::<u64>() as f64 + (wei_remainder.to::<u64>() as f64 / 1e18)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_discovery::{
        DiscoveryStats, GraphEdge, GraphNode, StopReason, TxCandidate, UndirectedGraph,
    };
    use alloy_primitives::B256;
    use std::collections::HashMap;

    #[test]
    fn select_transaction_hashes_prioritizes_deep_analysis_candidates() {
        let tx1 = B256::from([1u8; 32]);
        let tx2 = B256::from([2u8; 32]);
        let tx3 = B256::from([3u8; 32]);
        let addr1 = Address::from([1u8; 20]);
        let addr2 = Address::from([2u8; 20]);

        let discovery = DiscoveryOutput {
            graph: UndirectedGraph {
                nodes: HashMap::from([(
                    addr1,
                    GraphNode {
                        address: addr1,
                        entity_type: None,
                        is_contract: false,
                        name: None,
                        explored: true,
                        exploration_depth: 0,
                        first_seen_block: 1,
                    },
                )]),
                edges: vec![
                    GraphEdge {
                        node1: addr1,
                        node2: addr2,
                        tx_hash: tx2,
                        value: U256::ZERO,
                        block_number: 1,
                    },
                    GraphEdge {
                        node1: addr1,
                        node2: addr2,
                        tx_hash: tx3,
                        value: U256::ZERO,
                        block_number: 2,
                    },
                ],
            },
            priority_transactions: vec![TxCandidate {
                tx_hash: tx1,
                from_address: addr1,
                to_address: addr2,
                value: U256::ZERO,
                block_number: 1,
                priority_score: 42.0,
                involves_unknown: true,
                involves_router: false,
            }],
            stop_reason: StopReason::NoMoreCandidates,
            discovery_stats: DiscoveryStats::default(),
        };

        assert_eq!(
            select_transaction_hashes(&discovery, 3),
            vec![tx1, tx2, tx3]
        );
        assert_eq!(select_transaction_hashes(&discovery, 2), vec![tx1, tx2]);
    }
}
