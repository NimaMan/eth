/// Signal Detectors Module
/// 
/// Specialized detectors for different types of market signals

pub mod types;
pub mod honeypot_detector;
pub mod liquidity_detector;
pub mod tax_change_detector;
pub mod trading_status_detector;
pub mod signal_detector_engine;

pub use types::{Signal, TradingEnabledSignal, HighTaxWarningSignal, LiquidityRemovalSignal, ScamDetectionSignal, TaxWarningType};
pub use honeypot_detector::{HoneypotDetector, HoneypotSignal};
pub use liquidity_detector::{LiquidityDetector, LiquiditySignal, LiquidityChangeType};
pub use tax_change_detector::{TaxChangeDetector, TaxChangeSignal};
pub use trading_status_detector::{TradingStatusDetector, TradingStatusSignal};
pub use signal_detector_engine::{SignalDetector, SignalDetectionConfig, SimulationSignal};