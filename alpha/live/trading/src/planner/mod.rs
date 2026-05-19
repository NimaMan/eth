//! Live transaction planner for real priority sells.
//!
//! The planner is the bridge between an alpha `OrderIntent` and the existing
//! `tx_prep` value-capped Kartal request builder. It owns live route assembly,
//! allowance checks, final simulation evidence, and gas-rank inputs. It still
//! does not sign, reserve nonces, or broadcast.

mod allowance;
mod config;
mod error;
mod gas_rank;
mod input;
mod output;
mod priority_sell;
mod route_builder;
mod simulation;

pub use allowance::{
    AllowanceCheck, AllowanceChecker, AllowanceDecision, AllowanceMode, StaticAllowanceChecker,
    VaultInternalAllowanceChecker,
};
pub use config::LivePrioritySellPlannerConfig;
pub use error::LivePrioritySellPlannerError;
pub use gas_rank::{FixedGasRankProvider, GasRankPlan, GasRankProvider};
pub use input::{LivePrioritySellPlannerInput, PlannerTxContext};
pub use output::PrioritySellPlannerOutcome;
pub use priority_sell::{LivePrioritySellPlanner, PrioritySellPlanner};
pub use route_builder::{
    BaygusV2VaultSellRouteBuilder, RouteBuildRequest, SellRouteBuilder, UniswapV2SellRouteBuilder,
};
pub use simulation::{FixedPreSubmitSimulator, PreSubmitSimulator};
