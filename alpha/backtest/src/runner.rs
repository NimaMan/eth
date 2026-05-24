use eth_alpha_core::market::MarketEvent;
use eth_alpha_core::mempool_entry::projected_pool_from_risk_event;
use eth_alpha_engine::{AlphaEngine, EngineEvent};
use eyre::Result;
use tracing::{info, warn};

use crate::adapter::BacktestAdapter;

/// Summary statistics produced by a backtest run.
#[derive(Clone, Debug, Default)]
pub struct BacktestResult {
    pub events_processed: usize,
    pub reports_generated: usize,
    pub confirmed_reports: usize,
    pub failed_reports: usize,
}

/// Drive a sequence of historical events through an `AlphaEngine`.
///
/// The `adapter` handle is used to update the simulated market state
/// (pool snapshots and current block) before each event is handled.
/// Because the adapter is cloned into the engine, both handles share
/// the same underlying state.
pub async fn run_backtest<E, R, S, A>(
    engine: &mut AlphaEngine<E, R, S>,
    adapter: &A,
    events: Vec<EngineEvent>,
) -> Result<BacktestResult>
where
    E: eth_alpha_engine::EngineExecutionAdapter,
    R: eth_alpha_core::risk::RiskPolicy,
    S: eth_alpha_core::store::TradingStore,
    A: BacktestAdapter,
{
    let mut result = BacktestResult::default();

    for event in events {
        update_adapter_state(adapter, &event);

        match engine.handle_event(event).await {
            Ok(reports) => {
                result.events_processed += 1;
                record_reports(&mut result, &reports);
            }
            Err(error) => {
                warn!(error = %error, "engine event handling failed");
            }
        }
    }

    let reports = engine.flush_pending_executions().await?;
    record_reports(&mut result, &reports);

    info!(
        events_processed = result.events_processed,
        reports_generated = result.reports_generated,
        confirmed_reports = result.confirmed_reports,
        failed_reports = result.failed_reports,
        "backtest completed"
    );

    Ok(result)
}

fn record_reports(
    result: &mut BacktestResult,
    reports: &[eth_alpha_core::execution::ExecutionReport],
) {
    result.reports_generated += reports.len();
    for report in reports {
        use eth_alpha_core::execution::ExecutionStatus;
        match report.status {
            ExecutionStatus::Confirmed => result.confirmed_reports += 1,
            ExecutionStatus::Failed => result.failed_reports += 1,
            _ => {}
        }
    }
}

fn update_adapter_state<A: BacktestAdapter>(adapter: &A, event: &EngineEvent) {
    match event {
        EngineEvent::Market(MarketEvent::PoolUpdated { block_number, pool }) => {
            adapter
                .current_block()
                .store(*block_number, std::sync::atomic::Ordering::Relaxed);
            adapter
                .pools()
                .lock()
                .expect("pool lock")
                .insert(pool.address.clone(), pool.clone());
        }
        EngineEvent::Market(MarketEvent::TokenUpdated { block_number, .. }) => {
            adapter
                .current_block()
                .store(*block_number, std::sync::atomic::Ordering::Relaxed);
        }
        EngineEvent::Market(MarketEvent::BlockCompleted { block_number, .. }) => {
            adapter
                .current_block()
                .store(*block_number, std::sync::atomic::Ordering::Relaxed);
        }
        EngineEvent::Risk(risk) => {
            let mut block = risk.observed_block;
            if let Some(pool) = projected_pool_from_risk_event(risk) {
                block = Some(block.unwrap_or(pool.latest_block).max(pool.latest_block));
                adapter
                    .pools()
                    .lock()
                    .expect("pool lock")
                    .insert(pool.address.clone(), pool);
            }
            if let Some(block) = block {
                adapter
                    .current_block()
                    .store(block, std::sync::atomic::Ordering::Relaxed);
            }
        }
        EngineEvent::Execution(report) => {
            if let Some(block) = report.block_number {
                adapter
                    .current_block()
                    .store(block, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}
