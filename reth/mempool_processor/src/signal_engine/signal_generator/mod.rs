/// Signal Generator Module
/// 
/// Generates unified signals with risk scoring and aggregation

pub mod signal_builder;
pub mod risk_scorer;
pub mod signal_aggregator;

pub use signal_builder::{SignalBuilder, UnifiedSignal, SignalData};
pub use risk_scorer::{RiskScorer, RiskFactors};
pub use signal_aggregator::{SignalAggregator, AggregatedSignal};