use std::sync::Arc;
use std::time::Duration;

use reth_chain_query::provider::RethQueryProvider;
use tokio::task::JoinHandle;
use tracing::{error, warn};

use crate::canonical_head_cache::CanonicalHeadCache;
use tx_simulator::TxSimulator;

use super::pending_sequences::PendingSequences;

const PRUNE_INTERVAL: Duration = Duration::from_millis(500);

pub fn spawn_block_pruner(
    head_cache: Arc<CanonicalHeadCache>,
    tx_simulator: Arc<TxSimulator>,
    pending_sequences: PendingSequences,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let provider = match RethQueryProvider::with_simulator(tx_simulator.clone()) {
            Ok(provider) => Arc::new(provider),
            Err(err) => {
                error!(
                    "Failed to create RethQueryProvider for mined-tx pruning: {}",
                    err
                );
                return;
            }
        };

        let mut last_pruned: Option<u64> = None;
        loop {
            if let Some(snapshot) = head_cache.latest_snapshot().await {
                let latest_block = snapshot.number;
                let should_prune = match last_pruned {
                    Some(prev) => prev != latest_block,
                    None => true,
                };

                if should_prune {
                    let start_block = match last_pruned {
                        Some(prev) if latest_block > prev => prev + 1,
                        _ => latest_block,
                    };

                    let mut block = start_block;
                    while block <= latest_block {
                        if let Err(err) =
                            pending_sequences.prune_block(provider.clone(), block).await
                        {
                            warn!(
                                "Failed to prune mined transactions for block {}: {}",
                                block, err
                            );
                            break;
                        }
                        block += 1;
                    }

                    last_pruned = Some(latest_block);
                }
            }

            tokio::time::sleep(PRUNE_INTERVAL).await;
        }
    })
}
