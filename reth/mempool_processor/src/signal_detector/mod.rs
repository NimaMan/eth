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
/// Currently supports V2 pools only (V3/V4 in development)

pub mod types;
pub mod liquidity_detector;
pub mod tax_change_detector;
pub mod tax_detector;
pub mod trading_status_detector;
pub mod stablecoin_detector;
pub mod lp_approval_detector;
pub mod signal_manager;

pub use types::{Signal, TradingEnabledSignal, HighTaxWarningSignal, LiquidityRemovalSignal, ScamDetectionSignal, TaxWarningType};
pub use liquidity_detector::{LiquidityDetector, LiquiditySignal, LiquidityChangeType, SignalType as LiquiditySignalType};
pub use tax_change_detector::{TaxChangeDetector, TaxChangeSignal};
pub use tax_detector::{TaxDetector, TaxSignal, TaxSignalType};
pub use trading_status_detector::{TradingStatusDetector, TradingStatusSignal};
pub use stablecoin_detector::{StablecoinDetector, StablecoinSignal, StablecoinActivityType};
pub use lp_approval_detector::{LpApprovalDetector, LpApprovalSignal};
pub use signal_manager::{SignalManager, SignalManagerConfig};