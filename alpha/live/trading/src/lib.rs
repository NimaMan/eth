//! Live trading policy scaffolding.
//!
//! This crate owns live strategy policy and the guarded handoff to Kartal.
//! It owns live transaction preparation and audit metadata, but it does not
//! sign, reserve nonces, or broadcast transactions locally.

pub mod calibration;
pub mod kartal;
pub mod kartal_executor;
pub mod lp_approval_exit;
pub mod planner;
pub mod tx_prep;

pub use calibration::{
    CalibrationCaseInput, CalibrationCaseReport, CalibrationCaseVerdict, CalibrationInputFile,
    CalibrationOverallVerdict, CalibrationReport, CalibrationRunConfig, CalibrationSubmitReport,
    CalibrationSuiteInput, CalibrationSummary, CalibrationVerdictKind, ExpectedCalibrationOutcome,
    PlannerCalibrationFixtureConfig, PlannerCalibrationFixtureError, PlannerCalibrationRoute,
    build_planner_calibration_request, run_calibration,
};
pub use kartal::{
    KartalClient, KartalClientConfig, KartalClientError, KartalDailySpendStatus,
    KartalEthTxExecutorStatus, KartalEthTxPolicyStatus, KartalPolicyDecision,
    KartalPolicyDecisionList, KartalServerError, KartalStatusBroadcastMode,
};
pub use kartal_executor::{
    KartalBribeRequest, KartalExecutorClient, KartalExecutorClientConfig,
    KartalExecutorClientError, KartalSimulationReference, KartalSubmitDirectRawResult,
    LiveDirectRawTransactionRequest, LiveTraderTxSignal,
};
pub use lp_approval_exit::{
    BribeExitConfig, HeldPositionContext, LpApprovalSignal, LpSignalSource, PriorityRoute,
    PrioritySellPlan, SellUrgency, TradeAction, plan_lp_approval_response,
};
pub use planner::{
    AllowanceCheck, AllowanceChecker, AllowanceDecision, AllowanceMode, FixedGasRankProvider,
    FixedPreSubmitSimulator, GasRankPlan, GasRankProvider, LivePrioritySellPlanner,
    LivePrioritySellPlannerConfig, LivePrioritySellPlannerError, LivePrioritySellPlannerInput,
    PlannerTxContext, PreSubmitSimulator, PrioritySellPlanner, PrioritySellPlannerOutcome,
    RouteBuildRequest, SellRouteBuilder, StaticAllowanceChecker, UniswapV2SellRouteBuilder,
    UniswapV2TradingVaultBuyRouteBuilder, UniswapV2TradingVaultPreSubmitSimulator,
    UniswapV2TradingVaultSellRouteBuilder, VaultInternalAllowanceChecker,
    derive_min_output_from_expected_output,
};
pub use tx_prep::{
    GasRankProfile, PreSubmitSimulation, PreparedSellRoute, PriorityFeeBudget,
    PriorityFeeBudgetInput, PrioritySellTxPrep, RankedFeeCandidate, StrategyGasRankDefaults,
    StrategyGasRankPolicy, StrategyTxKind, TxPrepConfig, TxPrepOutcome, TxPrepReject,
    TxPrepRequestContext, TxPrepRouteError, TxPrepSimulationError, build_priority_sell_request,
    estimate_eth_cost_from_gwei, prepare_priority_sell,
};
