/// Risk Scorer
/// 
/// Calculates risk scores for signals based on multiple factors

use std::sync::Arc;
use crate::token_tracking::TokenTrackingCache;
use crate::signal_engine::classifier::TransactionCategory;
use crate::signal_engine::detectors::{
    HoneypotSignal, LiquiditySignal, TaxChangeSignal, TradingStatusSignal,
    LiquidityChangeType,
};
use crate::signal_engine::detectors::tax_change_detector::TaxRiskLevel;
use tracing::debug;

/// Risk scoring factors
#[derive(Debug, Clone)]
pub struct RiskFactors {
    /// Creator has previous scams
    pub creator_history_score: u8,
    /// Token age (newer = higher risk)
    pub token_age_score: u8,
    /// Liquidity amount (lower = higher risk)
    pub liquidity_score: u8,
    /// Tax levels
    pub tax_score: u8,
    /// Function risk
    pub function_score: u8,
    /// Private mempool usage
    pub private_mempool_score: u8,
    /// Pattern confidence
    pub pattern_score: u8,
}

/// Risk scorer
pub struct RiskScorer {
    token_cache: Option<Arc<TokenTrackingCache>>,
}

impl RiskScorer {
    pub fn new(token_cache: Option<Arc<TokenTrackingCache>>) -> Self {
        Self { token_cache }
    }

    /// Calculate risk score for honeypot signal
    pub async fn score_honeypot(
        &self,
        signal: &HoneypotSignal,
        category: &TransactionCategory,
    ) -> u8 {
        let mut factors = RiskFactors {
            creator_history_score: 0,
            token_age_score: 50, // Default medium risk
            liquidity_score: 50,
            tax_score: 0,
            function_score: 80, // High risk function
            private_mempool_score: 0,
            pattern_score: 90, // Honeypot pattern is very high risk
        };

        // Score based on taxes
        if let Some(sell_tax) = signal.sell_tax {
            factors.tax_score = match sell_tax {
                t if t > 90.0 => 100,
                t if t > 50.0 => 90,
                t if t > 30.0 => 70,
                t if t > 20.0 => 50,
                _ => 30,
            };
        }

        // Check creator history if available
        if let Some(ref cache) = self.token_cache {
            if let TransactionCategory::CreatorTransaction { creator, .. } = category {
                factors.creator_history_score = self.calculate_creator_risk(creator, cache).await;
            }
        }

        // Calculate weighted average
        self.calculate_weighted_score(&factors)
    }

    /// Calculate risk score for liquidity signal
    pub async fn score_liquidity(
        &self,
        signal: &LiquiditySignal,
        category: &TransactionCategory,
    ) -> u8 {
        let mut factors = RiskFactors {
            creator_history_score: 0,
            token_age_score: 50,
            liquidity_score: 0,
            tax_score: 0,
            function_score: 60, // Liquidity removal is risky
            private_mempool_score: 0,
            pattern_score: 0,
        };

        // Score based on liquidity change type
        factors.pattern_score = match signal.change_type {
            LiquidityChangeType::CompleteDrain => 100,
            LiquidityChangeType::MajorRemoval => 80,
            LiquidityChangeType::SignificantRemoval => 60,
            LiquidityChangeType::MinorRemoval => 30,
            LiquidityChangeType::Addition => 10,
        };

        // Score based on remaining liquidity
        factors.liquidity_score = if signal.remaining_liquidity < 0.3 {
            100 // Very low liquidity
        } else if signal.remaining_liquidity < 1.0 {
            80
        } else if signal.remaining_liquidity < 5.0 {
            60
        } else if signal.remaining_liquidity < 10.0 {
            40
        } else {
            20
        };

        self.calculate_weighted_score(&factors)
    }

    /// Calculate risk score for tax change signal
    pub async fn score_tax_change(
        &self,
        signal: &TaxChangeSignal,
        _category: &TransactionCategory,
    ) -> u8 {
        let mut factors = RiskFactors {
            creator_history_score: 0,
            token_age_score: 50,
            liquidity_score: 50,
            tax_score: 0,
            function_score: 70, // Tax changes are risky
            private_mempool_score: 0,
            pattern_score: 0,
        };

        // Score based on risk level
        factors.pattern_score = match signal.risk_level {
            TaxRiskLevel::Critical => 90,
            TaxRiskLevel::High => 70,
            TaxRiskLevel::Medium => 50,
            TaxRiskLevel::Low => 20,
        };

        // Extra risk if honeypot
        if signal.is_honeypot_after {
            factors.pattern_score = 100;
        }

        // Score based on tax levels
        let max_tax = signal.after.buy_tax.unwrap_or(0.0)
            .max(signal.after.sell_tax.unwrap_or(0.0));
        
        factors.tax_score = match max_tax {
            t if t > 50.0 => 100,
            t if t > 30.0 => 80,
            t if t > 20.0 => 60,
            t if t > 10.0 => 40,
            _ => 20,
        };

        self.calculate_weighted_score(&factors)
    }

    /// Calculate risk score for trading status signal
    pub async fn score_trading_status(
        &self,
        signal: &TradingStatusSignal,
        _category: &TransactionCategory,
    ) -> u8 {
        let factors = RiskFactors {
            creator_history_score: 0,
            token_age_score: 30, // Trading enable is lower risk
            liquidity_score: 50,
            tax_score: 0,
            function_score: 40,
            private_mempool_score: 0,
            pattern_score: if signal.can_trade_after { 20 } else { 80 },
        };

        self.calculate_weighted_score(&factors)
    }

    /// Calculate creator risk score
    async fn calculate_creator_risk(
        &self,
        creator_address: &str,
        cache: &TokenTrackingCache,
    ) -> u8 {
        // Check number of tokens created
        let tokens = cache.get_creator_tokens(creator_address).await;
        let token_count = tokens.len();

        // Check if uses private mempool
        let uses_private = cache.is_creator_private_mempool(creator_address).await;

        // Calculate score
        let mut score = match token_count {
            0..=1 => 20,  // New creator
            2..=5 => 40,  // Some history
            6..=10 => 60, // Many tokens
            _ => 80,      // Serial creator
        };

        if uses_private {
            score += 20; // Private mempool adds risk
        }

        score.min(100)
    }

    /// Calculate weighted risk score
    fn calculate_weighted_score(&self, factors: &RiskFactors) -> u8 {
        debug!("Risk factors: {:?}", factors);

        // Weights for different factors
        let weights = [
            (factors.creator_history_score, 0.15),
            (factors.token_age_score, 0.10),
            (factors.liquidity_score, 0.15),
            (factors.tax_score, 0.20),
            (factors.function_score, 0.10),
            (factors.private_mempool_score, 0.10),
            (factors.pattern_score, 0.20),
        ];

        let total_weight: f64 = weights.iter().map(|(_, w)| w).sum();
        let weighted_sum: f64 = weights.iter()
            .map(|(score, weight)| *score as f64 * weight)
            .sum();

        let final_score = (weighted_sum / total_weight).round() as u8;
        final_score.min(100)
    }
}