//! Live trading policy scaffolding.
//!
//! This crate owns live strategy policy and the guarded handoff to Kartal.
//! It owns live transaction preparation and audit metadata, but it does not
//! sign, reserve nonces, or broadcast transactions locally.

pub mod calibration;
pub mod flashbots;
pub mod kartal;
pub mod kartal_executor;
pub mod lp_approval_exit;
pub mod planner;
pub mod tx_prep;

pub use calibration::{
    build_planner_calibration_request, run_calibration, CalibrationCaseInput,
    CalibrationCaseReport, CalibrationCaseVerdict, CalibrationInputFile, CalibrationOverallVerdict,
    CalibrationReport, CalibrationRunConfig, CalibrationSubmitReport, CalibrationSuiteInput,
    CalibrationSummary, CalibrationVerdictKind, ExpectedCalibrationOutcome,
    PlannerCalibrationFixtureConfig, PlannerCalibrationFixtureError, PlannerCalibrationRoute,
};
pub use flashbots::{
    FlashbotsBundleSubmission, FlashbotsClientError, FlashbotsMevShareClient,
    FlashbotsMevShareClientConfig, FlashbotsTailBundleRequest, DEFAULT_FLASHBOTS_RELAY_URL,
};
pub use kartal::{
    KartalClient, KartalClientConfig, KartalClientError, KartalDailySpendStatus,
    KartalEthTxExecutorStatus, KartalEthTxPolicyStatus, KartalPolicyDecision,
    KartalPolicyDecisionList, KartalServerError, KartalStatusBroadcastMode,
};
pub use kartal_executor::{
    KartalBribeRequest, KartalExecutorClient, KartalExecutorClientConfig,
    KartalExecutorClientError, KartalSignDirectRawResult, KartalSimulationReference,
    KartalSubmitDirectRawResult, LiveDirectRawTransactionRequest, LiveTraderTxSignal,
    LiveTxExecution,
};
pub use lp_approval_exit::{
    plan_lp_approval_response, BribeExitConfig, HeldPositionContext, LpApprovalSignal,
    LpSignalSource, PriorityRoute, PrioritySellPlan, SellUrgency, TradeAction,
};
pub use planner::{
    derive_min_output_from_expected_output, AllowanceCheck, AllowanceChecker, AllowanceDecision,
    AllowanceMode, ChainServerGasRankProvider, FixedGasRankProvider, FixedPreSubmitSimulator,
    GasRankPlan, GasRankProvider, LivePrioritySellPlanner, LivePrioritySellPlannerConfig,
    LivePrioritySellPlannerError, LivePrioritySellPlannerInput, MempoolRaceGasRankProvider,
    PlannerTxContext, PreSubmitSimulator, PrioritySellPlanner, PrioritySellPlannerOutcome,
    RouteBuildRequest, SellRouteBuilder, StaticAllowanceChecker, UniswapV2SellRouteBuilder,
    UniswapV2TradingVaultBuyRouteBuilder, UniswapV2TradingVaultPreSubmitSimulator,
    UniswapV2TradingVaultSellRouteBuilder, VaultInternalAllowanceChecker,
    UNISWAP_V2_DIRECT_SELL_GAS_LIMIT, UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT,
    UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
};
pub use tx_prep::{
    build_priority_sell_request, estimate_eth_cost_from_gwei, prepare_priority_sell,
    GasEstimateConfig, GasRankProfile, PreSubmitSimulation, PreparedSellRoute, PriorityFeeBudget,
    PriorityFeeBudgetInput, PrioritySellTxPrep, RankedFeeCandidate, StrategyGasRankDefaults,
    StrategyGasRankPolicy, StrategyTxKind, TxPrepConfig, TxPrepOutcome, TxPrepReject,
    TxPrepRequestContext, TxPrepRouteError, TxPrepSimulationError,
    DEFAULT_SIMULATED_GAS_ESTIMATE_BUFFER_BPS,
};
