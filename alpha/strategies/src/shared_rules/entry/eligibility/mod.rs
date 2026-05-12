use eth_alpha_core::market::{PoolProtocol, PoolSnapshot};
use eth_pool_classification::{
    PoolClassificationConfig, PoolClassificationInput, classify_pool_with_config,
};
use rust_decimal::prelude::ToPrimitive;

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = "entry.eligibility";

/// Evaluate whether a pool is eligible for entry.
///
/// This is the first gate any strategy should apply in `on_market_event`.
/// Pools that fail classification (insufficient liquidity, unsupported denom,
/// scam, cannot buy/sell, etc.) are rejected before strategy-specific logic
/// runs.
///
/// Returns `RuleDecision::Enter` when the pool is eligible and tradable now,
/// otherwise returns `RuleDecision::Hold` with the classification reason.
pub fn evaluate(pool: &PoolSnapshot, config: &PoolClassificationConfig) -> RuleDecision {
    if let Some(reason) = unsupported_execution_route_reason(pool) {
        return RuleDecision::hold(RULE_NAME, reason);
    }

    let input = PoolClassificationInput::new(
        pool.denom_symbol.clone(),
        pool.denom_reserve.to_f64(),
        pool.can_buy,
        pool.can_sell,
        pool.is_scam,
    );

    let decision = classify_pool_with_config(&input, config);

    if !decision.eligible {
        let reason = decision
            .reason_key
            .unwrap_or_else(|| decision.category.key());
        return RuleDecision::hold(RULE_NAME, reason);
    }

    if !decision.tradable_now {
        let reason = decision
            .eligible_outcome
            .map(|o| o.key())
            .unwrap_or_else(|| decision.category.key());
        return RuleDecision::hold(RULE_NAME, reason);
    }

    RuleDecision::Enter { rule: RULE_NAME }
}

fn unsupported_execution_route_reason(pool: &PoolSnapshot) -> Option<&'static str> {
    if pool.protocol == PoolProtocol::UniswapV4 {
        let Some(v4) = pool.uniswap_v4.as_ref() else {
            return Some("unsupported_v4_missing_pool_key");
        };
        if !v4.hooks.is_zero() {
            return Some("unsupported_v4_hooks");
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{B256, address};
    use eth_alpha_core::{
        ids::TokenPoolId,
        market::{PoolSnapshot, UniswapV4PoolKeySnapshot},
    };
    use rust_decimal::Decimal;

    fn eligible_pool() -> PoolSnapshot {
        let token = address!("1000000000000000000000000000000000000001");
        PoolSnapshot {
            address: TokenPoolId::new(token, "0x2000000000000000000000000000000000000002"),
            token_address: token,
            protocol: PoolProtocol::UniswapV2,
            denom_address: Some(address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")),
            denom_symbol: Some("WETH".to_string()),
            denom_reserve: Decimal::new(1, 0),
            token_reserve: Decimal::new(100, 0),
            price_denom_per_token: None,
            token_decimals: Some(18),
            fee_tier: None,
            uniswap_v4: None,
            latest_block: 1,
            can_buy: true,
            can_sell: true,
            is_scam: false,
        }
    }

    fn v4_key(hooks: alloy_primitives::Address) -> UniswapV4PoolKeySnapshot {
        UniswapV4PoolKeySnapshot {
            pool_manager: address!("000000000004444c5dc75cb358380d2e3de08a90"),
            pool_id: B256::ZERO,
            currency0: address!("0000000000000000000000000000000000000000"),
            currency1: address!("1000000000000000000000000000000000000001"),
            fee: 10_000,
            tick_spacing: 200,
            hooks,
        }
    }

    #[test]
    fn rejects_hooked_v4_pools_for_entry() {
        let mut pool = eligible_pool();
        pool.protocol = PoolProtocol::UniswapV4;
        pool.uniswap_v4 = Some(v4_key(address!("298a86cc43af878cb78ca20e80ab0de0a59a0444")));

        let decision = evaluate(&pool, &PoolClassificationConfig::default());

        assert_eq!(
            decision,
            RuleDecision::hold(RULE_NAME, "unsupported_v4_hooks")
        );
    }

    #[test]
    fn rejects_v4_pools_without_pool_key_metadata() {
        let mut pool = eligible_pool();
        pool.protocol = PoolProtocol::UniswapV4;
        pool.uniswap_v4 = None;

        let decision = evaluate(&pool, &PoolClassificationConfig::default());

        assert_eq!(
            decision,
            RuleDecision::hold(RULE_NAME, "unsupported_v4_missing_pool_key")
        );
    }

    #[test]
    fn allows_plain_v4_pools_to_continue_to_classification() {
        let mut pool = eligible_pool();
        pool.protocol = PoolProtocol::UniswapV4;
        pool.uniswap_v4 = Some(v4_key(alloy_primitives::Address::ZERO));

        let decision = evaluate(&pool, &PoolClassificationConfig::default());

        assert_eq!(decision, RuleDecision::Enter { rule: RULE_NAME });
    }
}
