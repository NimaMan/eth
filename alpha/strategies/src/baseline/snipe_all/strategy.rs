use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PoolAddress, StrategyName, TokenAddress},
    market::{MarketEvent, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    position::Position,
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

    pub fn state(&self) -> &SnipeAllState {
        &self.state
    }

    fn buy_pool(&mut self, pool: &PoolSnapshot) -> StrategyDecision {
        self.state.mark_bought(pool.address.clone());
        StrategyDecision::SubmitOrder(OrderIntent {
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
        })
    }

    fn sell_pool(
        &mut self,
        ctx: &StrategyContext<'_>,
        token_address: TokenAddress,
        pool_address: PoolAddress,
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
            return StrategyDecision::Hold;
        };

        StrategyDecision::SubmitOrder(OrderIntent {
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
        })
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

        // Time-based exit: max hold duration exceeded.
        if let Some(max_hold) = self.config.max_hold_blocks {
            if let Some(entry_block) = position.entry_block {
                if current_block > entry_block + max_hold {
                    return Some(self.sell_pool(ctx, pool.token_address, pool.address.clone()));
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
                return Some(self.sell_pool(ctx, pool.token_address, pool.address.clone()));
            }
        }

        // Take-profit: price rose above threshold ratio.
        if let Some(tp_ratio) = self.config.take_profit_ratio {
            if price_ratio >= tp_ratio {
                return Some(self.sell_pool(ctx, pool.token_address, pool.address.clone()));
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
        StrategyName("snipe-all-v1".to_string())
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
            return Ok(StrategyDecision::Hold);
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

            // Evaluate proactive price-ratio / time-based exits.
            if let Some(decision) = self.evaluate_proactive_exit(ctx, position, pool, *block_number)
            {
                return Ok(decision);
            }

            return Ok(StrategyDecision::Hold);
        }

        // 1. Shared eligibility gate: reject ineligible pools first.
        match shared_rules::entry::eligibility::evaluate(pool, &self.config.classification_config())
        {
            RuleDecision::Hold { .. } => return Ok(StrategyDecision::Hold),
            _ => {}
        }

        if Self::has_blocking_entry_risk(ctx, pool.token_address, &pool.address) {
            return Ok(StrategyDecision::Hold);
        }

        Ok(match entry::evaluate(&self.state, pool) {
            RuleDecision::Enter { .. } => self.buy_pool(pool),
            RuleDecision::Hold { .. } | RuleDecision::Exit { .. } => StrategyDecision::Hold,
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
                    return Ok(self.sell_pool(ctx, event.token_address, pool_address));
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
                    return Ok(self.sell_pool(ctx, event.token_address, pool_address));
                }
            }
        }

        if self.config.exit_on_lp_approval {
            if let RuleDecision::Exit { .. } =
                shared_rules::exit::lp_approval::evaluate(ctx, &strategy_name, event)
            {
                if let Some(pool_address) = event
                    .pool_address
                    .clone()
                    .or_else(|| ctx.market.pool_address.clone())
                {
                    return Ok(self.sell_pool(ctx, event.token_address, pool_address));
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
                    return Ok(self.sell_pool(ctx, event.token_address, pool_address));
                }
            }
        }

        Ok(StrategyDecision::Hold)
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

    fn pool() -> PoolSnapshot {
        let token_address = Address::repeat_byte(0x11);
        PoolSnapshot {
            address: TokenPoolId::new(token_address, Address::repeat_byte(0x22).to_string()),
            token_address,
            protocol: PoolProtocol::UniswapV2,
            denom_address: Some(Address::repeat_byte(0x33)),
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
                error: None,
            })
            .unwrap();
        position
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
            StrategyDecision::SubmitOrder(intent) => {
                assert_eq!(intent.side, OrderSide::Buy);
                assert_eq!(intent.pool_address, pool.address);
            }
            StrategyDecision::Hold | StrategyDecision::CancelOrders { .. } => {
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
        assert_eq!(repeat, StrategyDecision::Hold);
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

        assert!(matches!(decision, StrategyDecision::SubmitOrder(_)));
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

        assert_eq!(decision, StrategyDecision::Hold);
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

        assert!(matches!(decision, StrategyDecision::SubmitOrder(_)));
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

        assert_eq!(decision, StrategyDecision::Hold);
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
        assert_eq!(decision, StrategyDecision::Hold);
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
            StrategyDecision::SubmitOrder(intent) => {
                assert_eq!(intent.side, OrderSide::Sell);
                assert_eq!(intent.pool_address, pool.address);
            }
            StrategyDecision::Hold | StrategyDecision::CancelOrders { .. } => {
                panic!("expected sell order")
            }
        }
    }
}
