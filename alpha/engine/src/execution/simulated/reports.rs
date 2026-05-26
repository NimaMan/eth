use alloy_primitives::U256;
use eth_alpha_core::{
    amount::Amount,
    execution::{ExecutionReport, ExecutionStatus},
    ids::OrderId,
    order::{OrderIntent, OrderSide},
    position::Position,
};

use crate::PositionValueSimulation;

pub(super) fn failed_report(order_id: OrderId, reason: impl Into<String>) -> ExecutionReport {
    failed_report_with_block(order_id, reason, None)
}

pub(super) fn failed_report_at(
    order_id: OrderId,
    reason: impl Into<String>,
    block: u64,
) -> ExecutionReport {
    failed_report_with_block(order_id, reason, Some(block))
}

pub(super) fn cancelled_report_at(
    order_id: OrderId,
    reason: impl Into<String>,
    block: u64,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Cancelled,
        tx_hash: None,
        block_number: Some(block),
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: Some(reason.into()),
    }
}

pub(super) fn failed_report_at_with_gas(
    order_id: OrderId,
    reason: impl Into<String>,
    block: u64,
    gas_used: Option<u64>,
    gas_cost: Option<U256>,
) -> ExecutionReport {
    let mut report = failed_report_with_block(order_id, reason, Some(block));
    report.gas_used = gas_used;
    report.gas_cost = gas_cost.map(gas_cost_amount);
    report
}

pub(super) fn failed_report_with_block(
    order_id: OrderId,
    reason: impl Into<String>,
    block_number: Option<u64>,
) -> ExecutionReport {
    ExecutionReport {
        order_id,
        status: ExecutionStatus::Failed,
        tx_hash: None,
        block_number,
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: Some(reason.into()),
    }
}

pub(super) fn submitted_live_chain_sim_report(
    order_id: OrderId,
    submitted_block: u64,
    expected_confirmation_block: u64,
) -> ExecutionReport {
    let mut report = ExecutionReport {
        order_id,
        status: ExecutionStatus::Submitted,
        tx_hash: None,
        block_number: Some(submitted_block),
        filled_amount: None,
        token_amount: None,
        gas_used: None,
        gas_cost: None,
        mined_evidence: None,
        error: None,
    };
    let mut evidence = report.mined_evidence.unwrap_or_default();
    evidence.submitted_block_number = Some(submitted_block);
    evidence.expected_confirmation_block = Some(expected_confirmation_block);
    evidence.receipt_status = Some("live_backtest_chain_sim_submitted".to_string());
    report.mined_evidence = Some(evidence);
    report
}

pub(super) fn with_live_chain_sim_evidence(
    mut report: ExecutionReport,
    decision_block: u64,
    expected_confirmation_block: u64,
    simulation_block: Option<u64>,
) -> ExecutionReport {
    let receipt_block = report.block_number;
    let mut evidence = report.mined_evidence.unwrap_or_default();
    evidence.submitted_block_number = Some(decision_block);
    evidence.expected_confirmation_block = Some(expected_confirmation_block);
    evidence.receipt_block_number = receipt_block;
    evidence.simulation_block_number = simulation_block;
    evidence.confirmation_lag_blocks =
        receipt_block.map(|block| block as i64 - expected_confirmation_block as i64);
    evidence.receipt_status = Some("live_backtest_chain_sim".to_string());
    report.mined_evidence = Some(evidence);
    report
}

pub(super) fn gas_cost_amount(raw: U256) -> Amount {
    Amount { raw, decimals: 18 }
}

pub(super) fn sell_intent_for_position(position: &Position) -> Option<OrderIntent> {
    Some(OrderIntent {
        trade_id: Some(position.trade_id.clone()),
        portfolio_id: position.key.portfolio_id.clone(),
        wallet_id: position.key.wallet_id.clone(),
        strategy_name: position.key.strategy_name.clone(),
        side: OrderSide::Sell,
        token_address: position.key.token_address,
        pool_address: position.key.pool_address.clone(),
        protocol: position.key.protocol.clone(),
        amount: position.entry_token_raw_amount.clone()?,
        route: None,
        max_slippage_bps: 0,
        deadline_secs: 0,
        decision_reason: None,
    })
}

pub(super) fn position_value_from_report(
    report: ExecutionReport,
    block_number: u64,
) -> Option<PositionValueSimulation> {
    match report.status {
        ExecutionStatus::Confirmed => {
            let current_value = report.filled_amount?;
            Some(PositionValueSimulation {
                block_number: report.block_number.unwrap_or(block_number),
                current_value,
                gas_used: report.gas_used,
                error: None,
            })
        }
        ExecutionStatus::Failed => {
            let error = report
                .error
                .unwrap_or_else(|| "chain-sim position valuation failed".to_string());
            Some(PositionValueSimulation {
                block_number,
                current_value: Amount::zero(18),
                gas_used: report.gas_used,
                error: Some(error),
            })
        }
        ExecutionStatus::Submitted
        | ExecutionStatus::Pending
        | ExecutionStatus::Deferred
        | ExecutionStatus::Cancelled => None,
    }
}

pub(super) fn unique_order_prefix() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    format!("chain-sim-{}-{millis}", std::process::id())
}
