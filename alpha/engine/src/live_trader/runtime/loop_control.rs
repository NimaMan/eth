use std::time::{Duration, Instant};

use eyre::{eyre, Result};
use tokio::time;
use tracing::{info, warn};

use crate::wire::{GasRankSamplesResponse, LiveStatusResponse};

use super::support::TraderExecutionMode;
use super::token_server::TokenServerClient;
use super::{
    CHAIN_SERVER_PREFLIGHT_POLL_INTERVAL_MS, CHAIN_SERVER_PREFLIGHT_TIMEOUT_SECS,
    CHAIN_SIM_SKIP_MEMPOOL_TRADING_ENABLED_REASON_CODE, LIVE_POLL_ERROR_RETRY_MS,
    LIVE_REAL_MEMPOOL_SIGNAL_POLL_INTERVAL_MS,
};

pub(super) async fn wait_for_next_loop_event(
    _client: &TokenServerClient,
    execution_mode: TraderExecutionMode,
    _current_block: Option<u64>,
) -> Result<()> {
    match execution_mode {
        TraderExecutionMode::ChainSim => {}
        TraderExecutionMode::KartalReal => {
            time::sleep(Duration::from_millis(
                LIVE_REAL_MEMPOOL_SIGNAL_POLL_INTERVAL_MS,
            ))
            .await;
        }
    }
    Ok(())
}

pub(super) fn poll_error_retry_delay(execution_mode: TraderExecutionMode) -> Duration {
    match execution_mode {
        TraderExecutionMode::ChainSim => Duration::from_millis(LIVE_POLL_ERROR_RETRY_MS),
        TraderExecutionMode::KartalReal => {
            Duration::from_millis(LIVE_REAL_MEMPOOL_SIGNAL_POLL_INTERVAL_MS)
        }
    }
}

pub(super) async fn preflight_chain_server_readiness(
    runner_name: &str,
    client: &TokenServerClient,
    required_gas_rank_samples: usize,
) -> Result<()> {
    let timeout = Duration::from_secs(CHAIN_SERVER_PREFLIGHT_TIMEOUT_SECS);
    let interval = Duration::from_millis(CHAIN_SERVER_PREFLIGHT_POLL_INTERVAL_MS);
    let started = Instant::now();
    let mut last_error = String::from("preflight has not run yet");

    while started.elapsed() < timeout {
        match chain_server_readiness_once(client, required_gas_rank_samples).await {
            Ok((status, samples)) => {
                info!(
                    live_status = %status.progress.status,
                    live_current_block = ?status.progress.current_block,
                    gas_rank_available_recent_blocks = samples.available_recent_blocks,
                    gas_rank_latest_block = ?samples.latest_block,
                    gas_rank_source = %samples.source,
                    "{} chain-server readiness preflight passed",
                    runner_name
                );
                return Ok(());
            }
            Err(error) => {
                last_error = error.to_string();
                warn!(
                    error = %last_error,
                    elapsed_secs = started.elapsed().as_secs(),
                    timeout_secs = CHAIN_SERVER_PREFLIGHT_TIMEOUT_SECS,
                    "{} waiting for chain-server readiness preflight",
                    runner_name
                );
                time::sleep(interval).await;
            }
        }
    }

    Err(eyre!(
        "{} requires chain-server live status and a gas-rank sample window of {} blocks before startup; last error after {}s: {}",
        runner_name,
        required_gas_rank_samples,
        CHAIN_SERVER_PREFLIGHT_TIMEOUT_SECS,
        last_error
    ))
}

async fn chain_server_readiness_once(
    client: &TokenServerClient,
    required_gas_rank_samples: usize,
) -> Result<(LiveStatusResponse, GasRankSamplesResponse)> {
    let status = client.versioned_status().await?;
    if status.progress.status != "live" {
        return Err(eyre!(
            "chain-server live status is {}; current_block={:?}; last_error={:?}",
            status.progress.status,
            status.progress.current_block,
            status.progress.last_error
        ));
    }
    let samples = client.gas_rank_samples(required_gas_rank_samples).await?;
    if samples.available_recent_blocks < required_gas_rank_samples || samples.latest_block.is_none()
    {
        return Err(eyre!(
            "chain-server gas-rank sample window is not ready; requested_blocks={}, required_blocks={}, available_recent_blocks={}, latest_block={:?}, source={}",
            samples.requested_blocks,
            required_gas_rank_samples,
            samples.available_recent_blocks,
            samples.latest_block,
            samples.source
        ));
    }
    Ok((status, samples))
}

pub(super) fn mempool_signal_skip_reason_code_for_execution_mode(
    execution_mode: TraderExecutionMode,
    signal_type: &str,
) -> Option<&'static str> {
    (matches!(execution_mode, TraderExecutionMode::ChainSim) && signal_type == "trading_enabled")
        .then_some(CHAIN_SIM_SKIP_MEMPOOL_TRADING_ENABLED_REASON_CODE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_sim_live_backtest_skips_trading_enabled_mempool_entries() {
        assert_eq!(
            mempool_signal_skip_reason_code_for_execution_mode(
                TraderExecutionMode::ChainSim,
                "trading_enabled"
            ),
            Some(CHAIN_SIM_SKIP_MEMPOOL_TRADING_ENABLED_REASON_CODE)
        );
        assert!(mempool_signal_skip_reason_code_for_execution_mode(
            TraderExecutionMode::ChainSim,
            "liquidity_removal"
        )
        .is_none());
    }

    #[test]
    fn kartal_real_keeps_trading_enabled_mempool_entries() {
        assert!(mempool_signal_skip_reason_code_for_execution_mode(
            TraderExecutionMode::KartalReal,
            "trading_enabled"
        )
        .is_none());
    }
}
