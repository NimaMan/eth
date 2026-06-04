use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eth_alpha_core::{
    ids::TokenPoolId,
    market::{MarketEvent, PoolSnapshot},
    risk::RiskPolicy,
    store::TradingStore,
};
use eth_alpha_store::PostgresTradingStore;
use eyre::Result;
use serde_json::json;
use tracing::info;

use eth_alpha_engine::wire::{LiveStatusResponse, PoolWire};
use eth_alpha_engine::{AlphaEngine, EngineEvent, EngineExecutionAdapter};

use super::super::mined_pool_risks::mined_pool_risks_from_update;
use super::super::support::{
    record_mined_pool_risk_observation, record_pool_observation, reports_payload,
};

pub(in crate::live_trader) struct PoolUpdateProcessingInput<'a> {
    pub(in crate::live_trader) store: &'a PostgresTradingStore,
    pub(in crate::live_trader) observation_strategy_name: &'a str,
    pub(in crate::live_trader) status: &'a LiveStatusResponse,
    pub(in crate::live_trader) first_poll: bool,
    pub(in crate::live_trader) suppress_events: bool,
    pub(in crate::live_trader) replay_current: bool,
    pub(in crate::live_trader) adapter_current_block: &'a Arc<AtomicU64>,
}

#[derive(Default)]
pub(in crate::live_trader) struct PoolUpdateProcessingSummary {
    pub(in crate::live_trader) market_events: usize,
    pub(in crate::live_trader) risk_events: usize,
    pub(in crate::live_trader) reports: usize,
}

pub(in crate::live_trader) async fn process_pool_updates<E, R, S>(
    input: PoolUpdateProcessingInput<'_>,
    engine: &mut AlphaEngine<E, R, S>,
    polled_pools: Vec<(PoolWire, PoolSnapshot)>,
    seen_pool_blocks: &mut HashMap<TokenPoolId, u64>,
    seen_mined_pool_risk_keys: &mut HashSet<String>,
) -> Result<PoolUpdateProcessingSummary>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    let mut summary = PoolUpdateProcessingSummary::default();
    for (pool_wire, pool) in polled_pools {
        let previous_block = seen_pool_blocks.get(&pool.address).copied();
        let changed = previous_block
            .map(|previous| pool.latest_block > previous)
            .unwrap_or(true);
        if !changed {
            continue;
        }
        seen_pool_blocks.insert(pool.address.clone(), pool.latest_block);
        let mined_risk_candidates = mined_pool_risks_from_update(&pool_wire, &pool);
        let mined_risk_candidate_count = mined_risk_candidates.len();

        if input.suppress_events || (input.first_poll && !input.replay_current) {
            let mut primed_mined_risks = 0usize;
            for candidate in mined_risk_candidates {
                if seen_mined_pool_risk_keys.insert(candidate.key.clone()) {
                    record_mined_pool_risk_observation(
                        input.store,
                        input.observation_strategy_name,
                        &candidate.key,
                        &candidate.event,
                        "primed",
                        0,
                        input.first_poll,
                        input.suppress_events,
                        input.status,
                        json!({ "phase": "primed_from_pool_update" }),
                    )
                    .await?;
                    primed_mined_risks += 1;
                }
            }
            record_pool_observation(
                input.store,
                input.observation_strategy_name,
                &pool_wire,
                &pool,
                previous_block,
                "primed",
                0,
                input.first_poll,
                input.suppress_events,
                input.status,
                json!({
                    "mined_pool_risk_candidates": mined_risk_candidate_count,
                    "mined_pool_risks_primed": primed_mined_risks,
                }),
            )
            .await?;
            continue;
        }

        let event = MarketEvent::PoolUpdated {
            block_number: pool.latest_block,
            pool: pool.clone(),
        };
        input
            .adapter_current_block
            .store(pool.latest_block, Ordering::Relaxed);
        let event_reports = engine.handle_event(EngineEvent::Market(event)).await?;
        let report_count = event_reports.len();
        let decision = if report_count > 0 {
            "submitted"
        } else {
            "hold"
        };
        record_pool_observation(
            input.store,
            input.observation_strategy_name,
            &pool_wire,
            &pool,
            previous_block,
            decision,
            report_count,
            input.first_poll,
            input.suppress_events,
            input.status,
            json!({
                "reports": reports_payload(&event_reports),
                "mined_pool_risk_candidates": mined_risk_candidate_count,
            }),
        )
        .await?;
        summary.reports += report_count;
        summary.market_events += 1;
        for report in event_reports {
            info!(
                order_id = %report.order_id.0,
                status = ?report.status,
                block_number = ?report.block_number,
                gas_used = ?report.gas_used,
                error = ?report.error,
                "chain-sim execution report"
            );
        }

        for candidate in mined_risk_candidates {
            if !seen_mined_pool_risk_keys.insert(candidate.key.clone()) {
                continue;
            }
            if let Some(block_number) = candidate.event.observed_block {
                input
                    .adapter_current_block
                    .store(block_number, Ordering::Relaxed);
            }
            let event_reports = engine
                .handle_event(EngineEvent::Risk(candidate.event.clone()))
                .await?;
            let report_count = event_reports.len();
            let decision = if report_count > 0 {
                "submitted"
            } else {
                "hold"
            };
            record_mined_pool_risk_observation(
                input.store,
                input.observation_strategy_name,
                &candidate.key,
                &candidate.event,
                decision,
                report_count,
                input.first_poll,
                input.suppress_events,
                input.status,
                json!({ "reports": reports_payload(&event_reports) }),
            )
            .await?;
            summary.reports += report_count;
            summary.risk_events += 1;
            for report in event_reports {
                info!(
                    order_id = %report.order_id.0,
                    status = ?report.status,
                    block_number = ?report.block_number,
                    gas_used = ?report.gas_used,
                    error = ?report.error,
                    risk_key = %candidate.key,
                    "chain-sim mined pool risk execution report"
                );
            }
        }
    }
    Ok(summary)
}
