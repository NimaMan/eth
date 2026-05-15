use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PoolAddress, StrategyName, TokenAddress},
    market::{MarketEvent, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    position::{Position, PositionState},
    risk::{RiskEvent, RiskKind, RiskSeverity},
    Result, Strategy, StrategyContext, StrategyDecision,
};

use crate::shared_rules;

use super::{
    config::SnipeAllConfig,
    rule::RuleDecision,
    rules::{creator_label, entry},
    state::SnipeAllState,
};

#[derive(Clone, Debug)]
pub struct SnipeAllStrategy {
    config: SnipeAllConfig,
    state: SnipeAllState,
}

impl SnipeAllStrategy {
    pub fn new(config: SnipeAllConfig) -> Self {
        Self {
            config,
            state: SnipeAllState::default(),
        }
    }

    pub fn with_bought_pools(
        config: SnipeAllConfig,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
    ) -> Self {
        Self {
            config,
            state: SnipeAllState::with_bought_pools(bought_pools),
        }
    }

    pub fn state(&self) -> &SnipeAllState {
        &self.state
    }

    fn buy_pool(&mut self, pool: &PoolSnapshot, reason: impl Into<String>) -> StrategyDecision {
        self.state.mark_bought(pool.address.clone());
        StrategyDecision::submit_order(
            OrderIntent {
                portfolio_id: self.config.portfolio_id.clone(),
                wallet_id: self.config.wallet_id.clone(),
                strategy_name: self.name(),
                side: OrderSide::Buy,
                token_address: pool.token_address,
                pool_address: pool.address.clone(),
                amount: self.config.buy_amount.clone(),
                route: None,
                max_slippage_bps: self.config.max_slippage_bps,
                deadline_secs: self.config.deadline_secs,
            },
            reason,
        )
    }

    fn sell_pool(
        &mut self,
        ctx: &StrategyContext<'_>,
        token_address: TokenAddress,
        pool_address: PoolAddress,
        reason: impl Into<String>,
    ) -> StrategyDecision {
        // Look up the open position to determine how many tokens to sell.
        let position = ctx.portfolio.positions.values().find(|p| {
            p.key.strategy_name == self.name()
                && p.key.token_address == token_address
                && p.key.pool_address == pool_address
                && p.can_submit_exit()
        });

        let Some(token_amount) = position
            .and_then(|position| sell_amount_from_position(position, self.config.sell_fraction))
        else {
            return StrategyDecision::hold("exit.no_sellable_position");
        };

        StrategyDecision::submit_order(
            OrderIntent {
                portfolio_id: self.config.portfolio_id.clone(),
                wallet_id: self.config.wallet_id.clone(),
                strategy_name: self.name(),
                side: OrderSide::Sell,
                token_address,
                pool_address,
                amount: token_amount,
                route: None,
                max_slippage_bps: self.config.max_slippage_bps,
                deadline_secs: self.config.deadline_secs,
            },
            reason,
        )
    }

    fn sell_position(&self, position: &Position, reason: impl Into<String>) -> StrategyDecision {
        let Some(token_amount) = sell_amount_from_position(position, self.config.sell_fraction)
        else {
            return StrategyDecision::hold("exit.no_token_amount");
        };

        StrategyDecision::submit_order(
            OrderIntent {
                portfolio_id: self.config.portfolio_id.clone(),
                wallet_id: self.config.wallet_id.clone(),
                strategy_name: self.name(),
                side: OrderSide::Sell,
                token_address: position.key.token_address,
                pool_address: position.key.pool_address.clone(),
                amount: token_amount,
                route: None,
                max_slippage_bps: self.config.max_slippage_bps,
                deadline_secs: self.config.deadline_secs,
            },
            reason,
        )
    }

