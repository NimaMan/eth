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
mod min_output;
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
pub use gas_rank::{
    ChainServerGasRankProvider, FixedGasRankProvider, GasRankPlan, GasRankProvider,
};
pub use input::{LivePrioritySellPlannerInput, PlannerTxContext};
pub use min_output::derive_min_output_from_expected_output;
pub use output::PrioritySellPlannerOutcome;
pub use priority_sell::{LivePrioritySellPlanner, PrioritySellPlanner};
pub use route_builder::{
    RouteBuildRequest, SellRouteBuilder, UniswapV2SellRouteBuilder,
    UniswapV2TradingVaultBuyRouteBuilder, UniswapV2TradingVaultSellRouteBuilder,
    UNISWAP_V2_DIRECT_SELL_GAS_LIMIT, UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT,
    UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
};
pub use simulation::{
    FixedPreSubmitSimulator, PreSubmitSimulator, UniswapV2TradingVaultPreSubmitSimulator,
};
