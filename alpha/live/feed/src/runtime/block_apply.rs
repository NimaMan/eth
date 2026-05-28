use std::collections::BTreeMap;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use eth_ops_events::{emit_bottleneck, PipelineBottleneckSample};
use eth_token::chain_metadata::{LiveRethChainMetadataProvider, TokenDiscoveryProvider};
use eyre::{bail, Result};
use serde_json::json;
use tx_processor::{
    load_processed_block, BlockProcessor, BlockStateSession, LivePoolBuySellSimulator,
    LiveStateDiffFrame, LoadedProcessedBlock as LiveBlockLoad,
};

use super::apply_report::{apply_report, push_bottleneck};
use super::block_update::LiveBlockUpdate;
use super::helpers::{should_log_block_apply, status_label};
use super::progress::LiveTokenStatus;
use super::service::{
    LiveTokenRuntime, LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET, LIVE_TOKEN_TRACKER_LOG_TARGET,
};
use super::time::now_unix_secs;

impl LiveTokenRuntime {
    pub async fn apply_live_block_update(&self, update: LiveBlockUpdate) -> Result<()> {
        let LiveBlockUpdate {
            loaded,
            state_diffs,
        } = update;
        let block_number = loaded.block.header.number;
        {
            let state = self.inner.state.read().await;
            if state.progress.status != LiveTokenStatus::Live {
                bail!(
                    "live token runtime is {}; cannot apply live block update {}",
                    status_label(&state.progress.status),
                    block_number
                );
            }
        }

        let discovery_provider = LiveRethChainMetadataProvider::new(self.inner.provider.as_ref());
        let pool_simulator =
            LivePoolBuySellSimulator::from_simulator(self.inner.provider.simulator().clone());
        self.apply_loaded_block(
            block_number,
            true,
            loaded,
            &discovery_provider,
            &pool_simulator,
            state_diffs.as_deref(),
        )
        .await
    }

    pub(super) async fn apply_block<P>(
        &self,
        block_number: u64,
        is_live_tail: bool,
        tx_processor: &BlockProcessor,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
    ) -> Result<()>
    where
        P: TokenDiscoveryProvider,
    {
        let loaded = load_processed_block(
            tx_processor,
            self.inner.provider.as_ref(),
            self.inner.processed_block_replay_store.clone(),
            block_number,
        )
        .await?;

        self.apply_loaded_block(
            block_number,
            is_live_tail,
            loaded,
            discovery_provider,
            pool_simulator,
            None,
        )
        .await
    }

