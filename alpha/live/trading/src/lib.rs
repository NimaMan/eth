//! Live trading policy scaffolding.
//!
//! This crate owns live strategy policy and the guarded handoff to ETH tx executor.
//! It owns live transaction preparation and audit metadata, but it does not
//! sign, reserve nonces, or broadcast transactions locally.

pub const ETH_UNSIGNED_TX_WIRE_PROTOCOL: &str = "eth_unsigned_tx";

pub mod calibration;
pub mod eth_tx_executor;
pub mod eth_tx_submission;
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
pub use eth_tx_executor::{
    EthTxDailySpendStatus, EthTxExecutorBroadcastMode, EthTxExecutorClient,
    EthTxExecutorClientConfig, EthTxExecutorClientError, EthTxExecutorServerError,
    EthTxExecutorStatus, EthTxPolicyDecision, EthTxPolicyDecisionList, EthTxPolicyStatus,
};
pub use eth_tx_submission::{
    EthTxBribeRequest, EthTxExecutorSubmitClient, EthTxExecutorSubmitClientConfig,
    EthTxExecutorSubmitClientError, EthTxSimulationReference, EthTxSubmitDirectRawResult,
    EthTxSubmitTransactionRequest, EthTxSubmitTransactionResult, LiveDirectRawTransactionRequest,
    LiveTraderTxSignal, LiveTxExecution, TxSubmissionPolicy,
};
pub use lp_approval_exit::{
    plan_lp_approval_response, BribeExitConfig, HeldPositionContext, LpApprovalSignal,
    LpSignalSource, PriorityRoute, PrioritySellPlan, SellUrgency, TradeAction,
};
pub use planner::{
    derive_min_output_from_expected_output, AllowanceCheck, AllowanceChecker, AllowanceDecision,
    AllowanceMode, ChainServerGasRankProvider, ChainServerLivePreSubmitSimulator,
    FixedGasRankProvider, FixedPreSubmitSimulator, GasRankPlan, GasRankProvider,
    LivePrioritySellPlanner, LivePrioritySellPlannerConfig, LivePrioritySellPlannerError,
    LivePrioritySellPlannerInput, MempoolRaceGasRankProvider, PlannerTxContext, PreSubmitSimulator,
    PrioritySellPlanner, PrioritySellPlannerOutcome, RouteBuildRequest, SellRouteBuilder,
    StaticAllowanceChecker, UniswapV2SellRouteBuilder, UniswapV2TradingVaultBuyRouteBuilder,
    UniswapV2TradingVaultSellRouteBuilder, VaultInternalAllowanceChecker,
    UNISWAP_V2_DIRECT_SELL_GAS_LIMIT, UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT,
    UNISWAP_V2_TRADING_VAULT_SELL_GAS_LIMIT,
};
pub use tx_prep::{
    apply_min_priority_fee_floor, apply_min_priority_fee_floor_to_candidates,
    build_priority_sell_request, estimate_eth_cost_from_gwei, prepare_priority_sell,
    GasEstimateConfig, GasRankProfile, PreSubmitSimulation, PreparedSellRoute, PriorityFeeBudget,
    PriorityFeeBudgetInput, PrioritySellTxPrep, RankedFeeCandidate, StrategyGasRankDefaults,
    StrategyGasRankPolicy, StrategyTxKind, TxPrepConfig, TxPrepOutcome, TxPrepReject,
    TxPrepRequestContext, TxPrepRouteError, TxPrepSimulationError, TxSubmissionRoute,
    DEFAULT_SIMULATED_GAS_ESTIMATE_BUFFER_BPS,
};
