//! Transaction selection and prioritization

use crate::graph_discovery::types::TxCandidate;
use alloy_primitives::U256;

pub struct TxSelector;

impl TxSelector {
    /// Calculate priority score for a transaction candidate
    pub fn calculate_priority_score(tx: &mut TxCandidate) {
        let mut score = 0.0;
        
        // 1. Value score (logarithmic scale)
        // 1 ETH = 18, 10 ETH = 19, 100 ETH = 20, etc.
        let eth_value = tx.value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        if eth_value > 0.0 {
            score += (eth_value + 1.0).log10() * 10.0;
        }
        
        // 2. Unknown entities boost
        if tx.involves_unknown {
            score += 20.0; // High priority for discovering new entities
        }
        
        // 3. Router involvement boost
        if tx.involves_router {
            score += 15.0; // Routers hide real fund flows
        }
        
        // 4. Recency boost (newer transactions might be more relevant)
        // This is a placeholder - could be enhanced with current block number
        score += (tx.block_number as f64 / 1_000_000.0).min(5.0);
        
        tx.priority_score = score;
    }
    
    /// Sort and select top transactions for deep analysis
    pub fn select_top_transactions(
        mut candidates: Vec<TxCandidate>,
        max_count: usize,
    ) -> Vec<TxCandidate> {
        // Calculate scores
        for tx in candidates.iter_mut() {
            Self::calculate_priority_score(tx);
        }
        
        // Sort by priority score (highest first)
        candidates.sort_by(|a, b| {
            b.priority_score
                .partial_cmp(&a.priority_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Take top N
        candidates.into_iter().take(max_count).collect()
    }
    
    /// Filter transactions by various criteria
    pub fn filter_candidates(
        candidates: Vec<TxCandidate>,
        min_value: Option<U256>,
        must_involve_unknown: bool,
    ) -> Vec<TxCandidate> {
        candidates
            .into_iter()
            .filter(|tx| {
                // Value filter
                if let Some(min) = &min_value {
                    if tx.value < *min {
                        return false;
                    }
                }
                
                // Unknown entity filter
                if must_involve_unknown && !tx.involves_unknown {
                    return false;
                }
                
                true
            })
            .collect()
    }
}