use eth_alpha_core::{
    amount::DecimalAmount,
    execution::{ExecutionReport, ExecutionStatus},
    market::MarketEvent,
    order::OrderSide,
};

use crate::EngineEvent;

pub(crate) fn submitted_report_for(
    report: &ExecutionReport,
    block_number: Option<u64>,
) -> ExecutionReport {
    ExecutionReport {
        order_id: report.order_id.clone(),
        status: ExecutionStatus::Submitted,
        tx_hash: report.tx_hash,
        block_number,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: None,
    }
}

pub(crate) fn should_record_submitted_report(report: &ExecutionReport) -> bool {
    !matches!(
        report.status,
        ExecutionStatus::Submitted | ExecutionStatus::Deferred | ExecutionStatus::Cancelled
    )
}

pub(crate) fn should_defer_report(submission_block: Option<u64>, report: &ExecutionReport) -> bool {
    if matches!(
        report.status,
        ExecutionStatus::Submitted | ExecutionStatus::Pending | ExecutionStatus::Deferred
    ) {
        return false;
    }
    match (submission_block, report.block_number) {
        (Some(submitted_at), Some(executed_at)) => executed_at > submitted_at,
        _ => false,
    }
}

pub(crate) fn fill_price_for_report(
    side: OrderSide,
    report: &ExecutionReport,
) -> Option<DecimalAmount> {
    if side != OrderSide::Buy {
        return None;
    }
    let (Some(cost), Some(tokens)) = (&report.filled_amount, &report.token_amount) else {
        return None;
    };
    let token_dec = tokens.to_decimal();
    if token_dec.is_zero() {
        return None;
    }
    Some(cost.to_decimal() / token_dec)
}

pub(crate) fn engine_event_block(event: &EngineEvent) -> Option<u64> {
    match event {
        EngineEvent::Market(
            MarketEvent::TokenUpdated { block_number, .. }
            | MarketEvent::PoolUpdated { block_number, .. }
            | MarketEvent::BlockCompleted { block_number, .. },
        ) => Some(*block_number),
        EngineEvent::Risk(risk) => risk.observed_block,
        EngineEvent::Execution(report) => report.block_number,
    }
}

pub(crate) fn market_event_block(event: &MarketEvent) -> u64 {
    match event {
        MarketEvent::TokenUpdated { block_number, .. }
        | MarketEvent::PoolUpdated { block_number, .. }
        | MarketEvent::BlockCompleted { block_number, .. } => *block_number,
    }
}
