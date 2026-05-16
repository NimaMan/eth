use crate::read_models::live::LiveProgressSummary;
use crate::read_models::pool::PoolView;
use crate::read_models::token::TokenView;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenPoolSurfaceKind {
    Live,
    Range,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolSurfaceContext {
    pub surface: TokenPoolSurfaceKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolSurfaceResponse {
    pub context: TokenPoolSurfaceContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_summary: Option<LiveProgressSummary>,
    pub stats: TokenPoolSurfaceStats,
    pub sections: TokenPoolSurfaceSections,
    pub ineligible_reason_counts: Vec<TokenPoolReasonCount>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolSurfaceStats {
    pub total_tokens: usize,
    pub pool_rows: usize,
    pub pooled_tokens: usize,
    pub unpooled_tokens: usize,
    pub eligible: usize,
    pub eligible_rate_percent: Option<f64>,
    pub ineligible: usize,
    pub active: usize,
    pub scammed: usize,
    pub launches_per_hour: Option<f64>,
    pub latest_hour_launches: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolSurfaceSections {
    pub active: TokenPoolSurfaceSection,
    pub ineligible: TokenPoolSurfaceSection,
    pub scammed: TokenPoolSurfaceSection,
    pub unpooled: TokenPoolSurfaceSection,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolSurfaceSection {
    pub key: String,
    pub label: String,
    pub row_count: usize,
    pub token_count: usize,
    pub rows: Vec<TokenPoolSurfaceRow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolSurfaceRow {
    pub section_keys: Vec<String>,
    pub primary_label: String,
    pub badges: Vec<TokenPoolSurfaceBadge>,
    pub token: TokenView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<PoolView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolSurfaceBadge {
    pub key: String,
    pub label: String,
    pub tone: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenPoolReasonCount {
    pub key: String,
    pub label: String,
    pub count: usize,
    pub share_percent: Option<f64>,
    pub ineligible_share_percent: Option<f64>,
}
