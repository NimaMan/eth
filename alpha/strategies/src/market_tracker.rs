use std::collections::HashSet;

use alloy_primitives::U256;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    ids::{PoolAddress, PortfolioId, StrategyName, TokenAddress, WalletId},
    market::{MarketEvent, PoolSnapshot},
    order::{OrderIntent, OrderSide},
    risk::{RiskEvent, RiskKind, RiskSeverity},
    AlphaCoreError, Result, Strategy, StrategyContext, StrategyDecision,
};
use eth_pool_classification::PoolClassificationConfig;
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(Clone, Debug)]
pub struct MarketTrackerConfig {
    pub portfolio_id: PortfolioId,
    pub wallet_id: WalletId,
    pub buy_amount: Amount,
    pub min_denom_reserve: DecimalAmount,
    pub min_stable_denom_reserve: DecimalAmount,
    pub supported_denom_symbols: Vec<String>,
    pub max_slippage_bps: u32,
    pub deadline_secs: u64,
}

impl Default for MarketTrackerConfig {
    fn default() -> Self {
        Self {
            portfolio_id: PortfolioId("chain-sim".to_string()),
            wallet_id: WalletId("chain-sim-wallet".to_string()),
            buy_amount: Amount {
                raw: U256::from(10_000_000_000_000_000u64),
                decimals: 18,
            },
            min_denom_reserve: Decimal::ZERO,
            min_stable_denom_reserve: Decimal::from(1_000u64),
            supported_denom_symbols: ["ETH", "WETH", "USDC", "USDT", "DAI"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            max_slippage_bps: 500,
            deadline_secs: 30,
        }
    }
}

impl MarketTrackerConfig {
    pub fn classification_config(&self) -> PoolClassificationConfig {
        PoolClassificationConfig {
            supported_quote_symbols: self.supported_denom_symbols.clone(),
            min_eth_liquidity: self.min_denom_reserve.to_f64().unwrap_or(0.0),
            min_stable_liquidity: self.min_stable_denom_reserve.to_f64().unwrap_or(0.0),
            ..PoolClassificationConfig::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarketTrackerStrategy {
    config: MarketTrackerConfig,
    submitted_pools: HashSet<PoolAddress>,
}

impl MarketTrackerStrategy {
    pub fn new(config: MarketTrackerConfig) -> Self {
        Self {
            config,
            submitted_pools: HashSet::new(),
        }
    }

    pub fn submitted_pools(&self) -> &HashSet<PoolAddress> {
        &self.submitted_pools
    }

    fn decision_for_pool(&mut self, pool: &PoolSnapshot) -> Result<StrategyDecision> {
        if self.submitted_pools.contains(&pool.address) {
            return Ok(StrategyDecision::Hold);
        }

        // Shared eligibility gate: reject ineligible pools first.
        use crate::baseline::snipe_all::rule::RuleDecision;
        use crate::shared_rules;
        match shared_rules::entry::eligibility::evaluate(pool, &self.config.classification_config())
        {
            RuleDecision::Hold { .. } => return Ok(StrategyDecision::Hold),
            _ => {}
        }

        self.submit_buy(pool.token_address, pool.address.clone())
    }

    fn submit_buy(
        &mut self,
        token_address: TokenAddress,
        pool_address: PoolAddress,
    ) -> Result<StrategyDecision> {
        self.submitted_pools.insert(pool_address.clone());
        Ok(StrategyDecision::SubmitOrder(OrderIntent {
            trade_id: None,
            portfolio_id: self.config.portfolio_id.clone(),
            wallet_id: self.config.wallet_id.clone(),
            strategy_name: self.name(),
            side: OrderSide::Buy,
            token_address,
            pool_address,
            amount: self.config.buy_amount.clone(),
            route: None,
            max_slippage_bps: self.config.max_slippage_bps,
            deadline_secs: self.config.deadline_secs,
        }))
    }

    fn has_blocking_risk(
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

impl Strategy for MarketTrackerStrategy {
    fn name(&self) -> StrategyName {
        StrategyName("market-tracker".to_string())
    }

    fn on_market_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &MarketEvent,
    ) -> Result<StrategyDecision> {
        let MarketEvent::PoolUpdated { pool, .. } = event else {
            return Ok(StrategyDecision::Hold);
        };
        if Self::has_blocking_risk(ctx, pool.token_address, &pool.address) {
            return Ok(StrategyDecision::Hold);
        }
        self.decision_for_pool(pool)
    }

    fn on_risk_event(
        &mut self,
        ctx: &StrategyContext<'_>,
        event: &RiskEvent,
    ) -> Result<StrategyDecision> {
        if event.kind != RiskKind::TradingEnabled {
            return Ok(StrategyDecision::Hold);
        }
        let Some(pool_address) = event.pool_address.clone() else {
            return Ok(StrategyDecision::Hold);
        };
        if self.submitted_pools.contains(&pool_address) {
            return Ok(StrategyDecision::Hold);
        }
        if Self::has_blocking_risk(ctx, event.token_address, &pool_address) {
            return Ok(StrategyDecision::Hold);
        }
        if ctx.market.token_address != event.token_address {
            return Err(AlphaCoreError::Strategy(
                "risk event market context token mismatch".to_string(),
            ));
        }
        self.submit_buy(event.token_address, pool_address)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use eth_alpha_core::{
        ids::TokenPoolId,
        market::{MarketSnapshotRef, PoolProtocol},
        portfolio::PortfolioState,
        risk::RiskSeverity,
    };

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
            initial_price_denom_per_token: None,
            price_ratio_to_initial: None,
            token_decimals: None,
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 1,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        }
    }

    #[test]
    fn buys_once_when_pool_is_tradable() {
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
        let ctx = StrategyContext {
            market: &market,
            portfolio: &portfolio,
            active_risks: &risks,
        };
        let mut strategy = MarketTrackerStrategy::new(MarketTrackerConfig::default());

        let decision = strategy
            .on_market_event(
                &ctx,
                &MarketEvent::PoolUpdated {
                    block_number: 1,
                    pool: pool.clone(),
                },
            )
            .unwrap();
        assert!(matches!(decision, StrategyDecision::SubmitOrder(_)));

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
    fn critical_risk_blocks_market_buy() {
        let pool = pool();
        let risk = RiskEvent {
            kind: RiskKind::LiquidityRemoval,
            severity: RiskSeverity::Critical,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            pending_tx_hash: None,
            observed_block: Some(1),
            message: "liquidity removal".to_string(),
        };
        let market = MarketSnapshotRef {
            block_number: 1,
            token_address: pool.token_address,
            pool_address: Some(pool.address.clone()),
            token: None,
            pool: Some(pool.clone()),
        };
        let portfolio = PortfolioState::default();
        let risks = vec![risk];
        let ctx = StrategyContext {
            market: &market,
            portfolio: &portfolio,
            active_risks: &risks,
        };
        let mut strategy = MarketTrackerStrategy::new(MarketTrackerConfig::default());

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
}
