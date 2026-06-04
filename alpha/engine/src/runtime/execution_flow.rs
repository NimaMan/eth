use std::collections::HashSet;

use eth_alpha_core::{
    amount::DecimalAmount,
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus},
    ids::{PositionId, TokenPoolId},
    order::{OrderIntent, OrderSide},
    position::{Position, PositionKey, PositionSnapshot, PositionState},
    risk::{RiskDecision, RiskEvent, RiskPolicy},
    store::TradingStore,
    strategy::StrategyDecision,
};

use crate::{
    decision::{enrich_decision_reason_with_risk_event, strategy_decision_action},
    ids::new_trade_id,
    valuation::{
        market_owns_open_valuation, snapshot_with_pool_metrics, valuation_safe_pool,
        zero_value_snapshot,
    },
    AlphaEngine, EngineExecutionAdapter, PendingExecutionReport,
};

use super::event_flow::{
    fill_price_for_report, should_defer_report, should_record_submitted_report,
    submitted_report_for,
};

impl<E, R, S> AlphaEngine<E, R, S>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    pub async fn flush_pending_executions(&mut self) -> Result<Vec<ExecutionReport>> {
        let pending = std::mem::take(&mut self.pending_execution_reports);
        self.apply_pending_execution_reports(pending, None).await
    }

    // `pub` (hidden) so the live runner's operator manual-close flow can apply
    // strategy decisions directly; the related `apply_*` helpers stay
    // crate-internal since live wiring only needs this entrypoint.
    #[doc(hidden)]
    pub async fn apply_decisions(
        &mut self,
        decisions: Vec<StrategyDecision>,
        event_source: &str,
        market_valuation_pool: Option<&TokenPoolId>,
    ) -> Result<Vec<ExecutionReport>> {
        self.apply_decisions_with_risk_context(decisions, event_source, market_valuation_pool, None)
            .await
    }

    pub(crate) async fn apply_risk_decisions(
        &mut self,
        decisions: Vec<StrategyDecision>,
        event_source: &str,
        risk_event: &RiskEvent,
    ) -> Result<Vec<ExecutionReport>> {
        self.apply_decisions_with_risk_context(decisions, event_source, None, Some(risk_event))
            .await
    }

    async fn apply_decisions_with_risk_context(
        &mut self,
        decisions: Vec<StrategyDecision>,
        event_source: &str,
        market_valuation_pool: Option<&TokenPoolId>,
        risk_event: Option<&RiskEvent>,
    ) -> Result<Vec<ExecutionReport>> {
        let mut reports = Vec::new();
        for decision in decisions {
            let action = strategy_decision_action(&decision);
            let mut structured_reason =
                decision.structured_reason(Some(event_source), Some(action));
            if let (Some(reason), Some(event)) = (structured_reason.as_mut(), risk_event) {
                enrich_decision_reason_with_risk_event(reason, event);
            }
            match decision {
                StrategyDecision::Hold
                | StrategyDecision::HoldWithReason { .. }
                | StrategyDecision::CancelOrders { .. } => {}
                StrategyDecision::SubmitOrder(mut intent)
                | StrategyDecision::SubmitOrderWithReason { mut intent, .. } => {
                    // Operator buy-halt ("exit-only"): when this strategy is
                    // paused, drop new BUY entries before any risk/execution.
                    // Sells, strategy exits and manual closes are always
                    // OrderSide::Sell, so they are never affected here.
                    if buy_entry_halted(
                        &self.halt_buys_strategies,
                        intent.side,
                        &intent.strategy_name.0,
                    ) {
                        tracing::info!(
                            strategy = %intent.strategy_name.0,
                            token = %intent.token_address,
                            pool = %intent.pool_address.0,
                            reason = "entry.paused_by_operator",
                            "buy entry skipped: strategy buys paused by operator (exit-only)"
                        );
                        continue;
                    }
                    intent.decision_reason = structured_reason;
                    reports.extend(
                        self.execute_if_allowed(intent, market_valuation_pool)
                            .await?,
                    );
                }
            }
        }
        Ok(reports)
    }

    async fn execute_if_allowed(
        &mut self,
        intent: OrderIntent,
        market_valuation_pool: Option<&TokenPoolId>,
    ) -> Result<Vec<ExecutionReport>> {
        match self.risk_policy.evaluate_order(&intent, &self.active_risks) {
            RiskDecision::Allow | RiskDecision::ReduceSize { .. } => {
                self.execute_intent(intent, market_valuation_pool).await
            }
            RiskDecision::ForceExit { intent, .. } => {
                self.execute_intent(*intent, market_valuation_pool).await
            }
            RiskDecision::Reject { .. } | RiskDecision::CancelOpenOrders { .. } => {
                self.store.record_order_intent(&intent).await?;
                Ok(Vec::new())
            }
        }
    }

    async fn execute_intent(
        &mut self,
        mut intent: OrderIntent,
        market_valuation_pool: Option<&TokenPoolId>,
    ) -> Result<Vec<ExecutionReport>> {
        let mut position = self.position_for_intent(&intent);
        intent.trade_id = Some(position.trade_id.clone());
        self.store.record_order_intent(&intent).await?;
        position.mark_intent_created(intent.side)?;
        let report = self.execution.execute(intent.clone()).await?;
        position.mark_order_submitted(report.order_id.clone(), intent.side)?;
        self.store.upsert_position(&position).await?;
        let submission_block = self
            .current_event_block
            .or_else(|| self.market.as_ref().map(|m| m.block_number));
        let mut reports = Vec::new();
        if should_record_submitted_report(&report) {
            let submitted_report =
                submitted_report_for(&report, submission_block.or(report.block_number));
            self.store
                .record_order_execution_report(&position.id, intent.side, &submitted_report)
                .await?;
            reports.push(submitted_report);
        }

        self.store.upsert_position(&position).await?;
        let position_id = position.id.clone();
        self.portfolio
            .positions
            .insert(position.id.clone(), position);

        if should_defer_report(submission_block, &report) {
            self.pending_execution_reports.push(PendingExecutionReport {
                position_id,
                side: intent.side,
                report,
            });
        } else {
            self.apply_final_execution_report(
                &position_id,
                intent.side,
                &report,
                market_valuation_pool,
            )
            .await?;
            reports.push(report);
        }

        Ok(reports)
    }

    pub(crate) async fn apply_due_pending_execution_reports(
        &mut self,
        block_number: u64,
        market_valuation_pool: Option<&TokenPoolId>,
    ) -> Result<Vec<ExecutionReport>> {
        let mut due = Vec::new();
        let mut remaining = Vec::new();
        for pending in self.pending_execution_reports.drain(..) {
            let is_due = pending
                .report
                .block_number
                .map(|execution_block| execution_block <= block_number)
                .unwrap_or(true);
            if is_due {
                due.push(pending);
            } else {
                remaining.push(pending);
            }
        }
        self.pending_execution_reports = remaining;
        self.apply_pending_execution_reports(due, market_valuation_pool)
            .await
    }

    pub(crate) async fn apply_pending_execution_reports(
        &mut self,
        mut pending_reports: Vec<PendingExecutionReport>,
        market_valuation_pool: Option<&TokenPoolId>,
    ) -> Result<Vec<ExecutionReport>> {
        pending_reports.sort_by_key(|pending| pending.report.block_number.unwrap_or_default());
        let mut reports = Vec::with_capacity(pending_reports.len());
        for pending in pending_reports {
            self.apply_final_execution_report(
                &pending.position_id,
                pending.side,
                &pending.report,
                market_valuation_pool,
            )
            .await?;
            reports.push(pending.report);
        }
        Ok(reports)
    }

    pub(crate) async fn apply_external_execution_report(
        &mut self,
        report: ExecutionReport,
    ) -> Result<ExecutionReport> {
        if !matches!(
            report.status,
            ExecutionStatus::Confirmed
                | ExecutionStatus::Deferred
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
        ) {
            self.store.record_execution_report(&report).await?;
            return Ok(report);
        }

        let matched = self
            .portfolio
            .positions
            .iter()
            .find_map(|(position_id, position)| {
                if position.entry_order_id.as_ref() == Some(&report.order_id) {
                    Some((position_id.clone(), OrderSide::Buy))
                } else if position.exit_order_id.as_ref() == Some(&report.order_id) {
                    Some((position_id.clone(), OrderSide::Sell))
                } else {
                    None
                }
            });

        let Some((position_id, side)) = matched else {
            self.store.record_execution_report(&report).await?;
            return Ok(report);
        };

        self.apply_final_execution_report(&position_id, side, &report, None)
            .await?;
        Ok(report)
    }

    async fn apply_final_execution_report(
        &mut self,
        position_id: &PositionId,
        side: OrderSide,
        report: &ExecutionReport,
        market_valuation_pool: Option<&TokenPoolId>,
    ) -> Result<()> {
        let mut position = self
            .portfolio
            .positions
            .get(position_id)
            .cloned()
            .ok_or_else(|| {
                AlphaCoreError::InvalidPositionTransition(format!(
                    "execution report {} has no matching position {}",
                    report.order_id.0, position_id.0
                ))
            })?;
        let report_status = report.status.clone();
        let fill_price = fill_price_for_report(side, report);
        position.apply_execution_report_with_price(report, fill_price)?;

        self.store.upsert_position(&position).await?;
        self.store
            .record_order_execution_report(&position.id, side, report)
            .await?;

        if matches!(
            report_status,
            ExecutionStatus::Failed | ExecutionStatus::Cancelled
        ) {
            if position.drained {
                // A confiscated/drained position cannot be sold; a failed/cancelled sell
                // must still close it terminally at zero value rather than leaving it open.
                if !position.state.is_terminal() {
                    position.mark_terminal_zero();
                    self.store.upsert_position(&position).await?;
                }
                let block_number = report
                    .block_number
                    .or_else(|| self.market.as_ref().map(|m| m.block_number))
                    .unwrap_or_default();
                let pool = self
                    .pool_snapshots
                    .get(&position.key.pool_address)
                    .filter(|pool| {
                        pool.token_address == position.key.token_address
                            && pool.latest_block == block_number
                    });
                let snapshot = zero_value_snapshot(&position, block_number, pool);
                self.append_position_snapshot_once(snapshot).await?;
            }
        } else if position.is_closed() {
            let block_number = report
                .block_number
                .or(self.current_event_block)
                .or_else(|| self.market.as_ref().map(|m| m.block_number))
                .unwrap_or_default();
            let pool = valuation_safe_pool(
                self.pool_snapshots.get(&position.key.pool_address),
                block_number,
            );
            let snapshot = PositionSnapshot {
                position_id: position.id.clone(),
                trade_id: position.trade_id.clone(),
                state: position.state.clone(),
                block_number,
                observed_block_number: pool.map(|pool| pool.latest_block).or(Some(block_number)),
                valuation_block_number: Some(block_number),
                current_value_eth: DecimalAmount::ZERO,
                realized_profit_eth: position.realized_pnl(),
                unrealized_profit_eth: DecimalAmount::ZERO,
                roi: if let Some(cost) = position.entry_cost_basis {
                    if !cost.is_zero() {
                        position.realized_pnl() / cost
                    } else {
                        DecimalAmount::ZERO
                    }
                } else {
                    DecimalAmount::ZERO
                },
                pool_price_to_initial_price_ratio: None,
                pool_initial_price_denom_per_token: None,
                pool_price_denom_per_token: None,
                pool_liquidity_denom: None,
                pool_token_reserve: None,
                pool_denom_symbol: None,
            };
            let snapshot = snapshot_with_pool_metrics(snapshot, pool);
            self.append_position_snapshot_once(snapshot).await?;
        } else if side == OrderSide::Buy
            && report_status == ExecutionStatus::Confirmed
            && !market_owns_open_valuation(market_valuation_pool, &position)
        {
            self.snapshot_confirmed_buy_position(&position).await?;
        }

        self.portfolio
            .positions
            .insert(position.id.clone(), position);
        Ok(())
    }

    fn position_for_intent(&self, intent: &OrderIntent) -> Position {
        let key = PositionKey {
            portfolio_id: intent.portfolio_id.clone(),
            wallet_id: intent.wallet_id.clone(),
            strategy_name: intent.strategy_name.clone(),
            token_address: intent.token_address,
            pool_address: intent.pool_address.clone(),
            protocol: intent.protocol.clone(),
        };
        if let Some(position) = self
            .portfolio
            .positions
            .values()
            .find(|position| {
                intent
                    .trade_id
                    .as_ref()
                    .map(|trade_id| position.trade_id == *trade_id)
                    .unwrap_or(false)
            })
            .cloned()
        {
            return position;
        }
        if intent.side == OrderSide::Sell {
            if let Some(position) = self
                .portfolio
                .positions
                .values()
                .find(|position| position.key == key && position.can_submit_exit())
                .cloned()
            {
                return position;
            }
        } else if let Some(position) = self
            .portfolio
            .positions
            .values()
            .find(|position| position.key == key && position.state == PositionState::BuyDeferred)
            .cloned()
        {
            return position;
        }
        let trade_id = intent.trade_id.clone().unwrap_or_else(new_trade_id);
        let id = PositionId(trade_id.0.clone());
        Position::with_trade_id(id, trade_id, key)
    }
}

