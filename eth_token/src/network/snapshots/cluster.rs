//! Cluster snapshots.

use serde::{Deserialize, Serialize};

/// Serializable snapshot of a detected cluster.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterSnapshot {
    pub cluster_id: String,
    pub member_addresses: Vec<String>,
    pub total_profit: f64,
    pub risk_score: f64,
    pub risk_reasons: Vec<String>,
    pub primary_labels: Vec<String>,
}
