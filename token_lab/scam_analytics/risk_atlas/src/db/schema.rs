use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskAtlasRun {
    pub run_id: String,
    pub chain: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub start_block: Option<i64>,
    pub end_block: Option<i64>,
    pub block_count: Option<i64>,
    pub token_count: Option<i64>,
    pub pool_count: Option<i64>,
    pub scam_label_count: Option<i64>,
    pub direct_lp_feature_row_count: Option<i64>,
    pub active_observation_row_count: Option<i64>,
    pub status: String,
    pub generated_at: DateTime<Utc>,
    pub metadata: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DistributionBucket {
    pub section: String,
    pub bucket: String,
    pub count: i64,
    pub share: Option<f64>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumericStat {
    pub section: String,
    pub metric: String,
    pub count: i64,
    pub min: Option<f64>,
    pub p25: Option<f64>,
    pub median: Option<f64>,
    pub p75: Option<f64>,
    pub p90: Option<f64>,
    pub p95: Option<f64>,
    pub max: Option<f64>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveTargetSummary {
    pub row_kind: String,
    pub horizon_active_observations: Option<i32>,
    pub rows: i64,
    pub unique_pools: Option<i64>,
    pub positives: Option<i64>,
    pub negatives: Option<i64>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewExample {
    pub queue: String,
    pub token_address: String,
    pub pool_address: String,
    pub protocol: Option<String>,
    pub symbol: Option<String>,
    pub mechanism: Option<String>,
    pub mechanism_label: Option<String>,
    pub label_block: Option<i64>,
    pub trading_enabled_block: Option<i64>,
    pub age_blocks: Option<i64>,
    pub liquidity_eth: Option<f64>,
    pub price_ratio_to_initial: Option<f64>,
    pub evidence_summary: Option<String>,
    pub evidence_ref: Option<String>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelReadinessItem {
    pub name: String,
    pub status: String,
    pub detail: Option<String>,
    pub sort_order: i32,
}