    async fn apply_loaded_block<P>(
        &self,
        block_number: u64,
        is_live_tail: bool,
        loaded: LiveBlockLoad,
        discovery_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
        live_state_diffs: Option<&[LiveStateDiffFrame]>,
    ) -> Result<()>
    where
        P: TokenDiscoveryProvider,
    {
        let block_apply_started = Instant::now();
        let block_transaction_count = loaded.block.transactions.len();
        let block_source = loaded.source;
        let upstream_ms = loaded.upstream_ms;
        let disk_cache_hit = loaded.disk_cache_hit;
        let disk_cache_read_ms = loaded.disk_cache_read_ms;
        let disk_cache_write_ms = loaded.disk_cache_write_ms;
        let live_block_sessions: StdMutex<BTreeMap<u64, BlockStateSession>> =
            StdMutex::new(BTreeMap::new());
        if is_live_tail {
            match live_state_diffs {
                Some(state_diffs) => {
                    match self
                        .build_direct_live_block_session(&loaded, pool_simulator, state_diffs)
                        .await
                    {
                        Ok(session) => {
                            live_block_sessions
                                .lock()
                                .map_err(|err| {
                                    eyre::eyre!("direct live block session lock poisoned: {err}")
                                })?
                                .insert(block_number, session.clone());
                            self.remember_direct_live_block_session(
                                block_number,
                                loaded.block.header.hash,
                                session,
                            )
                            .await;
                        }
                        Err(error) => {
                            tracing::warn!(
                                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                                block_number,
                                error = %error,
                                "failed to build direct live block state session"
                            );
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                        block_number,
                        "live block update did not include state diffs for direct state session"
                    );
                }
            }
        }

        let state_lock_started = Instant::now();
        let mut state = self.inner.state.write().await;
        let state_lock_wait_us = state_lock_started.elapsed().as_micros();
        state.progress.current_block = Some(block_number);
        state.progress.updated_at_unix_secs = now_unix_secs();

        let apply_started = Instant::now();
        let apply_timeout = Duration::from_millis(self.inner.config.block_apply_timeout_ms);
        let tracked_tokens_before = state.processor.registry().tokens.len();
        let tracked_pools_before: usize = state
            .processor
            .registry()
            .tokens
            .values()
            .map(|token| token.pool_count())
            .sum();
        let process_block_started = Instant::now();
        let report = if is_live_tail {
            let live_discovery_provider =
                LiveRethChainMetadataProvider::with_direct_live_block_sessions(
                    self.inner.provider.as_ref(),
                    &live_block_sessions,
                );
            state
                .processor
                .process_block_live_with_discovery_provider_and_sessions(
                    &loaded.block,
                    &live_discovery_provider,
                    pool_simulator,
                    &live_block_sessions,
                    true,
                )
                .await
        } else {
            state
                .processor
                .process_block_live_with_discovery_provider(
                    &loaded.block,
                    discovery_provider,
                    pool_simulator,
                )
                .await
        };
        let process_block_us = process_block_started.elapsed().as_micros();
        if process_block_started.elapsed() > apply_timeout {
            let mut sample = PipelineBottleneckSample::new(
                "eth_chain_server",
                "live_tracker",
                "block_apply",
                process_block_us / 1_000,
                "slow live token block apply",
            );
            sample.run_id = state.progress.id.clone();
            sample.block_number = Some(block_number);
            sample.threshold_ms = Some(u128::from(self.inner.config.block_apply_timeout_ms));
            sample
                .work_units
                .insert("txs".to_string(), json!(block_transaction_count));
            sample.work_units.insert(
                "tracked_tokens_before".to_string(),
                json!(tracked_tokens_before),
            );
            sample.work_units.insert(
                "tracked_pools_before".to_string(),
                json!(tracked_pools_before),
            );
            sample
                .breakdown_ms
                .insert("process_block".to_string(), json!(process_block_us / 1_000));
            sample
                .breakdown_ms
                .insert("upstream".to_string(), json!(upstream_ms));
            sample
                .breakdown_ms
                .insert("disk_cache_read".to_string(), json!(disk_cache_read_ms));
            sample
                .breakdown_ms
                .insert("disk_cache_write".to_string(), json!(disk_cache_write_ms));
            emit_bottleneck(&sample);
            push_bottleneck(&mut state, sample);
            tracing::warn!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                block_number,
                is_live_tail,
                source = block_source,
                txs = block_transaction_count,
                tracked_tokens_before,
                tracked_pools_before,
                slow_threshold_ms = self.inner.config.block_apply_timeout_ms,
                process_block_ms = process_block_us / 1_000,
                upstream_ms,
                disk_cache_hit,
                disk_cache_read_ms,
                disk_cache_write_ms,
                "slow live token block apply"
            );
        }
        let retention_started = Instant::now();
        let retention_report = state.processor.apply_index_retention_policy(block_number);
        let retention_us = retention_started.elapsed().as_micros();
        let token_apply_us = apply_started.elapsed().as_micros();
        let token_apply_ms = token_apply_us / 1_000;

        let state_update_started = Instant::now();
        let event = apply_report(
            &mut state,
            report,
            retention_report,
            loaded,
            token_apply_ms,
            is_live_tail,
        );
        let should_log_progress = should_log_block_apply(&state.progress, is_live_tail);
        let progress = should_log_progress.then(|| state.progress.clone());
        let state_update_us = state_update_started.elapsed().as_micros();
        let block_apply_wall_us = block_apply_started.elapsed().as_micros();
        let measured_us = state_lock_wait_us
            .saturating_add(token_apply_us)
            .saturating_add(state_update_us);
        let unaccounted_us = block_apply_wall_us.saturating_sub(measured_us);
        tracing::info!(
            target: LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET,
            block_number,
            is_live_tail,
            source = block_source,
            txs = block_transaction_count,
            tracked_tokens_before,
            tracked_pools_before,
            state_lock_wait_us,
            process_block_us,
            retention_us,
            token_apply_us,
            state_update_us,
            block_apply_wall_us,
            unaccounted_us,
            upstream_us = upstream_ms.saturating_mul(1_000),
            disk_cache_read_us = disk_cache_read_ms.saturating_mul(1_000),
            disk_cache_write_us = disk_cache_write_ms.saturating_mul(1_000),
            state_lock_wait_ms = state_lock_wait_us / 1_000,
            process_block_ms = process_block_us / 1_000,
            retention_ms = retention_us / 1_000,
            token_apply_ms,
            state_update_ms = state_update_us / 1_000,
            block_apply_wall_ms = block_apply_wall_us / 1_000,
            unaccounted_ms = unaccounted_us / 1_000,
            upstream_ms,
            disk_cache_hit,
            disk_cache_read_ms,
            disk_cache_write_ms,
            "live token apply profile"
        );
        if let Some(progress) = progress {
            tracing::info!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                status = ?progress.status,
                current_block = ?progress.current_block,
                blocks_processed = progress.blocks_processed,
                warmup_total_blocks = progress.warmup_total_blocks,
                live_blocks_processed = progress.live_blocks_processed,
                txs_processed = progress.txs_processed,
                transaction_failures = progress.transaction_failures,
                pool_simulation_failures = progress.pool_simulation_failures,
                tracked_tokens = progress.tracked_tokens,
                tracked_pools = progress.tracked_pools,
                tracked_v2_pools = progress.tracked_v2_pools,
                tracked_v3_pools = progress.tracked_v3_pools,
                tracked_v4_pools = progress.tracked_v4_pools,
                block_source = ?progress.last_block_source,
                disk_cache_hits = progress.processed_block_disk_cache_hits,
                disk_cache_misses = progress.processed_block_disk_cache_misses,
                token_apply_ms = ?progress.last_block_token_apply_ms,
                "live token runtime applied block"
            );
        }
        drop(state);
        let _ = self.inner.event_tx.send(event);
        Ok(())
    }
}
