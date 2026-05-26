use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eth_alpha_core::{market::MarketEvent, risk::RiskPolicy, store::TradingStore};
use eth_alpha_store::PostgresTradingStore;
use eyre::Result;
use serde_json::json;
use tracing::info;

use crate::wire::LiveStatusResponse;
use crate::{AlphaEngine, EngineEvent, EngineExecutionAdapter};

use super::super::support::{record_position_monitor_observation, reports_payload};

pub(in crate::live_trader) struct PositionMonitorInput<'a> {
    pub(in crate::live_trader) store: &'a PostgresTradingStore,
    pub(in crate::live_trader) observation_strategy_name: &'a str,
    pub(in crate::live_trader) status: &'a LiveStatusResponse,
    pub(in crate::live_trader) first_poll: bool,
    pub(in crate::live_trader) suppress_events: bool,
    pub(in crate::live_trader) replay_current: bool,
    pub(in crate::live_trader) market_events: usize,
    pub(in crate::live_trader) adapter_current_block: &'a Arc<AtomicU64>,
}

#[derive(Default)]
pub(in crate::live_trader) struct PositionMonitorSummary {
    pub(in crate::live_trader) position_monitor_events: usize,
    pub(in crate::live_trader) reports: usize,
}

pub(in crate::live_trader) async fn process_position_monitor<E, R, S>(
    input: PositionMonitorInput<'_>,
    engine: &mut AlphaEngine<E, R, S>,
    last_position_monitor_block: &mut Option<u64>,
) -> Result<PositionMonitorSummary>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    let mut summary = PositionMonitorSummary::default();
    if input.suppress_events || (input.first_poll && !input.replay_current) {
        return Ok(summary);
    }
    let Some(block_number) = input.status.progress.current_block else {
        return Ok(summary);
    };
    let should_monitor = last_position_monitor_block
        .map(|previous| block_number > previous)
        .unwrap_or(true);
    if !should_monitor {
        return Ok(summary);
    }

    let event = MarketEvent::BlockCompleted {
        block_number,
        updated_tokens: 0,
        updated_pools: input.market_events,
    };
    input
        .adapter_current_block
        .store(block_number, Ordering::Relaxed);
    let event_reports = engine.handle_event(EngineEvent::Market(event)).await?;
    let report_count = event_reports.len();
    record_position_monitor_observation(
        input.store,
        input.observation_strategy_name,
        block_number,
        "checked",
        report_count,
        input.first_poll,
        input.suppress_events,
        input.status,
        json!({ "reports": reports_payload(&event_reports) }),
    )
    .await?;
    summary.reports += report_count;
    summary.position_monitor_events += 1;
    *last_position_monitor_block = Some(block_number);
    for report in event_reports {
        info!(
            order_id = %report.order_id.0,
            status = ?report.status,
            block_number = ?report.block_number,
            gas_used = ?report.gas_used,
            error = ?report.error,
            "chain-sim position monitor execution report"
        );
    }
    Ok(summary)
}
