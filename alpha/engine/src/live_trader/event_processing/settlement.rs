use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use eth_alpha_core::{risk::RiskPolicy, store::TradingStore};
use eth_alpha_store::PostgresTradingStore;
use eyre::Result;
use tracing::{info, warn};

use crate::wire::LiveStatusResponse;
use crate::{AlphaEngine, EngineEvent, EngineExecutionAdapter};

use super::super::execution_lifecycle::ChainSimSettlement;
use super::super::receipt_reconciliation::{JsonRpcReceiptProvider, VaultReceiptReconciler};

#[derive(Default)]
pub(in crate::live_trader) struct ChainSimSettlementSummary {
    pub(in crate::live_trader) loaded: usize,
    pub(in crate::live_trader) reports: usize,
    pub(in crate::live_trader) pending: usize,
    pub(in crate::live_trader) waiting_state: usize,
    pub(in crate::live_trader) missing_block: usize,
    pub(in crate::live_trader) total_reports: usize,
}

#[derive(Default)]
pub(in crate::live_trader) struct ReceiptReconciliationSummary {
    pub(in crate::live_trader) reports: usize,
    pub(in crate::live_trader) unresolved: usize,
    pub(in crate::live_trader) total_reports: usize,
}

pub(in crate::live_trader) async fn settle_chain_sim_executions<E, R, S>(
    settlement: Option<&ChainSimSettlement>,
    status: &LiveStatusResponse,
    suppress_events: bool,
    adapter_current_block: &Arc<AtomicU64>,
    engine: &mut AlphaEngine<E, R, S>,
) -> Result<ChainSimSettlementSummary>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    let mut summary = ChainSimSettlementSummary::default();
    if suppress_events {
        return Ok(summary);
    }
    let (Some(settlement), Some(block_number)) = (settlement, status.progress.current_block) else {
        return Ok(summary);
    };

    adapter_current_block.store(block_number, Ordering::Relaxed);
    match settlement.reports_due_at(block_number).await {
        Ok(batch) => {
            summary.loaded = batch.loaded;
            summary.pending = batch.pending_future_block;
            summary.waiting_state = batch.waiting_for_live_state;
            summary.missing_block = batch.missing_submission_block;
            for report in batch.reports {
                let event_reports = engine.handle_event(EngineEvent::Execution(report)).await?;
                summary.reports += event_reports.len();
                summary.total_reports += event_reports.len();
                for report in event_reports {
                    info!(
                        order_id = %report.order_id.0,
                        status = ?report.status,
                        block_number = ?report.block_number,
                        gas_used = ?report.gas_used,
                        error = ?report.error,
                        "chain-sim submitted execution settled"
                    );
                }
            }
        }
        Err(error) => {
            warn!(error = %error, "chain-sim submitted execution settlement failed");
        }
    }
    Ok(summary)
}

pub(in crate::live_trader) async fn reconcile_real_receipts<E, R, S>(
    reconciler: Option<&VaultReceiptReconciler<JsonRpcReceiptProvider>>,
    store: &PostgresTradingStore,
    status: &LiveStatusResponse,
    suppress_events: bool,
    engine: &mut AlphaEngine<E, R, S>,
) -> Result<ReceiptReconciliationSummary>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    let mut summary = ReceiptReconciliationSummary::default();
    if suppress_events {
        return Ok(summary);
    }
    let Some(reconciler) = reconciler else {
        return Ok(summary);
    };

    match store.load_submitted_executions(50).await {
        Ok(submitted) if submitted.is_empty() => {}
        Ok(submitted) => match reconciler
            .reconcile_after_processed_block(submitted, status.progress.current_block)
            .await
        {
            Ok(batch) => {
                summary.unresolved = batch.unresolved.len();
                for issue in batch.unresolved {
                    warn!(
                        order_id = %issue.order_id,
                        tx_hash = %issue.tx_hash,
                        reason = %issue.reason,
                        "real receipt reconciliation has no final vault evidence yet"
                    );
                }
                for report in batch.reports {
                    let event_reports = engine.handle_event(EngineEvent::Execution(report)).await?;
                    summary.reports += event_reports.len();
                    summary.total_reports += event_reports.len();
                    for report in event_reports {
                        info!(
                            order_id = %report.order_id.0,
                            status = ?report.status,
                            tx_hash = ?report.tx_hash,
                            block_number = ?report.block_number,
                            gas_used = ?report.gas_used,
                            error = ?report.error,
                            "real receipt reconciled execution report"
                        );
                    }
                }
            }
            Err(error) => {
                warn!(error = %error, "real receipt reconciliation failed");
            }
        },
        Err(error) => {
            warn!(error = %error, "failed to load submitted executions for receipt reconciliation");
        }
    }
    Ok(summary)
}
