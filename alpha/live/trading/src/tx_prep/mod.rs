//! Priority transaction preparation for live exits.
//!
//! This module turns an already approved live exit plan into a fully prepared
//! Kartal direct-raw request. It owns value-capped priority-fee policy and
//! audit metadata, but it does not discover routes, sign, reserve nonces, or
//! broadcast.

mod budget;
mod config;
mod gas_plan;
mod metadata;
mod policy;
mod request;
mod route;
mod simulation;
mod strategy_gas_policy;

pub use budget::{estimate_eth_cost_from_gwei, PriorityFeeBudget, PriorityFeeBudgetInput};
pub use config::{GasEstimateConfig, DEFAULT_SIMULATED_GAS_ESTIMATE_BUFFER_BPS};
pub use gas_plan::{
    apply_min_priority_fee_floor, apply_min_priority_fee_floor_to_candidates, choose_ranked_fee,
    GasPlan, GasPlanDecision, RankedFeeCandidate, MEMPOOL_RACE_GAS_LABEL, MEMPOOL_RACE_GAS_SOURCE,
};
pub use policy::{
    prepare_priority_sell, PrioritySellTxPrep, TxPrepConfig, TxPrepOutcome, TxPrepReject,
};
pub use request::{build_priority_sell_request, TxPrepRequestContext};
pub use route::{PreparedSellRoute, TxPrepRouteError};
pub use simulation::{PreSubmitSimulation, TxPrepSimulationError};
pub use strategy_gas_policy::{
    GasRankProfile, StrategyGasRankDefaults, StrategyGasRankPolicy, StrategyTxKind,
    TxSubmissionRoute,
};

pub(crate) fn gwei_to_wei_string(gwei: eth_alpha_core::amount::DecimalAmount) -> String {
    decimal_floor_string(gwei.max(eth_alpha_core::amount::DecimalAmount::ZERO) * wei_per_gwei())
}

pub(crate) fn decimal_floor_string(value: eth_alpha_core::amount::DecimalAmount) -> String {
    let text = value.normalize().to_string();
    match text.find('.') {
        Some(index) => text[..index].to_string(),
        None => text,
    }
}

pub(crate) fn wei_per_gwei() -> eth_alpha_core::amount::DecimalAmount {
    eth_alpha_core::amount::DecimalAmount::from(1_000_000_000u64)
}

pub(crate) fn gwei_per_eth() -> eth_alpha_core::amount::DecimalAmount {
    eth_alpha_core::amount::DecimalAmount::from(1_000_000_000u64)
}
