//! Cluster confidence and risk-feature scoring.

use std::collections::BTreeSet;

/// Risk score for a cluster of addresses.
#[derive(Clone, Debug)]
pub struct ClusterRiskScore {
    pub cluster_id: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub confidence: f64,
}

/// Scores clusters based on control overlap, profit concentration,
/// funding homogeneity, timing synchrony, and liquidity events.
pub struct ClusterScorer;

impl ClusterScorer {
    pub fn new() -> Self {
        Self
    }

    /// Compute risk scores for all clusters.
    pub fn score_clusters(&self, _cluster_members: &BTreeSet<String>) -> Vec<ClusterRiskScore> {
        // TODO: Implement cluster risk scoring
        vec![]
    }
}
