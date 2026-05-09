use std::sync::Arc;
use std::time::Duration;

use reth_chain_query::provider::RethQueryProvider;
use tokio::task::JoinHandle;
use tracing::{error, warn};

use tx_simulator::TxSimulator;

use super::pending_sequences::PendingSequences;

const PRUNE_INTERVAL: Duration = Duration::from_millis(500);

pub fn spawn_block_pruner(
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
            let latest_block = match tx_simulator.live_latest_block_number().await {
                Ok(Some(number)) => number,
                Ok(None) => match tx_simulator.get_latest_block() {
                    Ok(number) => number,
                    Err(err) => {
                        warn!("Unable to determine latest block from simulator: {}", err);
                        tokio::time::sleep(PRUNE_INTERVAL).await;
                        continue;
                    }
                },
                Err(err) => {
                    warn!("Live cache block lookup failed: {}", err);
                    tokio::time::sleep(PRUNE_INTERVAL).await;
                    continue;
                }
            };

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
                    if let Err(err) = pending_sequences.prune_block(provider.clone(), block).await {
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

            tokio::time::sleep(PRUNE_INTERVAL).await;
        }
    })
}
