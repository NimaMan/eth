pub mod liquidity_detector;
pub mod lp_approval_detector;
pub mod signal_manager;
pub mod tax_signal_detector;
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
/// V2/Sushi and V3 pool signals are supported when token-cache metadata is
/// available. V4 liquidity-removal intent is surfaced as unknown-severity risk
/// until V4 simulation/PnL is validated end to end.
pub mod types;

pub use liquidity_detector::{
    LiquidityChangeType, LiquidityDetector, LiquiditySignal, SignalType as LiquiditySignalType,
};
pub use lp_approval_detector::{LpApprovalDetector, LpApprovalSignal};
pub use signal_manager::{SignalManager, SignalManagerConfig};
pub use tax_signal_detector::{TaxDetector, TaxSignal, TaxSignalType};
pub use trading_status_detector::{TradingStatusDetector, TradingStatusSignal};
pub use types::{
    HighTaxWarningSignal, HoneypotSignal, LiquidityRemovalSignal, Signal, TaxWarningType,
    TradingEnabledSignal,
};
