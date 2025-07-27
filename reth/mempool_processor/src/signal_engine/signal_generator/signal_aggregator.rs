/// Signal Aggregator
/// 
/// Aggregates multiple signals for the same transaction/token

use super::signal_builder::{UnifiedSignal, SignalType};
use std::collections::HashMap;
use ethers::types::H256;
use serde::{Serialize, Deserialize};
use crate::signal_engine::types::Severity;

/// Aggregated signal combining multiple detections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedSignal {
    pub tx_hash: H256,
    pub primary_signal: UnifiedSignal,
    pub related_signals: Vec<UnifiedSignal>,
    pub combined_risk_score: u8,
    pub combined_severity: Severity,
    pub summary: String,
}

/// Signal aggregator
pub struct SignalAggregator {
    /// Buffer for aggregating signals
    signal_buffer: HashMap<H256, Vec<UnifiedSignal>>,
    /// Aggregation window in milliseconds
    aggregation_window_ms: u64,
}

impl SignalAggregator {
    pub fn new(aggregation_window_ms: u64) -> Self {
        Self {
            signal_buffer: HashMap::new(),
            aggregation_window_ms,
        }
    }

    /// Add signal to aggregator
    pub fn add_signal(&mut self, signal: UnifiedSignal) {
        self.signal_buffer
            .entry(signal.tx_hash)
            .or_insert_with(Vec::new)
            .push(signal);
    }

    /// Process and aggregate signals
    pub fn aggregate(&mut self) -> Vec<AggregatedSignal> {
        let mut aggregated = Vec::new();

        // Process each transaction's signals
        for (tx_hash, signals) in self.signal_buffer.drain() {
            if signals.is_empty() {
                continue;
            }

            if signals.len() == 1 {
                // Single signal, no aggregation needed
                let signal = signals.into_iter().next().unwrap();
                aggregated.push(AggregatedSignal {
                    tx_hash,
                    primary_signal: signal.clone(),
                    related_signals: vec![],
                    combined_risk_score: signal.risk_score,
                    combined_severity: signal.severity,
                    summary: self.generate_summary(&[signal]),
                });
            } else {
                // Multiple signals, need aggregation
                let agg_signal = self.aggregate_multiple(tx_hash, signals);
                aggregated.push(agg_signal);
            }
        }

        aggregated
    }

    /// Aggregate multiple signals for the same transaction
    fn aggregate_multiple(&self, tx_hash: H256, mut signals: Vec<UnifiedSignal>) -> AggregatedSignal {
        // Sort by severity (highest first) then by risk score
        signals.sort_by(|a, b| {
            b.severity.cmp(&a.severity)
                .then_with(|| b.risk_score.cmp(&a.risk_score))
        });

        // Primary signal is the most severe
        let primary_signal = signals.remove(0);
        let related_signals = signals;

        // Calculate combined risk score (weighted average with emphasis on highest)
        let combined_risk_score = self.calculate_combined_risk(&primary_signal, &related_signals);

        // Combined severity is the highest severity
        let combined_severity = primary_signal.severity;

        // Generate summary
        let mut all_signals = vec![primary_signal.clone()];
        all_signals.extend(related_signals.clone());
        let summary = self.generate_summary(&all_signals);

        AggregatedSignal {
            tx_hash,
            primary_signal,
            related_signals,
            combined_risk_score,
            combined_severity,
            summary,
        }
    }

    /// Calculate combined risk score
    fn calculate_combined_risk(&self, primary: &UnifiedSignal, related: &[UnifiedSignal]) -> u8 {
        if related.is_empty() {
            return primary.risk_score;
        }

        // Weight: 60% primary, 40% average of others
        let primary_weight = 0.6;
        let others_weight = 0.4;

        let others_avg: f64 = related.iter()
            .map(|s| s.risk_score as f64)
            .sum::<f64>() / related.len() as f64;

        let combined = (primary.risk_score as f64 * primary_weight) + (others_avg * others_weight);
        
        // Boost if multiple critical signals
        let critical_count = related.iter()
            .filter(|s| s.severity == Severity::Critical)
            .count();
        
        let boosted = if critical_count > 0 {
            combined + (critical_count as f64 * 5.0)
        } else {
            combined
        };

        boosted.round().min(100.0) as u8
    }

    /// Generate human-readable summary
    fn generate_summary(&self, signals: &[UnifiedSignal]) -> String {
        let mut summary_parts = Vec::new();

        // Count signal types
        let mut type_counts = HashMap::new();
        for signal in signals {
            *type_counts.entry(&signal.signal_type).or_insert(0) += 1;
        }

        // Add primary risks
        for signal in signals.iter().take(2) {
            match &signal.signal_type {
                SignalType::Honeypot => {
                    summary_parts.push("HONEYPOT DETECTED".to_string());
                }
                SignalType::LiquidityChange => {
                    summary_parts.push("Liquidity removal detected".to_string());
                }
                SignalType::TaxChange => {
                    summary_parts.push("Tax manipulation detected".to_string());
                }
                SignalType::TradingStatus => {
                    summary_parts.push("Trading status changed".to_string());
                }
                _ => {}
            }
        }

        // Add count summary if many signals
        if signals.len() > 2 {
            summary_parts.push(format!("{} total signals detected", signals.len()));
        }

        if summary_parts.is_empty() {
            "Multiple market signals detected".to_string()
        } else {
            summary_parts.join(". ")
        }
    }

    /// Clear old signals from buffer
    pub fn clear_old_signals(&mut self, current_time_ms: u64) {
        // In production, would check signal timestamps and remove old ones
        // For now, this is a placeholder
        if self.signal_buffer.len() > 1000 {
            self.signal_buffer.clear();
        }
    }
}