    fn should_retry_failed_exit(&self, position: &Position, current_block: u64) -> bool {
        if position.state != PositionState::SellFailed || !position.can_submit_exit() {
            return false;
        }

        let Some(retry_interval) = self.config.exit_retry_interval_blocks else {
            return false;
        };

        if let Some(max_retries) = self.config.max_exit_retries {
            if position.exit_failure_count >= max_retries {
                return false;
            }
        }

        position
            .last_exit_failure_block
            .map(|last_failed| current_block >= last_failed.saturating_add(retry_interval))
            .unwrap_or(true)
    }

    /// Evaluate proactive price-ratio and time-based exits for an open position.
    /// Returns Some(decision) if an exit should be triggered, None otherwise.
    fn evaluate_proactive_exit(
        &mut self,
        ctx: &StrategyContext<'_>,
        position: &eth_alpha_core::position::Position,
        pool: &PoolSnapshot,
        current_block: u64,
    ) -> Option<StrategyDecision> {
        if !position.can_submit_exit() {
            return None;
        }

        // Time-based exit: sell once the configured hold window has elapsed.
        if let Some(max_hold) = self.config.max_hold_blocks {
            if let Some(entry_block) = position.entry_block {
                if current_block >= entry_block.saturating_add(max_hold) {
                    return Some(self.sell_pool(
                        ctx,
                        pool.token_address,
                        pool.address.clone(),
                        "exit.max_hold",
                    ));
                }
            }
        }

        // Price-ratio exits require entry_price.
        let Some(entry_price) = position.entry_price else {
            return None;
        };
        if entry_price.is_zero() {
            return None;
        }

        let current_price = pool.price_denom_per_token.unwrap_or_default();
        if current_price.is_zero() {
            return None;
        }

        let price_ratio = current_price / entry_price;

        // Stop-loss: price fell below threshold ratio.
        if let Some(sl_ratio) = self.config.stop_loss_ratio {
            if price_ratio <= sl_ratio {
                return Some(self.sell_pool(
                    ctx,
                    pool.token_address,
                    pool.address.clone(),
                    "exit.stop_loss",
                ));
            }
        }

        // Take-profit: price rose above threshold ratio.
        if let Some(tp_ratio) = self.config.take_profit_ratio {
            if price_ratio >= tp_ratio {
                return Some(self.sell_pool(
                    ctx,
                    pool.token_address,
                    pool.address.clone(),
                    "exit.take_profit",
                ));
            }
        }

        None
    }

    fn has_blocking_entry_risk(
        ctx: &StrategyContext<'_>,
        token_address: TokenAddress,
        pool_address: &PoolAddress,
    ) -> bool {
        ctx.active_risks.iter().rev().any(|risk| {
            risk.severity == RiskSeverity::Critical
                && risk.kind != RiskKind::TradingEnabled
                && risk.token_address == token_address
                && risk
                    .pool_address
                    .as_ref()
                    .map(|pool| pool == pool_address)
                    .unwrap_or(true)
        })
    }
}

fn sell_amount_from_position(position: &Position, sell_fraction: DecimalAmount) -> Option<Amount> {
    let raw_amount = position.entry_token_raw_amount.clone()?;
    if sell_fraction == DecimalAmount::from(1) {
        return Some(raw_amount);
    }
    let scaled_amount = raw_amount.to_decimal() * sell_fraction;
    Some(Amount::from_decimal(scaled_amount, raw_amount.decimals))
}

impl Strategy for SnipeAllStrategy {
    fn name(&self) -> StrategyName {
        self.config.strategy_name.clone()
    }

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        let MarketEvent::PoolUpdated {
            pool, block_number, ..
        } = event
        else {
            return Ok(StrategyDecision::hold("market_event_not_pool_update"));
        };

        let strategy_name = self.name();

        // Check if we have an active position for this pool.
        if let Some(position) = ctx.portfolio.positions.values().find(|p| {
            p.key.strategy_name == strategy_name
                && p.key.token_address == pool.token_address
                && p.key.pool_address == pool.address
                && p.has_exposure()
        }) {
            self.state.mark_bought(pool.address.clone());

            if position.state == PositionState::BuyConfirmed {
                // Evaluate proactive price-ratio / time-based exits.
                if let Some(decision) =
                    self.evaluate_proactive_exit(ctx, position, pool, *block_number)
                {
                    return Ok(decision);
                }

                return Ok(StrategyDecision::hold("position_open_no_exit"));
            }

            return Ok(StrategyDecision::hold(
                "position_exit_waiting_for_retry_policy",
            ));
        }

