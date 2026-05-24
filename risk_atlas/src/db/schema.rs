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
pub struct PoolEligibilityRow {
    pub token_address: String,
    pub pool_address: String,
    pub protocol: Option<String>,
    pub quote_symbol: Option<String>,
    pub eligible: bool,
    pub eligibility_block: Option<i64>,
    pub eligibility_liquidity: Option<f64>,
    pub first_observed_block: Option<i64>,
    pub last_observed_block: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventEvidenceRow {
    pub token_address: String,
    pub pool_address: String,
    pub protocol: String,
    pub event_kind: String,
    pub mechanism: Option<String>,
    pub block_number: Option<i64>,
    pub block_timestamp: Option<i64>,
    pub tx_hash: Option<String>,
    pub mempool_first_seen_ms: Option<i64>,
    pub source: Option<String>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationRow {
    pub token_address: String,
    pub pool_address: String,
    pub denom_address: String,
    pub protocol: String,
    pub active_observation_index: i64,
    pub block_number: i64,
    pub timestamp: Option<i64>,
    pub active_reasons: Value,
    pub tx_count: i32,
    pub token_transfer_count: i32,
    pub denom_transfer_count: i32,
    pub buy_volume_denom: Option<f64>,
    pub sell_volume_denom: Option<f64>,
    pub total_bribe_eth: Option<f64>,
    pub can_buy: bool,
    pub can_sell: bool,
    pub effective_can_buy: bool,
    pub effective_can_sell: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub liquidity_removed_as_of: bool,
    pub liquidity_removal_in_block: bool,
    pub liquidity_removal_block_as_of: Option<i64>,
    pub direct_lp_removal_as_of: bool,
    pub direct_lp_removal_in_block: bool,
    pub direct_lp_target_1: Option<bool>,
    pub direct_lp_target_2: Option<bool>,
    pub direct_lp_target_3: Option<bool>,
    pub direct_lp_target_5: Option<bool>,
    pub direct_lp_target_10: Option<bool>,
    pub denom_reserve: Option<f64>,
    pub token_reserve: Option<f64>,
    pub total_liquidity_denom: Option<f64>,
    pub price_to_initial_ratio: Option<f64>,
    pub lp_approved_pct_as_of: Option<f64>,
    pub token_decimals: Option<i32>,
    pub price_denom_per_token: Option<f64>,
    pub initial_price_denom_per_token: Option<f64>,
    pub lp_approval_count_in_block: i32,
    pub lp_approval_seen_as_of: Option<bool>,
    pub lp_total_supply: Option<f64>,
    pub lp_max_approval_amount_as_of: Option<f64>,
    pub lp_max_approval_pct_as_of: Option<f64>,
    pub lp_removable_pct_as_of: Option<f64>,
    pub lp_router_removable_pct_as_of: Option<f64>,
    pub lp_approval_owner_is_creator: Option<bool>,
    pub creator_lp_balance_pct_as_of: Option<f64>,
    pub creator_lp_approved_pct_as_of: Option<f64>,
    pub creator_lp_removable_pct_as_of: Option<f64>,
    pub creator_lp_router_removable_pct_as_of: Option<f64>,
    pub creator_lp_approved_gt_90_pct_as_of: Option<bool>,
    pub creator_lp_router_removable_gt_90_pct_as_of: Option<bool>,
    pub last_lp_approval_amount_pct_of_total_supply: Option<f64>,
    pub first_lp_approval_to_as_of_chain_block_delta: Option<i64>,
    pub last_lp_approval_to_as_of_chain_block_delta: Option<i64>,
    pub pool_creation_to_first_lp_approval_chain_block_delta: Option<i64>,
    pub pool_creation_to_last_lp_approval_chain_block_delta: Option<i64>,
    pub trading_enabled_to_first_lp_approval_chain_block_delta: Option<i64>,
    pub trading_enabled_to_last_lp_approval_chain_block_delta: Option<i64>,
    pub control_transfer_from_after_renounce_seen_as_of: Option<bool>,
    pub control_transfer_from_after_renounce_in_block: Option<bool>,
    pub control_transfer_from_holder_to_burn_seen_as_of: Option<bool>,
    pub control_transfer_from_holder_to_burn_in_block: Option<bool>,
    pub control_transfer_from_pair_seen_as_of: Option<bool>,
    pub control_transfer_from_pair_in_block: Option<bool>,
    pub control_transfer_from_without_transfer_log_seen_as_of: Option<bool>,
    pub control_transfer_from_without_transfer_log_in_block: Option<bool>,
    pub pair_token_to_control_seen_as_of: Option<bool>,
    pub pair_token_to_control_in_block: Option<bool>,
    pub pair_token_to_control_to_pool_reserve_ratio: Option<f64>,
    pub pair_balance_backdoor_signal_seen_as_of: Option<bool>,
    pub pair_balance_backdoor_signal_in_block: Option<bool>,
    pub last_pair_balance_backdoor_signal_to_as_of_chain_block_delta: Option<i64>,
    pub token_transfer_to_total_supply_ratio: Option<f64>,
    pub token_transfer_to_pool_token_reserve_ratio: Option<f64>,
    pub observation: Value,
    pub features: Value,
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
    pub active_observation_delta: Option<i32>,
    pub rows: i64,
    pub unique_pools: Option<i64>,
    pub positives: Option<i64>,
    pub negatives: Option<i64>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionQuestion {
    pub question_id: String,
    pub category: String,
    pub question: String,
    pub headline: Option<String>,
    pub answer: Option<String>,
    pub status: String,
    pub denominator_label: Option<String>,
    pub denominator_count: Option<i64>,
    pub payload: Value,
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
    pub trading_enabled_to_label_chain_block_delta: Option<i64>,
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
