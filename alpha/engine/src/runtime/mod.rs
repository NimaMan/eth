mod event_flow;
mod execution_flow;

use eth_alpha_core::{
    amount::DecimalAmount,
    error::Result,
    execution::ExecutionReport,
    market::{MarketEvent, MarketSnapshotRef},
    position::PositionSnapshot,
    risk::{RiskEvent, RiskKind, RiskPolicy},
    store::TradingStore,
    strategy::StrategyContext,
};

use crate::{
    valuation::{market_open_valuation_pool, snapshot_with_pool_metrics},
    AlphaEngine, EngineEvent, EngineExecutionAdapter,
};

use event_flow::{engine_event_block, market_event_block};

impl<E, R, S> AlphaEngine<E, R, S>
where
    E: EngineExecutionAdapter,
    R: RiskPolicy,
    S: TradingStore,
{
    pub async fn handle_event(&mut self, event: EngineEvent) -> Result<Vec<ExecutionReport>> {
        self.current_event_block = engine_event_block(&event);
        match event {
            EngineEvent::Market(event) => {
                let market_valuation_pool = market_open_valuation_pool(&event).cloned();
                let mut reports = self
                    .apply_due_pending_execution_reports(
                        market_event_block(&event),
                        market_valuation_pool.as_ref(),
                    )
                    .await?;
                self.apply_market_event(&event);
                reports.extend(
                    self.run_market_strategies(&event, market_valuation_pool.as_ref())
                        .await?,
                );
                Ok(reports)
            }
            EngineEvent::Risk(event) => {
                let mut reports = if let Some(block_number) = event.observed_block {
                    self.apply_due_pending_execution_reports(block_number, None)
                        .await?
                } else {
                    Vec::new()
                };
                self.active_risks.push(event.clone());
                self.store.record_risk_event(&event).await?;
                reports.extend(self.run_risk_strategies(&event).await?);
                Ok(reports)
            }
            EngineEvent::Execution(report) => {
                self.store.record_execution_report(&report).await?;
                Ok(vec![report])
            }
        }
    }

    fn apply_market_event(&mut self, event: &MarketEvent) {
        match event {
            MarketEvent::TokenUpdated {
                block_number,
                token,
            } => {
                self.market = Some(MarketSnapshotRef {
                    block_number: *block_number,
                    token_address: token.address,
                    pool_address: None,
                    token: Some(token.clone()),
                    pool: None,
                });
            }
            MarketEvent::PoolUpdated { block_number, pool } => {
                self.pool_snapshots
                    .insert(pool.address.clone(), pool.clone());
                self.market = Some(MarketSnapshotRef {
                    block_number: *block_number,
                    token_address: pool.token_address,
                    pool_address: Some(pool.address.clone()),
                    token: None,
                    pool: Some(pool.clone()),
                });
            }
            MarketEvent::BlockCompleted { block_number, .. } => {
                if let Some(market) = self.market.as_mut() {
                    market.block_number = *block_number;
                }
            }
        }
    }

    async fn run_market_strategies(
        &mut self,
        event: &MarketEvent,
        market_valuation_pool: Option<&eth_alpha_core::ids::TokenPoolId>,
    ) -> Result<Vec<ExecutionReport>> {
        let market = match self.market.clone() {
            Some(market) => market,
            None if matches!(event, MarketEvent::BlockCompleted { .. }) => MarketSnapshotRef {
                block_number: market_event_block(event),
                token_address: Default::default(),
                pool_address: None,
                token: None,
                pool: None,
            },
            None => return Ok(Vec::new()),
        };
        let portfolio = self.portfolio.clone();
        let active_risks = self.active_risks.clone();
        let ctx = StrategyContext {
            market: &market,
            portfolio: &portfolio,
            active_risks: &active_risks,
        };

        let mut market_decisions = Vec::with_capacity(self.strategies.len());
        let mut monitor_decisions = Vec::new();
        let mut market_decisions_to_record = Vec::new();
        let mut monitor_decisions_to_record = Vec::new();
        for strategy in &mut self.strategies {
            let strategy_name = strategy.name();
            let decision = strategy.on_market_event(&ctx, event)?;
            if !matches!(event, MarketEvent::BlockCompleted { .. }) {
                market_decisions_to_record.push((strategy_name.0.clone(), decision.clone()));
            }
            market_decisions.push(decision);
            if let MarketEvent::BlockCompleted { block_number, .. } = event {
                let decisions = strategy.on_position_monitor(&ctx, *block_number)?;
                for (index, decision) in decisions.iter().enumerate() {
                    monitor_decisions_to_record.push((
                        strategy_name.0.clone(),
                        *block_number,
                        index,
                        decision.clone(),
                    ));
                }
                monitor_decisions.extend(decisions);
            }
        }
        for (strategy_name, decision) in market_decisions_to_record {
            self.record_strategy_decision(&strategy_name, "market", event, &decision)
                .await?;
        }
        for (strategy_name, block_number, index, decision) in monitor_decisions_to_record {
            self.record_position_monitor_decision(&strategy_name, block_number, index, &decision)
                .await?;
        }
        let mut reports = self
            .apply_decisions(market_decisions, "market", market_valuation_pool)
            .await?;
        reports.extend(
            self.apply_decisions(monitor_decisions, "position_monitor", market_valuation_pool)
                .await?,
        );
        self.snapshot_open_positions_for_pool(event).await?;
        Ok(reports)
    }

    async fn run_risk_strategies(&mut self, event: &RiskEvent) -> Result<Vec<ExecutionReport>> {
        let cached_pool = event
            .pool_address
            .as_ref()
            .and_then(|pool_address| self.pool_snapshots.get(pool_address))
            .filter(|pool| pool.token_address == event.token_address)
            .cloned();
        let market = self
            .market
            .clone()
            .filter(|market| {
                market.token_address == event.token_address
                    && event
                        .pool_address
                        .as_ref()
                        .map(|pool| Some(pool) == market.pool_address.as_ref())
                        .unwrap_or(true)
            })
            .or_else(|| {
                cached_pool.as_ref().map(|pool| MarketSnapshotRef {
                    block_number: event.observed_block.unwrap_or(pool.latest_block),
                    token_address: event.token_address,
                    pool_address: Some(pool.address.clone()),
                    token: None,
                    pool: Some(pool.clone()),
                })
            })
            .unwrap_or_else(|| MarketSnapshotRef {
                block_number: event.observed_block.unwrap_or_default(),
                token_address: event.token_address,
                pool_address: event.pool_address.clone(),
                token: None,
                pool: None,
            });
        let portfolio = self.portfolio.clone();
        let active_risks = self.active_risks.clone();
        let ctx = StrategyContext {
            market: &market,
            portfolio: &portfolio,
            active_risks: &active_risks,
        };

        let mut decisions = Vec::with_capacity(self.strategies.len());
        let mut risk_decisions_to_record = Vec::new();
        for strategy in &mut self.strategies {
            let strategy_name = strategy.name();
            let decision = strategy.on_risk_event(&ctx, event)?;
            risk_decisions_to_record.push((strategy_name.0.clone(), decision.clone()));
            decisions.push(decision);
        }
        for (strategy_name, decision) in risk_decisions_to_record {
            self.record_risk_strategy_decision(&strategy_name, event, &decision)
                .await?;
        }
        let event_source = event.source.as_deref().unwrap_or("risk");
        let reports = self.apply_decisions(decisions, event_source, None).await?;

        // Worst-case baseline: mark open positions as drained on
        // liquidity removal or scam confirmation, even if strategy does not exit.
        if matches!(
            event.kind,
            RiskKind::LiquidityRemoval | RiskKind::ScamConfirmed
        ) {
            if let Some(ref pool_address) = event.pool_address {
                let mut drained_snapshots = Vec::new();
                for position in self.portfolio.positions.values_mut() {
                    if position.key.pool_address == *pool_address
                        && position.has_exposure()
                        && !position.drained
                    {
                        position.mark_drained();
                        let _ = self.store.upsert_position(position).await;
                        // Snapshot the drained state so baseline PnL is honest
                        // even when no pool update follows the signal.
                        // Use a high block number so this snapshot is picked as
                        // the latest by DISTINCT ON ... ORDER BY block_number DESC.
                        let block_number = event.observed_block.unwrap_or(u64::MAX - 1);
                        let snapshot = PositionSnapshot {
                            position_id: position.id.clone(),
                            trade_id: position.trade_id.clone(),
                            state: position.state.clone(),
                            block_number,
                            observed_block_number: event.observed_block,
                            valuation_block_number: event.observed_block,
                            current_value_eth: DecimalAmount::ZERO,
                            realized_profit_eth: position.realized_pnl(),
                            unrealized_profit_eth: -position.entry_cost_basis.unwrap_or_default(),
                            roi: DecimalAmount::from(-1),
                            pool_price_to_initial_price_ratio: None,
                            pool_initial_price_denom_per_token: None,
                            pool_price_denom_per_token: None,
                            pool_liquidity_denom: None,
                            pool_token_reserve: None,
                            pool_denom_symbol: None,
                        };
                        let pool_snapshot = self.pool_snapshots.get(pool_address).filter(|pool| {
                            pool.token_address == event.token_address
                                && event
                                    .observed_block
                                    .map(|observed_block| pool.latest_block == observed_block)
                                    .unwrap_or(false)
                        });
                        let snapshot = snapshot_with_pool_metrics(snapshot, pool_snapshot);
                        drained_snapshots.push(snapshot);
                    }
                }
                for snapshot in drained_snapshots {
                    let _ = self.append_position_snapshot_once(snapshot).await;
                }
            }
        }

        Ok(reports)
    }
}
