//! Priority transaction preparation for live exits.
//!
//! This module turns an already approved live exit plan into a fully prepared
//! Kartal direct-raw request. It owns value-capped priority-fee policy and
//! audit metadata, but it does not discover routes, sign, reserve nonces, or
//! broadcast.

mod budget;
mod gas_plan;
mod metadata;
mod policy;
mod request;
mod route;
mod simulation;
mod strategy_gas_policy;

pub use budget::{PriorityFeeBudget, PriorityFeeBudgetInput, estimate_eth_cost_from_gwei};
pub use gas_plan::{GasPlan, GasPlanDecision, RankedFeeCandidate, choose_ranked_fee};
pub use policy::{
    PrioritySellTxPrep, TxPrepConfig, TxPrepOutcome, TxPrepReject, prepare_priority_sell,
};
pub use request::{TxPrepRequestContext, build_priority_sell_request};
pub use route::{PreparedSellRoute, TxPrepRouteError};
pub use simulation::{PreSubmitSimulation, TxPrepSimulationError};
pub use strategy_gas_policy::{
    GasRankProfile, StrategyGasRankDefaults, StrategyGasRankPolicy, StrategyTxKind,
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
