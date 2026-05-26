use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eth_alpha_core::{ids::TokenPoolId, market::PoolSnapshot};
use eyre::Result;
use tracing::warn;

use crate::wire::{LiveStatusResponse, MempoolSignalsResponse, PoolWire};

use super::super::token_server::TokenServerClient;

pub(in crate::live_trader) struct LivePollBatch {
    pub(in crate::live_trader) status: LiveStatusResponse,
    pub(in crate::live_trader) signals: MempoolSignalsResponse,
    pub(in crate::live_trader) pool_response_count: usize,
    pub(in crate::live_trader) polled_pools: Vec<(PoolWire, PoolSnapshot)>,
    pub(in crate::live_trader) polled_pool_wires: HashMap<TokenPoolId, PoolWire>,
}

pub(in crate::live_trader) async fn poll_live_inputs(
    client: &TokenServerClient,
    signal_limit: i64,
    mempool_since_days: i64,
    pool_updates: &Arc<Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
) -> Result<LivePollBatch> {
    let status = client.status().await?;
    let pools = client.pools().await?;
    let signals = client
        .mempool_signals(signal_limit, mempool_since_days)
        .await?;
    let pool_response_count = pools.count;
    let mut polled_pools = Vec::with_capacity(pools.pools.len());
    let mut polled_pool_wires = HashMap::with_capacity(pools.pools.len());
    for pool_wire in pools.pools {
        match pool_wire.to_pool_snapshot() {
            Ok(pool) => {
                polled_pool_wires.insert(pool.address.clone(), pool_wire.clone());
                polled_pools.push((pool_wire, pool));
            }
            Err(error) => {
                warn!(error = %error, "skipping pool snapshot");
            }
        }
    }
    {
        let mut pool_updates = pool_updates.lock().expect("pool lock");
        for (_, pool) in &polled_pools {
            pool_updates.insert(pool.address.clone(), pool.clone());
        }
    }

    Ok(LivePollBatch {
        status,
        signals,
        pool_response_count,
        polled_pools,
        polled_pool_wires,
    })
}