        // 1. Shared eligibility gate: reject ineligible pools first.
        match shared_rules::entry::eligibility::evaluate(pool, &self.config.classification_config())
        {
            RuleDecision::Hold { rule, reason } => {
                return Ok(StrategyDecision::hold(format!("{rule}:{reason}")));
            }
            _ => {}
        }

        if Self::has_blocking_entry_risk(ctx, pool.token_address, &pool.address) {
            return Ok(StrategyDecision::hold("entry.blocked_by_active_risk"));
        }

        Ok(match entry::evaluate(&self.state, pool) {
            RuleDecision::Enter { rule } => self.buy_pool(pool, rule),
            RuleDecision::Hold { rule, reason } => {
                StrategyDecision::hold(format!("{rule}:{reason}"))
            }
            RuleDecision::Exit { rule } => StrategyDecision::hold(format!("{rule}:exit_ignored")),
        })
    }

    fn on_risk_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &RiskEvent,
    ) -> Result<StrategyDecision> {
        let _creator_label = creator_label::evaluate(&self.config, ctx, event);
        let strategy_name = self.name();

        // Shared exit rules: any strategy with an open position should exit on
        // these signals. The strategy config controls which ones are enabled.
        // Each rule is evaluated independently so we can quantify per-rule effect.
        if self.config.exit_on_liquidity_removal {
            if let RuleDecision::Exit { .. } =
                shared_rules::exit::liquidity_removal::evaluate(ctx, &strategy_name, event)
            {
                if let Some(pool_address) = event
                    .pool_address
                    .clone()
                    .or_else(|| ctx.market.pool_address.clone())
                {
                    return Ok(self.sell_pool(
                        ctx,
                        event.token_address,
                        pool_address,
                        "exit.liquidity_removal",
                    ));
                }
            }
        }

        if self.config.exit_on_tax {
            if let RuleDecision::Exit { .. } =
                shared_rules::exit::tax::evaluate(ctx, &strategy_name, event)
            {
                if let Some(pool_address) = event
                    .pool_address
                    .clone()
                    .or_else(|| ctx.market.pool_address.clone())
                {
                    return Ok(self.sell_pool(ctx, event.token_address, pool_address, "exit.tax"));
                }
            }
        }

        if self.config.exit_on_lp_approval {
            if self.config.exit_on_critical_lp_approval_only
                && event.kind == RiskKind::LpApproval
                && event.severity != RiskSeverity::Critical
            {
                return Ok(StrategyDecision::hold("exit.lp_approval_not_critical"));
            }
            if let RuleDecision::Exit { .. } =
                shared_rules::exit::lp_approval::evaluate(ctx, &strategy_name, event)
            {
                if let Some(pool_address) = event
                    .pool_address
                    .clone()
                    .or_else(|| ctx.market.pool_address.clone())
                {
                    return Ok(self.sell_pool(
                        ctx,
                        event.token_address,
                        pool_address,
                        "exit.lp_approval",
                    ));
                }
            }
        }

        if self.config.exit_on_scam {
            if let RuleDecision::Exit { .. } =
                shared_rules::exit::scam::evaluate(ctx, &strategy_name, event)
            {
                if let Some(pool_address) = event
                    .pool_address
                    .clone()
                    .or_else(|| ctx.market.pool_address.clone())
                {
                    return Ok(self.sell_pool(ctx, event.token_address, pool_address, "exit.scam"));
                }
            }
        }

        Ok(StrategyDecision::hold("risk.no_exit_rule_matched"))
    }

    fn on_position_monitor(
        &mut self,
        ctx: &StrategyContext<'_>,
        block_number: u64,
    ) -> Result<Vec<StrategyDecision>> {
        if self.config.max_hold_blocks.is_none() && self.config.exit_retry_interval_blocks.is_none()
        {
            return Ok(Vec::new());
        }
        let strategy_name = self.name();
        let mut positions = ctx
            .portfolio
            .positions
            .values()
            .filter(|position| {
                if position.key.strategy_name != strategy_name {
                    return false;
                }

                match position.state {
                    PositionState::BuyConfirmed => {
                        let Some(max_hold) = self.config.max_hold_blocks else {
                            return false;
                        };
                        position.can_submit_exit()
                            && position
                                .entry_block
                                .map(|entry_block| {
                                    block_number >= entry_block.saturating_add(max_hold)
                                })
                                .unwrap_or(false)
                    }
                    PositionState::SellFailed => {
                        self.should_retry_failed_exit(position, block_number)
                    }
                    _ => false,
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| left.id.0.cmp(&right.id.0));

        Ok(positions
            .iter()
            .map(|position| {
                let reason = if position.state == PositionState::SellFailed {
                    "exit.failed_retry"
                } else {
                    "exit.max_hold"
                };
                self.sell_position(position, reason)
            })
            .filter(|decision| !decision.is_hold())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Address, U256};
    use eth_alpha_core::{
        amount::Amount,
        execution::{ExecutionReport, ExecutionStatus},
        ids::{OrderId, PositionId, TokenPoolId},
        market::{MarketSnapshotRef, PoolProtocol},
        order::OrderSide,
        portfolio::PortfolioState,
        position::{Position, PositionKey},
    };
    use rust_decimal::Decimal;

    use super::*;

    const WETH_ADDRESS: Address =
        alloy_primitives::address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

    fn pool() -> PoolSnapshot {
        let token_address = Address::repeat_byte(0x11);
        PoolSnapshot {
            address: TokenPoolId::new(token_address, Address::repeat_byte(0x22).to_string()),
            token_address,
            protocol: PoolProtocol::UniswapV2,
            denom_address: Some(WETH_ADDRESS),
            denom_symbol: Some("WETH".to_string()),
            denom_reserve: Decimal::new(1, 0),
            token_reserve: Decimal::new(100, 0),
            price_denom_per_token: None,
            token_decimals: None,
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 1,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        }
    }

    fn ctx<'a>(
        market: &'a MarketSnapshotRef,
        portfolio: &'a PortfolioState,
        risks: &'a [RiskEvent],
    ) -> StrategyContext<'a> {
        StrategyContext {
            market,
            portfolio,
            active_risks: risks,
        }
    }

    fn confirmed_position(strategy: &SnipeAllStrategy, pool: &PoolSnapshot) -> Position {
        let mut position = Position::new(
            PositionId("position-1".to_string()),
            PositionKey {
                portfolio_id: strategy.config.portfolio_id.clone(),
                wallet_id: strategy.config.wallet_id.clone(),
                strategy_name: strategy.name(),
                token_address: pool.token_address,
                pool_address: pool.address.clone(),
            },
        );
        position.mark_intent_created(OrderSide::Buy).unwrap();
        position
            .mark_order_submitted(OrderId("buy-1".to_string()), OrderSide::Buy)
            .unwrap();
        position
            .apply_execution_report(&ExecutionReport {
                order_id: OrderId("buy-1".to_string()),
                status: ExecutionStatus::Confirmed,
                tx_hash: None,
                block_number: Some(1),
                filled_amount: Some(Amount {
                    raw: Default::default(),
                    decimals: 18,
                }),
                token_amount: Some(Amount {
                    raw: U256::from(1_000_000u64),
                    decimals: 9,
                }),
                gas_used: Some(21_000),
                gas_cost: None,
                error: None,
            })
            .unwrap();
        position
    }

    fn failed_exit_position(strategy: &SnipeAllStrategy, pool: &PoolSnapshot) -> Position {
        let mut position = confirmed_position(strategy, pool);
        position.mark_intent_created(OrderSide::Sell).unwrap();
        position
            .mark_order_submitted(OrderId("sell-1".to_string()), OrderSide::Sell)
            .unwrap();
        position
            .apply_execution_report(&ExecutionReport {
                order_id: OrderId("sell-1".to_string()),
                status: ExecutionStatus::Failed,
                tx_hash: None,
                block_number: Some(202),
                filled_amount: None,
                token_amount: None,
                gas_used: Some(21_000),
                gas_cost: None,
                error: Some("temporary sell failure".to_string()),
            })
            .unwrap();
        position
    }

    #[test]
    fn uses_configured_strategy_name() {
        let strategy = SnipeAllStrategy::new(SnipeAllConfig {
            strategy_name: StrategyName("snipe-all-maxhold20-liq-exit".to_string()),
            ..SnipeAllConfig::default()
        });

        assert_eq!(
            strategy.name(),
            StrategyName("snipe-all-maxhold20-liq-exit".to_string())
        );
    }

    #[test]
    fn buys_every_eligible_pool_once() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let portfolio = PortfolioState::default();
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 1,
                    pool: pool.clone(),
                },
            )
            .unwrap();
        match decision {
            StrategyDecision::SubmitOrder(intent)
            | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
                assert_eq!(intent.side, OrderSide::Buy);
                assert_eq!(intent.pool_address, pool.address);
            }
            StrategyDecision::Hold
            | StrategyDecision::HoldWithReason { .. }
            | StrategyDecision::CancelOrders { .. } => {
                panic!("expected buy order")
            }
        }

        let repeat = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 2,
                    pool,
                },
            )
            .unwrap();
        assert!(repeat.is_hold());
    }

    #[test]
    fn position_monitor_exits_at_max_hold_without_pool_update() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 201,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut portfolio = PortfolioState::default();
        let risks = Vec::new();
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            max_hold_blocks: Some(200),
            ..SnipeAllConfig::default()
        });
        let position = confirmed_position(&strategy, &pool);
        portfolio.positions.insert(position.id.clone(), position);
        let ctx = ctx(&market, &portfolio, &risks);

        let decisions = strategy.on_position_monitor(&ctx, 201).unwrap();

        assert_eq!(decisions.len(), 1);
        match &decisions[0] {
            StrategyDecision::SubmitOrder(intent)
            | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
                assert_eq!(intent.side, OrderSide::Sell);
                assert_eq!(intent.pool_address, pool.address);
            }
            StrategyDecision::Hold
            | StrategyDecision::HoldWithReason { .. }
            | StrategyDecision::CancelOrders { .. } => {
                panic!("expected sell order")
            }
        }
    }

    #[test]
    fn position_monitor_holds_before_max_hold_boundary() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 200,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut portfolio = PortfolioState::default();
        let risks = Vec::new();
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            max_hold_blocks: Some(200),
            ..SnipeAllConfig::default()
        });
        let position = confirmed_position(&strategy, &pool);
        portfolio.positions.insert(position.id.clone(), position);
        let ctx = ctx(&market, &portfolio, &risks);

        let decisions = strategy.on_position_monitor(&ctx, 200).unwrap();

        assert!(decisions.is_empty());
    }

    #[test]
    fn position_monitor_does_not_retry_failed_exit_every_block() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 250,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut portfolio = PortfolioState::default();
        let risks = Vec::new();
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            max_hold_blocks: Some(200),
            ..SnipeAllConfig::default()
        });
        let mut position = confirmed_position(&strategy, &pool);
        position.mark_intent_created(OrderSide::Sell).unwrap();
        position
            .mark_order_submitted(OrderId("sell-1".to_string()), OrderSide::Sell)
            .unwrap();
        position
            .apply_execution_report(&ExecutionReport {
                order_id: OrderId("sell-1".to_string()),
                status: ExecutionStatus::Failed,
                tx_hash: None,
                block_number: Some(202),
                filled_amount: None,
                token_amount: None,
                gas_used: Some(21_000),
                gas_cost: None,
                error: Some("temporary sell failure".to_string()),
            })
            .unwrap();
        assert!(position.can_submit_exit());
        portfolio.positions.insert(position.id.clone(), position);
        let ctx = ctx(&market, &portfolio, &risks);

        let decisions = strategy.on_position_monitor(&ctx, 250).unwrap();

        assert!(decisions.is_empty());
    }

    #[test]
    fn market_event_does_not_retry_failed_exit_without_retry_policy() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 250,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut portfolio = PortfolioState::default();
        let risks = Vec::new();
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            max_hold_blocks: Some(200),
            ..SnipeAllConfig::default()
        });
        let position = failed_exit_position(&strategy, &pool);
        portfolio.positions.insert(position.id.clone(), position);
        let ctx = ctx(&market, &portfolio, &risks);

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 250,
                    pool,
                },
            )
            .unwrap();

        assert!(decision.is_hold());
        assert_eq!(
            decision.reason(),
            Some("position_exit_waiting_for_retry_policy")
        );
    }

    #[test]
    fn position_monitor_retries_failed_exit_after_configured_interval() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 227,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut portfolio = PortfolioState::default();
        let risks = Vec::new();
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            exit_retry_interval_blocks: Some(25),
            max_exit_retries: Some(3),
            ..SnipeAllConfig::default()
        });
        let position = failed_exit_position(&strategy, &pool);
        portfolio.positions.insert(position.id.clone(), position);
        let ctx = ctx(&market, &portfolio, &risks);

        let decisions = strategy.on_position_monitor(&ctx, 227).unwrap();

        assert_eq!(decisions.len(), 1);
        match &decisions[0] {
            StrategyDecision::SubmitOrder(intent)
            | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
                assert_eq!(intent.side, OrderSide::Sell);
                assert_eq!(intent.pool_address, pool.address);
            }
            StrategyDecision::Hold
            | StrategyDecision::HoldWithReason { .. }
            | StrategyDecision::CancelOrders { .. } => {
                panic!("expected retry sell order")
            }
        }
    }

    #[test]
    fn position_monitor_waits_for_failed_exit_retry_interval() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 226,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut portfolio = PortfolioState::default();
        let risks = Vec::new();
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            exit_retry_interval_blocks: Some(25),
            max_exit_retries: Some(3),
            ..SnipeAllConfig::default()
        });
        let position = failed_exit_position(&strategy, &pool);
        portfolio.positions.insert(position.id.clone(), position);
        let ctx = ctx(&market, &portfolio, &risks);

        let decisions = strategy.on_position_monitor(&ctx, 226).unwrap();

        assert!(decisions.is_empty());
    }

    #[test]
    fn position_monitor_respects_max_failed_exit_retries() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 300,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut portfolio = PortfolioState::default();
        let risks = Vec::new();
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig {
            exit_retry_interval_blocks: Some(25),
            max_exit_retries: Some(1),
            ..SnipeAllConfig::default()
        });
        let position = failed_exit_position(&strategy, &pool);
        portfolio.positions.insert(position.id.clone(), position);
        let ctx = ctx(&market, &portfolio, &risks);

        let decisions = strategy.on_position_monitor(&ctx, 300).unwrap();

        assert!(decisions.is_empty());
    }

    #[test]
    fn buys_usd_stable_pool_at_stable_liquidity_floor() {
        let mut pool = pool();
        pool.denom_symbol = Some("USDC".to_string());
        pool.denom_reserve = Decimal::from(1_000u64);
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let portfolio = PortfolioState::default();
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 1,
                    pool,
                },
            )
            .unwrap();

        assert!(decision.order_intent().is_some());
    }

    #[test]
    fn holds_usd_stable_pool_below_stable_liquidity_floor() {
        let mut pool = pool();
        pool.denom_symbol = Some("USDT".to_string());
        pool.denom_reserve = Decimal::from(999u64);
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let portfolio = PortfolioState::default();
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 1,
                    pool,
                },
            )
            .unwrap();

        assert!(decision.is_hold());
    }

    #[test]
    fn buys_dai_pool_at_stable_liquidity_floor() {
        let mut pool = pool();
        pool.denom_symbol = Some("DAI".to_string());
        pool.denom_reserve = Decimal::from(1_000u64);
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let portfolio = PortfolioState::default();
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 1,
                    pool,
                },
            )
            .unwrap();

        assert!(decision.order_intent().is_some());
    }

    #[test]
    fn holds_weth_pool_below_eth_liquidity_floor() {
        let mut pool = pool();
        pool.denom_reserve = Decimal::new(49, 2);
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let portfolio = PortfolioState::default();
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 1,
                    pool,
                },
            )
            .unwrap();

        assert!(decision.is_hold());
    }

    #[test]
    fn restored_open_position_prevents_duplicate_buy() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
        let position = confirmed_position(&strategy, &pool);
        let mut portfolio = PortfolioState::default();
        portfolio.positions.insert(position.id.clone(), position);
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 2,
                    pool,
                },
            )
            .unwrap();
        assert!(decision.is_hold());
    }

    #[test]
    fn restored_seen_pool_prevents_duplicate_buy_after_closed_position() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let portfolio = PortfolioState::default();
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let mut strategy = SnipeAllStrategy::with_bought_pools(
            SnipeAllConfig::default(),
            vec![pool.address.clone()],
        );

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 2,
                    pool,
                },
            )
            .unwrap();

        assert!(decision.is_hold());
        assert_eq!(
            decision.reason(),
            Some("entry.buy_eligible_pool_once:pool already bought")
        );
    }

    #[test]
    fn sells_restored_open_position_on_liquidity_removal() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
        let position = confirmed_position(&strategy, &pool);
        let mut portfolio = PortfolioState::default();
        portfolio.positions.insert(position.id.clone(), position);
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let risk = RiskEvent {
            kind: RiskKind::LiquidityRemoval,
            severity: RiskSeverity::Critical,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            pending_tx_hash: None,
            observed_block: Some(2),
            message: "liquidity removal".to_string(),
        };

        let decision = strategy.on_risk_event(&ctx, &risk).unwrap();
        match decision {
            StrategyDecision::SubmitOrder(intent)
            | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
                assert_eq!(intent.side, OrderSide::Sell);
                assert_eq!(intent.pool_address, pool.address);
            }
            StrategyDecision::Hold
            | StrategyDecision::HoldWithReason { .. }
            | StrategyDecision::CancelOrders { .. } => {
                panic!("expected sell order")
            }
        }
    }

    #[test]
    fn sells_open_position_on_lp_approval() {
        let pool = pool();
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let mut strategy = SnipeAllStrategy::new(SnipeAllConfig::default());
        let position = confirmed_position(&strategy, &pool);
        let mut portfolio = PortfolioState::default();
        portfolio.positions.insert(position.id.clone(), position);
        let risks = Vec::new();
        let ctx = ctx(&market, &portfolio, &risks);
        let risk = RiskEvent {
            kind: RiskKind::LpApproval,
            severity: RiskSeverity::Warning,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            pending_tx_hash: None,
            observed_block: Some(2),
            message: "lp approval".to_string(),
        };

        let decision = strategy.on_risk_event(&ctx, &risk).unwrap();
        match decision {
            StrategyDecision::SubmitOrder(intent)
            | StrategyDecision::SubmitOrderWithReason { intent, .. } => {
                assert_eq!(intent.side, OrderSide::Sell);
                assert_eq!(intent.pool_address, pool.address);
            }
            StrategyDecision::Hold
            | StrategyDecision::HoldWithReason { .. }
            | StrategyDecision::CancelOrders { .. } => {
                panic!("expected sell order")
            }
        }
    }
}
