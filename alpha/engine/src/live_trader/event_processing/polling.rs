use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eth_alpha_core::{ids::TokenPoolId, market::PoolSnapshot};
use eyre::{Result, WrapErr};
use tracing::{debug, warn};

use crate::wire::{LiveStatusResponse, MempoolSignalsResponse, PoolWire};
use crate::LiveChainSimExecutionAdapter;

use super::super::token_server::TokenServerClient;

const LIVE_STATE_POLL_SYNC_TIMEOUT: Duration = Duration::from_secs(5);

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
    state_status_adapter: Option<&LiveChainSimExecutionAdapter>,
) -> Result<LivePollBatch> {
    let status = client.status().await?;
    let require_live_state_sync =
        status.progress.status == "live" && state_status_adapter.is_some();
    sync_live_state_for_block(
        state_status_adapter,
        require_live_state_sync
            .then_some(status.progress.current_block)
            .flatten(),
        "live_status",
    )
    .await?;
    let pools = client.pools().await?;
    let pool_response_count = pools.count;
    let mut polled_pools = Vec::with_capacity(pools.pools.len());
    let mut polled_pool_wires = HashMap::with_capacity(pools.pools.len());
    let mut required_state_block = status.progress.current_block;
    for pool_wire in pools.pools {
        match pool_wire.to_pool_snapshot() {
            Ok(pool) => {
                required_state_block = Some(
                    required_state_block
                        .unwrap_or_default()
                        .max(pool.latest_block),
                );
                polled_pool_wires.insert(pool.address.clone(), pool_wire.clone());
                polled_pools.push((pool_wire, pool));
            }
            Err(error) => {
                warn!(error = %error, "skipping pool snapshot");
            }
        }
    }
    sync_live_state_for_block(
        state_status_adapter,
        require_live_state_sync
            .then_some(required_state_block)
            .flatten(),
        "live_pool_snapshot",
    )
    .await?;
    let signals = client
        .mempool_signals(signal_limit, mempool_since_days)
        .await?;
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

async fn sync_live_state_for_block(
    state_status_adapter: Option<&LiveChainSimExecutionAdapter>,
    block_number: Option<u64>,
    phase: &'static str,
) -> Result<()> {
    let Some(block_number) = block_number.filter(|block| *block > 0) else {
        return Ok(());
    };
    let Some(state_status_adapter) = state_status_adapter else {
        return Ok(());
    };
    let started = Instant::now();
    let status = state_status_adapter
        .wait_for_state_at(block_number, LIVE_STATE_POLL_SYNC_TIMEOUT)
        .await
        .wrap_err_with(|| {
            format!("live simulator state did not reach required {phase} block {block_number}")
        })?;
    debug!(
        phase,
        required_block = block_number,
        selected_block = status.selected_block_number,
        wait_ms = started.elapsed().as_millis(),
        "live simulator state synchronized before live input processing"
    );
    Ok(())
}
