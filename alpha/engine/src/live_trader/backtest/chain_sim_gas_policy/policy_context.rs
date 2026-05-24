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
    if reason.contains("lp_approval") || reason.contains("liquidity_removal") {
        return SellPolicyContext {
            policy: &gas_policy.lp_approval_exit_gas_rank_policy,
            action: "lp_approval_exit",
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
