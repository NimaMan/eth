use eth_alpha_core::order::OrderIntent;
use eth_live_trading::{
    tx_prep::{MEMPOOL_RACE_GAS_LABEL, MEMPOOL_RACE_GAS_SOURCE},
    RankedFeeCandidate, StrategyGasRankPolicy,
};

use crate::live_trader::gas_policy::LiveRealGasPolicy;

pub(super) struct SellPolicyContext<'a> {
    pub(super) policy: &'a StrategyGasRankPolicy,
    pub(super) action: &'static str,
    pub(super) signal: String,
}

pub(super) fn sell_policy_context<'a>(
    intent: &OrderIntent,
    gas_policy: &'a LiveRealGasPolicy,
) -> SellPolicyContext<'a> {
    let reason = intent
        .decision_reason
        .as_ref()
        .map(|reason| reason.code.as_str())
        .unwrap_or("exit.unknown");
    let source = intent
        .decision_reason
        .as_ref()
        .and_then(|reason| reason.source.as_deref())
        .unwrap_or_default();

    if reason.contains("buy_confirm") {
        return SellPolicyContext {
            policy: &gas_policy.lp_approval_exit_gas_rank_policy,
            action: "lp_approval_exit",
            signal: reason.to_string(),
        };
    }
    if source.contains("mempool") || reason.contains("mempool") {
        return SellPolicyContext {
            policy: &gas_policy.mempool_pre_mine_gas_rank_policy,
            action: "mempool_race_exit",
            signal: reason.to_string(),
        };
    }
    if reason.contains("lp_approval") {
        return SellPolicyContext {
            policy: &gas_policy.lp_approval_exit_gas_rank_policy,
            action: "lp_approval_exit",
            signal: reason.to_string(),
        };
    }
    if reason.contains("liquidity_removal") {
        return SellPolicyContext {
            policy: &gas_policy.lp_approval_exit_gas_rank_policy,
            action: "mined_liquidity_removal_exit",
            signal: reason.to_string(),
        };
    }
    SellPolicyContext {
        policy: &gas_policy.normal_exit_gas_rank_policy,
        action: "normal_exit",
        signal: reason.to_string(),
    }
}

pub(super) fn policy_allows_mempool_race_candidate(
    policy: &StrategyGasRankPolicy,
    candidate: &RankedFeeCandidate,
) -> bool {
    candidate.label == MEMPOOL_RACE_GAS_LABEL
        && candidate.source.as_deref() == Some(MEMPOOL_RACE_GAS_SOURCE)
        && policy
            .allowed_profiles
            .iter()
            .any(|profile| profile.label() == MEMPOOL_RACE_GAS_LABEL)
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;
    use eth_alpha_core::{
        amount::Amount,
        decision_rationale::{DecisionReason, ReasonCategory},
        ids::{PoolAddress, PortfolioId, StrategyName, WalletId},
        market::PoolProtocol,
        order::{OrderIntent, OrderSide},
    };
    use eth_live_trading::StrategyGasRankPolicy;
    use rust_decimal::Decimal;
    use serde_json::json;

    use crate::live_trader::gas_policy::LiveRealGasPolicy;

    use super::sell_policy_context;

    #[test]
    fn mined_liquidity_removal_uses_explicit_action_label() {
        let intent = sell_intent("exit.liquidity_removal", Some("pool_update"));
        let gas_policy = gas_policy();

        let context = sell_policy_context(&intent, &gas_policy);

        assert_eq!(context.action, "mined_liquidity_removal_exit");
        assert_eq!(context.signal, "exit.liquidity_removal");
    }

    #[test]
    fn mempool_liquidity_removal_uses_mempool_race_action_label() {
        let intent = sell_intent(
            "exit.mempool_liquidity_removal_signal",
            Some("mempool_signal"),
        );
        let gas_policy = gas_policy();

        let context = sell_policy_context(&intent, &gas_policy);

        assert_eq!(context.action, "mempool_race_exit");
    }

    fn sell_intent(reason_code: &str, source: Option<&str>) -> OrderIntent {
        OrderIntent {
            trade_id: None,
            portfolio_id: PortfolioId("portfolio".to_string()),
            wallet_id: WalletId("wallet".to_string()),
            strategy_name: StrategyName("strategy".to_string()),
            side: OrderSide::Sell,
            token_address: Address::with_last_byte(1),
            pool_address: PoolAddress::from("0xtoken:0xpool"),
            protocol: PoolProtocol::UniswapV2,
            amount: Amount::zero(18),
            route: None,
            max_slippage_bps: 0,
            deadline_secs: 0,
            decision_reason: Some(DecisionReason {
                code: reason_code.to_string(),
                category: ReasonCategory::Exit,
                label: reason_code.to_string(),
                source: source.map(str::to_string),
                raw: Some(reason_code.to_string()),
                details: json!({}),
            }),
        }
    }

    fn gas_policy() -> LiveRealGasPolicy {
        LiveRealGasPolicy {
            required_gas_rank_source: "eth_chain_server_gas_rank".to_string(),
            gas_rank_lookback_blocks: 20,
            gas_rank_priority_tie_breaker_gwei: Decimal::ZERO,
            simulated_gas_buffer_bps: 0,
            max_priority_fee_gwei: Decimal::new(5, 0),
            entry_max_estimated_gas_fee_eth: Decimal::new(1, 1),
            exit_max_estimated_gas_fee_eth: Decimal::new(1, 1),
            safety_buffer_eth: Decimal::ZERO,
            v2_vault_buy_gas_limit: 1,
            v2_vault_sell_gas_limit: 1,
            mempool_race_priority_buffer_min_gwei: Decimal::new(1, 1),
            mempool_race_priority_buffer_max_gwei: Decimal::new(2, 1),
            entry_buy_gas_rank_policy: StrategyGasRankPolicy::p85_first(),
            tail_entry_buy_gas_rank_policy: StrategyGasRankPolicy::p85_first(),
            normal_exit_gas_rank_policy: StrategyGasRankPolicy::p75_first(),
            mempool_pre_mine_gas_rank_policy: StrategyGasRankPolicy::mempool_race_only(),
            lp_approval_exit_gas_rank_policy: StrategyGasRankPolicy::p90_first(),
        }
    }
}
