//! Block-window planning for flow-context lookups.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::network::graph::RawTokenNetworkGraph;

use super::{config::FlowContextConfig, model::BlockRange, seeds::FlowContextSeed};

/// Address-specific block window to query in the participation index.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FlowContextWindow {
    pub address: String,
    pub range: BlockRange,
    pub reason: String,
}

/// Build deduplicated query windows around each seed's token-network activity.
pub fn build_flow_context_windows(
    graph: &RawTokenNetworkGraph,
    seeds: &[FlowContextSeed],
    config: &FlowContextConfig,
) -> Vec<FlowContextWindow> {
    let graph_range = graph_observed_range(graph);
    let mut seen = BTreeSet::new();
    let mut windows = Vec::new();

    for seed in seeds {
        for (block, reason) in [
            (seed.first_seen_block, "seed_first_seen"),
            (seed.last_seen_block, "seed_last_seen"),
        ] {
            let Some(block) = block else {
                continue;
            };
            let range = BlockRange::around(block, config.lookback_blocks, config.lookahead_blocks);
            let key = (seed.address.clone(), range, reason.to_string());
            if seen.insert(key) {
                windows.push(FlowContextWindow {
                    address: seed.address.clone(),
                    range,
                    reason: reason.to_string(),
                });
            }
        }

        if seed.first_seen_block.is_none() && seed.last_seen_block.is_none() {
            if let Some(range) = graph_range {
                let key = (seed.address.clone(), range, "token_lifetime".to_string());
                if seen.insert(key) {
                    windows.push(FlowContextWindow {
                        address: seed.address.clone(),
                        range,
                        reason: "token_lifetime".to_string(),
                    });
                }
            }
        }
    }

    windows
}

fn graph_observed_range(graph: &RawTokenNetworkGraph) -> Option<BlockRange> {
    let first = graph
        .nodes
        .values()
        .filter_map(|node| node.observed.first_seen.as_ref())
        .map(|observation| observation.block_number)
        .min()?;
    let last = graph
        .nodes
        .values()
        .filter_map(|node| node.observed.last_seen.as_ref())
        .map(|observation| observation.block_number)
        .max()
        .unwrap_or(first);
    Some(BlockRange::new(first, last))
}
