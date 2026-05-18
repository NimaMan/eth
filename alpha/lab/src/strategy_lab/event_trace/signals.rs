use std::collections::BTreeSet;

use super::model::{
    DecisionRow, RiskEventRow, SignalCandidate, SnapshotRow, TradeRecord, TradeTiming,
};

pub fn exit_reason(decisions: &[DecisionRow], timing: &TradeTiming) -> Option<String> {
    let sell_block = timing.sell_submitted_block;
    decisions
        .iter()
        .filter(|decision| decision.action == "submit_sell")
        .filter(|decision| {
            sell_block
                .map(|block| decision.block_number == Some(block))
                .unwrap_or(true)
        })
        .filter_map(|decision| decision.reason.clone())
        .next()
        .or_else(|| {
            decisions
                .iter()
                .filter(|decision| decision.action == "submit_sell")
                .filter_map(|decision| decision.reason.clone())
                .next()
        })
}

pub fn detect_signal_candidates(
    trade: &TradeRecord,
    timing: &TradeTiming,
    exit_reason: Option<&str>,
    risk_events: &[RiskEventRow],
    snapshots: &[SnapshotRow],
) -> Vec<SignalCandidate> {
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();

    if let (Some(lp_block), Some(buy_submitted)) =
        (timing.first_lp_approval_block, timing.buy_submitted_block)
    {
        if lp_block < buy_submitted {
            push_once(
                &mut candidates,
                &mut seen,
                "lp_approval_before_buy_submit",
                "critical",
                Some(lp_block),
                "LP approval was visible before buy submission; entry gate should block this pool.",
            );
        }
    }

    if let (Some(lp_block), Some(buy_confirmed)) =
        (timing.first_lp_approval_block, timing.buy_confirmed_block)
    {
        if lp_block == buy_confirmed {
            push_once(
                &mut candidates,
                &mut seen,
                "lp_approval_in_buy_confirm_block",
                "warn",
                Some(lp_block),
                "LP approval appears in the same block as buy confirmation; this is the debated timing edge.",
            );
        } else if lp_block > buy_confirmed {
            push_once(
                &mut candidates,
                &mut seen,
                "lp_approval_after_entry",
                "info",
                Some(lp_block),
                "LP approval appeared after entry and can be evaluated as an exit signal.",
            );
        }
    }

    if let (Some(lp_block), Some(sell_submitted)) =
        (timing.first_lp_approval_block, timing.sell_submitted_block)
    {
        if lp_block <= sell_submitted {
            push_once(
                &mut candidates,
                &mut seen,
                "lp_approval_visible_by_sell_submit",
                "info",
                Some(lp_block),
                "LP approval was visible no later than the sell submission block.",
            );
        }
    }

    if let (Some(lp_block), Some(removal_block)) = (
        timing.first_lp_approval_block,
        timing.first_liquidity_removal_block,
    ) {
        let lead = removal_block.saturating_sub(lp_block);
        if lead <= 1 {
            push_once(
                &mut candidates,
                &mut seen,
                "lp_approval_lead_lte_1_block",
                "warn",
                Some(lp_block),
                "LP approval gave at most one block of warning before direct liquidity removal.",
            );
        }
    }

    if let (Some(removal_block), Some(sell_submitted)) = (
        timing.first_liquidity_removal_block,
        timing.sell_submitted_block,
    ) {
        if removal_block <= sell_submitted {
            push_once(
                &mut candidates,
                &mut seen,
                "sell_submitted_after_liquidity_removal",
                "critical",
                Some(removal_block),
                "Direct liquidity removal was already visible before or at sell submission.",
            );
        }
    }

    if let (Some(removal_block), Some(sell_submitted), Some(sell_confirmed)) = (
        timing.first_liquidity_removal_block,
        timing.sell_submitted_block,
        timing.sell_confirmed_block,
    ) {
        if sell_submitted < removal_block && removal_block <= sell_confirmed {
            push_once(
                &mut candidates,
                &mut seen,
                "sell_confirmation_raced_liquidity_removal",
                "critical",
                Some(removal_block),
                "Sell was submitted before liquidity removal but confirmed at or after removal.",
            );
        }
    }

    if matches!(exit_reason, Some(reason) if reason.contains("max_hold")) && is_loss(trade) {
        push_once(
            &mut candidates,
            &mut seen,
            "max_hold_exit_loser",
            "warn",
            timing.sell_submitted_block,
            "The trade lost money while exiting by max active-hold.",
        );
    }

    if current_value_crossed_below_entry(trade, snapshots) {
        push_once(
            &mut candidates,
            &mut seen,
            "mark_to_market_below_entry_before_exit",
            "info",
            snapshots
                .iter()
                .find(|snapshot| {
                    snapshot.current_value_eth.parse::<f64>().unwrap_or(0.0)
                        < trade
                            .entry_cost_eth
                            .as_deref()
                            .and_then(|value| value.parse::<f64>().ok())
                            .unwrap_or(0.0)
                })
                .map(|snapshot| snapshot.block_number),
            "Mark-to-market value crossed below entry cost before the final exit.",
        );
    }

    if gas_is_material(trade) {
        push_once(
            &mut candidates,
            &mut seen,
            "gas_material_to_loss",
            "info",
            timing.sell_confirmed_block.or(timing.buy_confirmed_block),
            "Gas cost is at least 25% of the total trade loss.",
        );
    }

    if risk_events.is_empty() && is_loss(trade) {
        push_once(
            &mut candidates,
            &mut seen,
            "no_pool_risk_event_before_loss",
            "info",
            timing.exit_or_latest_block(),
            "The trade lost money without a captured LP approval or liquidity-removal risk event.",
        );
    }

    candidates
}

trait TimingExt {
    fn exit_or_latest_block(&self) -> Option<i64>;
}

impl TimingExt for TradeTiming {
    fn exit_or_latest_block(&self) -> Option<i64> {
        self.sell_confirmed_block
            .or(self.sell_submitted_block)
            .or(self.buy_confirmed_block)
    }
}

fn push_once(
    candidates: &mut Vec<SignalCandidate>,
    seen: &mut BTreeSet<String>,
    code: &str,
    severity: &str,
    block_number: Option<i64>,
    message: &str,
) {
    if !seen.insert(code.to_string()) {
        return;
    }
    candidates.push(SignalCandidate {
        code: code.to_string(),
        severity: severity.to_string(),
        block_number,
        message: message.to_string(),
    });
}

fn is_loss(trade: &TradeRecord) -> bool {
    trade
        .total_pnl_eth
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value < 0.0)
        .unwrap_or(false)
}

fn current_value_crossed_below_entry(trade: &TradeRecord, snapshots: &[SnapshotRow]) -> bool {
    let Some(entry_cost) = trade
        .entry_cost_eth
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok())
    else {
        return false;
    };
    snapshots
        .iter()
        .filter(|snapshot| Some(snapshot.block_number) < trade.exit_block)
        .any(|snapshot| {
            snapshot
                .current_value_eth
                .parse::<f64>()
                .map(|value| value < entry_cost)
                .unwrap_or(false)
        })
}

fn gas_is_material(trade: &TradeRecord) -> bool {
    let Some(gas) = trade
        .gas_cost_eth
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok())
    else {
        return false;
    };
    let loss = trade
        .total_pnl_eth
        .as_deref()
        .and_then(|value| value.parse::<f64>().ok())
        .map(f64::abs)
        .unwrap_or(0.0);
    loss > 0.0 && gas >= loss * 0.25
}
