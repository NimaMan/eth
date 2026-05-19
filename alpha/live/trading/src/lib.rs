//! Live trading policy scaffolding.
//!
//! This crate owns live strategy policy and the guarded handoff to Kartal.
//! It owns live transaction preparation and audit metadata, but it does not
//! sign, reserve nonces, or broadcast transactions locally.

pub mod kartal_executor;
pub mod lp_approval_exit;
pub mod planner;
pub mod tx_prep;

pub use kartal_executor::{
    KartalBribeRequest, KartalExecutorClient, KartalExecutorClientConfig,
    KartalExecutorClientError, KartalSimulationReference, KartalSubmitDirectRawResult,
    LiveDirectRawTransactionRequest, LiveTraderTxSignal,
};
pub use lp_approval_exit::{
    plan_lp_approval_response, BribeExitConfig, HeldPositionContext, LpApprovalSignal,
    LpSignalSource, PriorityRoute, PrioritySellPlan, SellUrgency, TradeAction,
};
pub use planner::{
    AllowanceCheck, AllowanceChecker, AllowanceDecision, AllowanceMode, FixedGasRankProvider,
    FixedPreSubmitSimulator, GasRankPlan, GasRankProvider, LivePrioritySellPlanner,
    LivePrioritySellPlannerConfig, LivePrioritySellPlannerError, LivePrioritySellPlannerInput,
    PlannerTxContext, PreSubmitSimulator, PrioritySellPlanner, PrioritySellPlannerOutcome,
    RouteBuildRequest, SellRouteBuilder, StaticAllowanceChecker, UniswapV2SellRouteBuilder,
    UniswapV2TradingVaultSellRouteBuilder, VaultInternalAllowanceChecker,
};
pub use tx_prep::{
    build_priority_sell_request, estimate_eth_cost_from_gwei, prepare_priority_sell,
    PreSubmitSimulation, PreparedSellRoute, PriorityFeeBudget, PriorityFeeBudgetInput,
    PrioritySellTxPrep, RankedFeeCandidate, TxPrepConfig, TxPrepOutcome, TxPrepReject,
    TxPrepRequestContext, TxPrepRouteError, TxPrepSimulationError,
};
