pub mod liquidity_detector;
pub mod lp_approval_detector;
mod lp_approval_enrichment;
mod pool_labels;
mod position_approval_signal;
pub mod signal_manager;
pub mod tax_signal_detector;
pub mod token_supply_risk_detector;
pub mod trading_status_detector;
/// Signal Detectors Module - Per-Pool Signal Generation
///
/// CRITICAL ARCHITECTURE:
/// All signals are generated PER-POOL, not per-token.
///
/// Key Concepts:
/// - A token can have multiple pools (WETH/TOKEN, USDC/TOKEN, etc.)
/// - Each pool is monitored and signaled INDEPENDENTLY
/// - Signal = f(token_address, pool_address)
/// - Pool-specific metrics: taxes, liquidity, trading status
///
/// Signal Flow:
/// 1. Simulation Manager processes EACH pool separately
/// 2. Each pool gets its own SimulationResult
/// 3. Signal Manager receives pool-specific results
/// 4. Generates unique signal for (token, pool) pair
///
/// Protocol-specific pool signals are supported only when token-cache metadata
/// provides enough pool identity and ownership-share context for public action.
pub mod types;

pub use liquidity_detector::{
    LiquidityChangeType, LiquidityDetector, LiquiditySignal, SignalType as LiquiditySignalType,
};
pub use lp_approval_detector::{LpApprovalDetector, LpApprovalSignal};
pub(crate) use lp_approval_enrichment::enrich_erc20_liquidity_approval;
pub(crate) use pool_labels::pool_type_label;
pub use position_approval_signal::build_position_approval_signals;
pub use signal_manager::{SignalManager, SignalManagerConfig};
pub use tax_signal_detector::{TaxDetector, TaxSignal, TaxSignalType};
pub use token_supply_risk_detector::TokenSupplyRiskDetector;
pub use trading_status_detector::{TradingStatusDetector, TradingStatusSignal};
pub use types::{
    HighTaxWarningSignal, HoneypotSignal, LiquidityRemovalSignal, Signal, TaxSignalRecord,
    TaxWarningType, TokenSupplyRiskSignal, TradingEnabledSignal,
};
