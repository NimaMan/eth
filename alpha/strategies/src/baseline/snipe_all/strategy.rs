use std::collections::HashSet;

use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{BlockNumber, PoolAddress, PositionId, StrategyName, TokenAddress},
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

    pub fn with_restored_state(
        config: SnipeAllConfig,
        bought_pools: impl IntoIterator<Item = PoolAddress>,
        active_hold_blocks: impl IntoIterator<Item = (PositionId, u64, Option<BlockNumber>)>,
    ) -> Self {
        Self {
            config,
            state: SnipeAllState::with_bought_pools_and_active_hold_blocks(
                bought_pools,
                active_hold_blocks,
            ),
        }
    }

    pub fn state(&self) -> &SnipeAllState {
        &self.state
    }

    fn buy_pool(&mut self, pool: &PoolSnapshot, reason: impl Into<String>) -> StrategyDecision {
        self.state.mark_bought(pool.address.clone());
        StrategyDecision::submit_order(
            OrderIntent {
                trade_id: None,
                portfolio_id: self.config.portfolio_id.clone(),
                wallet_id: self.config.wallet_id.clone(),
                strategy_name: self.name(),
                side: OrderSide::Buy,
                token_address: pool.token_address,
                pool_address: pool.address.clone(),
                protocol: pool.protocol.clone(),
                amount: self.config.buy_amount.clone(),
                route: None,
                max_slippage_bps: self.config.max_slippage_bps,
                deadline_secs: self.config.deadline_secs,
                decision_reason: None,
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
        let reason = reason.into();

        // Look up the open position to determine how many tokens to sell.
        let position = ctx.portfolio.positions.values().find(|p| {
            p.key.strategy_name == self.name()
                && p.key.token_address == token_address
                && p.key.pool_address == pool_address
                && p.can_submit_exit()
        });

        let Some(position) = position else {
            return StrategyDecision::hold("exit.no_sellable_position");
        };
        let Some(token_amount) = sell_amount_from_position(position, self.config.sell_fraction)
        else {
            return StrategyDecision::hold("exit.no_sellable_position");
        };
        if let Some(pool) = current_pool_snapshot(ctx, &pool_address) {
            if let Some(skip_reason) = shared_rules::exit::sell_safety::dust_pool_exit_reason(
                pool,
                self.config.min_sell_pool_denom_reserve,
                &reason,
            ) {
                return StrategyDecision::hold(skip_reason);
            }
        }

        StrategyDecision::submit_order(
            OrderIntent {
                trade_id: None,
                portfolio_id: self.config.portfolio_id.clone(),
                wallet_id: self.config.wallet_id.clone(),
                strategy_name: self.name(),
                side: OrderSide::Sell,
                token_address,
                pool_address,
                protocol: position.key.protocol.clone(),
                amount: token_amount,
                route: None,
                max_slippage_bps: self.config.max_slippage_bps,
                deadline_secs: self.config.deadline_secs,
                decision_reason: None,
            },
            reason,
        )
    }

    fn sell_position(
        &self,
        ctx: &StrategyContext<'_>,
        position: &Position,
        reason: impl Into<String>,
    ) -> StrategyDecision {
        let reason = reason.into();
        if let Some(pool) = current_pool_snapshot(ctx, &position.key.pool_address) {
            if let Some(skip_reason) = shared_rules::exit::sell_safety::dust_pool_exit_reason(
                pool,
                self.config.min_sell_pool_denom_reserve,
                &reason,
            ) {
                return StrategyDecision::hold(skip_reason);
            }
        }

        let Some(token_amount) = sell_amount_from_position(position, self.config.sell_fraction)
        else {
            return StrategyDecision::hold("exit.no_token_amount");
        };

        StrategyDecision::submit_order(
            OrderIntent {
                trade_id: None,
                portfolio_id: self.config.portfolio_id.clone(),
                wallet_id: self.config.wallet_id.clone(),
                strategy_name: self.name(),
                side: OrderSide::Sell,
                token_address: position.key.token_address,
                pool_address: position.key.pool_address.clone(),
                protocol: position.key.protocol.clone(),
                amount: token_amount,
                route: None,
                max_slippage_bps: self.config.max_slippage_bps,
                deadline_secs: self.config.deadline_secs,
                decision_reason: None,
            },
            reason,
        )
    }

    /// Evaluate proactive price-ratio and active-hold exits for an open position.
    /// Returns Some(decision) if an exit should be triggered, None otherwise.
    fn evaluate_proactive_exit(
        &mut self,
        ctx: &StrategyContext<'_>,
        position: &eth_alpha_core::position::Position,
        pool: &PoolSnapshot,
        active_hold_blocks: u64,
    ) -> Option<StrategyDecision> {
        if !position.can_submit_exit() {
            return None;
        }

        // Active-hold exit: sell once the position has seen the configured
        // number of distinct pool-update blocks while open.
        if let Some(max_hold) = self.config.max_hold_blocks {
            if active_hold_blocks >= max_hold {
                return Some(self.sell_pool(
                    ctx,
                    pool.token_address,
                    pool.address.clone(),
                    "exit.max_hold_active_blocks",
                ));
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

    fn has_matching_buy_confirm_block_position(
        &self,
        ctx: &StrategyContext<'_>,
        event: &RiskEvent,
    ) -> bool {
        let Some(observed_block) = event.observed_block else {
            return false;
        };
        let Some(pool_address) = event
            .pool_address
            .as_ref()
            .or(ctx.market.pool_address.as_ref())
        else {
            return false;
        };
        let strategy_name = self.name();
        ctx.portfolio.positions.values().any(|position| {
            position.key.strategy_name == strategy_name
                && position.key.token_address == event.token_address
                && &position.key.pool_address == pool_address
                && position.entry_block == Some(observed_block)
                && position.can_submit_exit()
        })
    }

    fn entry_bankroll_available_wei(&self, ctx: &StrategyContext<'_>) -> Option<U256> {
        let mut available = self.config.entry_bankroll_wei?;
        let strategy_name = self.name();
        let mut portfolio_pools = HashSet::new();

        for position in ctx
            .portfolio
            .positions
            .values()
            .filter(|position| position.key.strategy_name == strategy_name)
        {
            portfolio_pools.insert(position.key.pool_address.clone());
            available = apply_position_to_entry_bankroll(available, position, &self.config);
        }

        for pool in self.state.bought_pools() {
            if !portfolio_pools.contains(pool) {
                available = available.saturating_sub(self.config.buy_amount.raw);
            }
        }

        Some(available)
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

fn position_entry_spend_wei(position: &Position, config: &SnipeAllConfig) -> U256 {
    position
        .entry_cost_basis
        .map(|cost| Amount::from_decimal(cost, 18).raw)
        .unwrap_or(config.buy_amount.raw)
}

fn position_exit_proceeds_wei(position: &Position) -> U256 {
    position
        .exit_proceeds
        .map(|proceeds| Amount::from_decimal(proceeds, 18).raw)
        .unwrap_or(U256::ZERO)
}

fn apply_position_to_entry_bankroll(
    available: U256,
    position: &Position,
    config: &SnipeAllConfig,
) -> U256 {
    match position.state {
        PositionState::Init
        | PositionState::BuyFailed
        | PositionState::BuyCancelled
        | PositionState::Cancelled => available,
        PositionState::BuyIntentCreated | PositionState::BuySubmitted => {
            available.saturating_sub(config.buy_amount.raw)
        }
        PositionState::BuyConfirmed
        | PositionState::SellIntentCreated
        | PositionState::SellSubmitted
        | PositionState::SellFailed
        | PositionState::SellCancelled
        | PositionState::Scammed => {
            available.saturating_sub(position_entry_spend_wei(position, config))
        }
        PositionState::SellConfirmed => available
            .saturating_sub(position_entry_spend_wei(position, config))
            .saturating_add(position_exit_proceeds_wei(position)),
    }
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
                let active_hold_blocks = self
                    .state
                    .observe_active_hold_block(&position.id, *block_number);

                // Evaluate proactive price-ratio / active-hold exits.
                if let Some(decision) =
                    self.evaluate_proactive_exit(ctx, position, pool, active_hold_blocks)
                {
                    return Ok(decision);
                }

                return Ok(StrategyDecision::hold("position_open_no_exit"));
            }

            let reason = if position.state == PositionState::SellFailed {
                "position_exit_failed_no_strategy_retry"
            } else {
                "position_exit_pending"
            };
            return Ok(StrategyDecision::hold(reason));
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
        if !self.config.entry_enabled {
            return Ok(StrategyDecision::hold("entry.disabled"));
        }
        if self
            .config
            .max_entry_pools
            .map(|limit| self.state.bought_pool_count() >= limit)
            .unwrap_or(false)
        {
            return Ok(StrategyDecision::hold("entry.max_entry_pools_reached"));
        }
        if let Some(available) = self.entry_bankroll_available_wei(ctx) {
            if available < self.config.buy_amount.raw {
                return Ok(StrategyDecision::hold("entry.bankroll_insufficient"));
            }
        }
        if self.config.block_entry_on_lp_approval {
            match shared_rules::lp_approval::entry_gate::evaluate(
                ctx,
                pool.token_address,
                &pool.address,
                self.config.lp_approval_gate_min_pct,
            ) {
                RuleDecision::Hold { rule, reason } => {
                    return Ok(StrategyDecision::hold(format!("{rule}:{reason}")));
                }
                _ => {}
            }
        }
        if !self.config.allowed_protocols.is_empty() {
            let protocol = pool.protocol.label();
            if !self
                .config
                .allowed_protocols
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(protocol.as_ref()))
            {
                return Ok(StrategyDecision::hold(format!(
                    "entry.protocol_not_allowed:{protocol}"
                )));
            }
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
            if let RuleDecision::Exit { rule } =
                shared_rules::exit::liquidity_removal::evaluate(ctx, &strategy_name, event)
            {
                if let Some(pool_address) = event
                    .pool_address
                    .clone()
                    .or_else(|| ctx.market.pool_address.clone())
                {
                    return Ok(self.sell_pool(ctx, event.token_address, pool_address, rule));
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
            if self.config.defer_buy_confirm_block_lp_approval_to_max_hold
                && event.kind == RiskKind::LpApproval
                && self.has_matching_buy_confirm_block_position(ctx, event)
            {
                return Ok(StrategyDecision::hold(
                    "exit.lp_approval_buy_confirm_block_deferred_to_max_hold",
                ));
            }
            match shared_rules::lp_approval::exit_gate::evaluate(
                ctx,
                &strategy_name,
                event,
                self.config.lp_approval_gate_min_pct,
            ) {
                RuleDecision::Exit { .. } => {
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
                RuleDecision::Hold { rule, reason } if event.kind == RiskKind::LpApproval => {
                    return Ok(StrategyDecision::hold(format!("{rule}:{reason}")));
                }
                _ => {}
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
        _block_number: u64,
    ) -> Result<Vec<StrategyDecision>> {
        if self.config.max_hold_blocks.is_none() {
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
                    PositionState::BuyConfirmed => self
                        .config
                        .max_hold_blocks
                        .map(|max_hold| {
                            self.state.active_hold_block_count(&position.id) >= max_hold
                        })
                        .unwrap_or(false),
                    _ => false,
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        positions.sort_by(|left, right| left.id.0.cmp(&right.id.0));

        Ok(positions
            .iter()
            .map(|position| {
                self.sell_position(ctx, position, "exit.max_hold_active_blocks_restored")
            })
            .filter(|decision| !decision.is_hold())
            .collect())
    }
}

fn current_pool_snapshot<'a>(
    ctx: &'a StrategyContext<'_>,
    pool_address: &PoolAddress,
) -> Option<&'a PoolSnapshot> {
    ctx.market
        .pool
        .as_ref()
        .filter(|pool| &pool.address == pool_address)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