/// True when a new BUY entry should be skipped because an operator has paused
/// buys for this strategy ("exit-only"). Only buys are gated — sells, strategy
/// exits and manual closes (always `OrderSide::Sell`) always pass through.
fn buy_entry_halted(halt_buys: &HashSet<String>, side: OrderSide, strategy_name: &str) -> bool {
    side == OrderSide::Buy && halt_buys.contains(strategy_name)
}

#[cfg(test)]
mod buy_halt_tests {
    use super::buy_entry_halted;
    use eth_alpha_core::order::OrderSide;
    use std::collections::HashSet;

    #[test]
    fn halts_only_buys_for_paused_strategy() {
        let halt = HashSet::from(["alpha-A".to_string()]);
        // Buy for the paused strategy is halted...
        assert!(buy_entry_halted(&halt, OrderSide::Buy, "alpha-A"));
        // ...but its sells / exits / manual closes still flow.
        assert!(!buy_entry_halted(&halt, OrderSide::Sell, "alpha-A"));
        // A different, unpaused strategy keeps buying.
        assert!(!buy_entry_halted(&halt, OrderSide::Buy, "alpha-B"));
    }

    #[test]
    fn empty_halt_set_allows_all_buys() {
        let halt: HashSet<String> = HashSet::new();
        assert!(!buy_entry_halted(&halt, OrderSide::Buy, "alpha-A"));
    }
}
