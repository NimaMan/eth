use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eth_alpha_core::{ids::TokenPoolId, market::PoolSnapshot};
use eyre::{eyre, Result};
use tracing::warn;

use crate::wire::{LiveBlockFrameWire, LiveStatusResponse, MempoolSignalsResponse, PoolWire};

use super::super::support::TraderExecutionMode;
use super::super::token_server::TokenServerClient;
use super::super::LIVE_UPDATE_WAIT_TIMEOUT_MS;

const LIVE_REAL_FRAME_POLL_TIMEOUT_MS: u64 = 1;

pub(in crate::live_trader) struct LiveInputBatch {
    pub(in crate::live_trader) status: LiveStatusResponse,
    pub(in crate::live_trader) signals: MempoolSignalsResponse,
    pub(in crate::live_trader) frame_event: String,
    pub(in crate::live_trader) frame_block: Option<u64>,
    pub(in crate::live_trader) frame_hash: Option<String>,
    pub(in crate::live_trader) frame_pool_count: usize,
    pub(in crate::live_trader) polled_pools: Vec<(PoolWire, PoolSnapshot)>,
    pub(in crate::live_trader) polled_pool_wires: HashMap<TokenPoolId, PoolWire>,
}

pub(in crate::live_trader) async fn read_live_inputs(
    client: &TokenServerClient,
    execution_mode: TraderExecutionMode,
    after_block: u64,
    signal_limit: i64,
    mempool_since_days: i64,
    pool_updates: &Arc<Mutex<HashMap<TokenPoolId, PoolSnapshot>>>,
    pool_wire_cache: &mut HashMap<TokenPoolId, PoolWire>,
) -> Result<LiveInputBatch> {
    let frame_response = client
        .next_block_frame(after_block, frame_wait_timeout_ms(execution_mode))
        .await?;
    let frame_event = frame_response.event.clone();
    let frame = match frame_response.frame {
        Some(frame) => Some(frame),
        None if matches!(execution_mode, TraderExecutionMode::ChainSim) => {
            return Err(eyre!(
                "chain-server live block frame unavailable after block {}; event={} status={}",
                after_block,
                frame_response.event,
                frame_response.status
            ));
        }
        None => None,
    };

    let status = match frame.as_ref() {
        Some(frame) => frame.status.clone(),
        None => client.status().await?,
    };
    let mut polled_pools = Vec::with_capacity(frame.as_ref().map_or(0, |frame| frame.pools.len()));
    if let Some(frame) = frame.as_ref() {
        collect_frame_pools(frame, &mut polled_pools, pool_wire_cache);
    }

    {
        let mut pool_updates = pool_updates.lock().expect("pool lock");
        for (_, pool) in &polled_pools {
            pool_updates.insert(pool.address.clone(), pool.clone());
        }
    }

    let signals = client
        .mempool_signals(signal_limit, mempool_since_days)
        .await?;
    Ok(LiveInputBatch {
        status,
        signals,
        frame_event,
        frame_block: frame.as_ref().map(|frame| frame.block_number),
        frame_hash: frame.as_ref().map(|frame| frame.block_hash.clone()),
        frame_pool_count: polled_pools.len(),
        polled_pools,
        polled_pool_wires: pool_wire_cache.clone(),
    })
}

fn collect_frame_pools(
    frame: &LiveBlockFrameWire,
    polled_pools: &mut Vec<(PoolWire, PoolSnapshot)>,
    pool_wire_cache: &mut HashMap<TokenPoolId, PoolWire>,
) {
    for pool_wire in frame.pools.iter().cloned() {
        match pool_wire.to_pool_snapshot() {
            Ok(pool) => {
                pool_wire_cache.insert(pool.address.clone(), pool_wire.clone());
                polled_pools.push((pool_wire, pool));
            }
            Err(error) => {
                warn!(
                    error = %error,
                    frame_block = frame.block_number,
                    frame_hash = %frame.block_hash,
                    "skipping block-frame pool snapshot"
                );
            }
        }
    }
}

fn frame_wait_timeout_ms(execution_mode: TraderExecutionMode) -> u64 {
    match execution_mode {
        TraderExecutionMode::ChainSim => LIVE_UPDATE_WAIT_TIMEOUT_MS,
        TraderExecutionMode::KartalReal => LIVE_REAL_FRAME_POLL_TIMEOUT_MS,
    }
}
