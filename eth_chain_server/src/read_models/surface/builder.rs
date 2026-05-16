use std::collections::{BTreeMap, BTreeSet};

use eth_token::erc20::ERC20Token;
use eth_token::tracking::TrackedTokenStatus;

use super::filters::{is_active_pool, is_active_pool_parts};
use super::types::{
    TokenPoolReasonCount, TokenPoolSurfaceBadge, TokenPoolSurfaceContext, TokenPoolSurfaceKind,
    TokenPoolSurfaceResponse, TokenPoolSurfaceRow, TokenPoolSurfaceSection,
    TokenPoolSurfaceSections, TokenPoolSurfaceStats,
};
use crate::ranges::RangeIndexJob;
use crate::read_models::live::LiveProgressSummary;
use crate::read_models::pool::PoolView;
use crate::read_models::token::TokenView;

pub async fn range_surface(run: &RangeIndexJob) -> TokenPoolSurfaceResponse {
    let state = run.state.read().await;
    build_token_pool_surface(
        TokenPoolSurfaceContext {
            surface: TokenPoolSurfaceKind::Range,
            run_id: Some(run.id.clone()),
        },
        None,
        state.processor.registry.tokens.values(),
        |token_address| {
            state
                .processor
                .token_index
                .entries
                .get(&normalize_address(token_address))
                .map(|entry| entry.token_status.clone())
        },
    )
}

pub fn build_token_pool_surface<'a, I, F>(
    context: TokenPoolSurfaceContext,
    progress_summary: Option<LiveProgressSummary>,
    tokens: I,
    mut index_status: F,
) -> TokenPoolSurfaceResponse
where
    I: IntoIterator<Item = &'a ERC20Token>,
    F: FnMut(&str) -> Option<TrackedTokenStatus>,
{
    let mut active_rows = Vec::new();
    let mut ineligible_rows = Vec::new();
    let mut scammed_rows = Vec::new();
    let mut unpooled_rows = Vec::new();
    let mut pooled_token_addresses = BTreeSet::new();
    let mut launch_timestamps_by_token = BTreeMap::<String, u64>::new();
    let mut ineligible_reason_counts = BTreeMap::<(String, String), usize>::new();
    let mut total_tokens = 0usize;
    let mut pool_rows = 0usize;
    let mut eligible = 0usize;
    let mut ineligible = 0usize;
    let mut active = 0usize;
    let mut scammed = 0usize;

    for token in tokens {
        total_tokens += 1;
        let token_view = TokenView::from_token(token, index_status(&token.contract_address));
        let pools = PoolView::from_token_pool_summaries(token);

        if pools.is_empty() {
            unpooled_rows.push(TokenPoolSurfaceRow {
                section_keys: vec!["unpooled".to_string()],
                primary_label: "No pool".to_string(),
                badges: vec![TokenPoolSurfaceBadge {
                    key: "unpooled".to_string(),
                    label: "No pool".to_string(),
                    tone: "muted".to_string(),
                }],
                token: token_view,
                pool: None,
            });
            continue;
        }

        pooled_token_addresses.insert(token.contract_address.clone());
        for pool in pools {
            pool_rows += 1;
            let pool_is_eligible = pool.pool_classification.eligible;
            let pool_is_active = is_active_pool(&pool);
            let pool_is_scammed = pool.is_scam;
            let pool_is_ineligible = !pool_is_eligible;
            let section_keys = row_section_keys(&pool);
            let row = TokenPoolSurfaceRow {
                primary_label: row_primary_label(&pool),
                badges: row_badges(&pool),
                section_keys,
                token: token_view.clone(),
                pool: Some(pool.clone()),
            };

            record_launch_timestamp(&mut launch_timestamps_by_token, token, &pool);

            if pool_is_eligible {
                eligible += 1;
            }
            if pool_is_ineligible {
                ineligible += 1;
                record_ineligible_reason(&mut ineligible_reason_counts, &pool);
                ineligible_rows.push(row.clone());
            }
            if pool_is_active {
                active += 1;
                active_rows.push(row.clone());
            }
            if pool_is_scammed {
                scammed += 1;
                scammed_rows.push(row);
            }
        }
    }

    sort_rows(&mut active_rows);
    sort_rows(&mut ineligible_rows);
    sort_rows(&mut scammed_rows);
    sort_rows(&mut unpooled_rows);

    let launch_stats = launch_stats(&launch_timestamps_by_token);
    let reason_counts = reason_counts(ineligible_reason_counts, pool_rows, ineligible);
    TokenPoolSurfaceResponse {
        context,
        progress_summary,
        stats: TokenPoolSurfaceStats {
            total_tokens,
            pool_rows,
            pooled_tokens: pooled_token_addresses.len(),
            unpooled_tokens: unpooled_rows.len(),
            eligible,
            eligible_rate_percent: percent(eligible, pool_rows),
            ineligible,
            active,
            scammed,
            launches_per_hour: launch_stats.launches_per_hour,
            latest_hour_launches: launch_stats.latest_hour_launches,
        },
        sections: TokenPoolSurfaceSections {
            active: section("active", "Active Pools", active_rows),
            ineligible: section("ineligible", "Ineligible Pools", ineligible_rows),
            scammed: section("scammed", "Scammed Pools", scammed_rows),
            unpooled: section("unpooled", "Tokens Without Pools", unpooled_rows),
        },
        ineligible_reason_counts: reason_counts,
    }
}

fn row_section_keys(pool: &PoolView) -> Vec<String> {
    section_keys_for_parts(pool.pool_classification.eligible, pool.is_scam)
}

fn section_keys_for_parts(eligible: bool, is_scam: bool) -> Vec<String> {
    let mut keys = Vec::new();
    if is_active_pool_parts(eligible, is_scam) {
        keys.push("active".to_string());
    }
    if !eligible {
        keys.push("ineligible".to_string());
    }
    if is_scam {
        keys.push("scammed".to_string());
    }
    keys
}

fn row_primary_label(pool: &PoolView) -> String {
    if is_active_pool(pool) {
        return "Active".to_string();
    }
    if !pool.pool_classification.eligible {
        return pool
            .pool_classification
            .reason_label
            .unwrap_or("Ineligible")
            .to_string();
    }
    if pool.is_scam {
        return pool
            .scam_label
            .clone()
            .or_else(|| pool.risk_label.clone())
            .unwrap_or_else(|| "Scammed".to_string());
    }
    "Pool".to_string()
}

fn row_badges(pool: &PoolView) -> Vec<TokenPoolSurfaceBadge> {
    let mut badges = Vec::new();
    if pool.pool_classification.eligible {
        badges.push(TokenPoolSurfaceBadge {
            key: "eligible".to_string(),
            label: "Eligible".to_string(),
            tone: "good".to_string(),
        });
    } else {
        badges.push(TokenPoolSurfaceBadge {
            key: pool
                .pool_classification
                .reason_key
                .unwrap_or("ineligible")
                .to_string(),
            label: pool
                .pool_classification
                .reason_label
                .unwrap_or("Ineligible")
                .to_string(),
            tone: "warn".to_string(),
        });
    }
    if is_active_pool(pool) {
        badges.push(TokenPoolSurfaceBadge {
            key: "active".to_string(),
            label: "Active".to_string(),
            tone: "good".to_string(),
        });
    }
    if pool.is_scam {
        badges.push(TokenPoolSurfaceBadge {
            key: "scammed".to_string(),
            label: pool
                .scam_label
                .clone()
                .or_else(|| pool.risk_label.clone())
                .unwrap_or_else(|| "Scammed".to_string()),
            tone: "bad".to_string(),
        });
    }
    badges
}

fn record_launch_timestamp(
    timestamps_by_token: &mut BTreeMap<String, u64>,
    token: &ERC20Token,
    pool: &PoolView,
) {
    let timestamp = token
        .creation_timestamp
        .or(pool.creation_timestamp)
        .unwrap_or_default();
    if timestamp == 0 {
        return;
    }
    timestamps_by_token
        .entry(token.contract_address.clone())
        .and_modify(|existing| *existing = (*existing).min(timestamp))
        .or_insert(timestamp);
}

fn record_ineligible_reason(counts: &mut BTreeMap<(String, String), usize>, pool: &PoolView) {
    let key = pool
        .pool_classification
        .reason_key
        .unwrap_or("unknown")
        .to_string();
    let label = pool
        .pool_classification
        .reason_label
        .unwrap_or("Unknown")
        .to_string();
    *counts.entry((key, label)).or_default() += 1;
}

fn reason_counts(
    counts: BTreeMap<(String, String), usize>,
    pool_rows: usize,
    ineligible: usize,
) -> Vec<TokenPoolReasonCount> {
    let mut rows: Vec<_> = counts
        .into_iter()
        .map(|((key, label), count)| TokenPoolReasonCount {
            key,
            label,
            count,
            share_percent: percent(count, pool_rows),
            ineligible_share_percent: percent(count, ineligible),
        })
        .collect();
    rows.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then(left.label.cmp(&right.label))
    });
    rows
}

#[derive(Clone, Copy, Debug)]
struct LaunchStats {
    launches_per_hour: Option<f64>,
    latest_hour_launches: Option<usize>,
}

fn launch_stats(timestamps_by_token: &BTreeMap<String, u64>) -> LaunchStats {
    if timestamps_by_token.is_empty() {
        return LaunchStats {
            launches_per_hour: None,
            latest_hour_launches: None,
        };
    }

    let mut buckets = BTreeMap::<u64, usize>::new();
    for timestamp in timestamps_by_token.values() {
        *buckets.entry(timestamp / 3600).or_default() += 1;
    }

    let min_hour = buckets.keys().next().copied().unwrap_or_default();
    let max_hour = buckets.keys().next_back().copied().unwrap_or(min_hour);
    let span_hours = (max_hour.saturating_sub(min_hour) + 1).max(1);
    LaunchStats {
        launches_per_hour: Some(timestamps_by_token.len() as f64 / span_hours as f64),
        latest_hour_launches: buckets.get(&max_hour).copied(),
    }
}

fn section(
    key: &'static str,
    label: &'static str,
    rows: Vec<TokenPoolSurfaceRow>,
) -> TokenPoolSurfaceSection {
    TokenPoolSurfaceSection {
        key: key.to_string(),
        label: label.to_string(),
        row_count: rows.len(),
        token_count: token_count(&rows),
        rows,
    }
}

fn sort_rows(rows: &mut [TokenPoolSurfaceRow]) {
    rows.sort_by(|left, right| {
        row_timestamp(right)
            .cmp(&row_timestamp(left))
            .then(
                left.token
                    .contract_address
                    .cmp(&right.token.contract_address),
            )
            .then(
                left.pool
                    .as_ref()
                    .map(|pool| pool.pool_address.as_str())
                    .unwrap_or_default()
                    .cmp(
                        right
                            .pool
                            .as_ref()
                            .map(|pool| pool.pool_address.as_str())
                            .unwrap_or_default(),
                    ),
            )
    });
}

fn row_timestamp(row: &TokenPoolSurfaceRow) -> Option<u64> {
    row.pool
        .as_ref()
        .and_then(|pool| pool.creation_timestamp)
        .or(row.token.creation_timestamp)
}

fn token_count(rows: &[TokenPoolSurfaceRow]) -> usize {
    rows.iter()
        .map(|row| row.token.contract_address.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn percent(count: usize, total: usize) -> Option<f64> {
    (total > 0).then(|| (count as f64 / total as f64) * 100.0)
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_keys_allow_ineligible_scam_overlap() {
        let keys = section_keys_for_parts(false, true);
        assert_eq!(keys, vec!["ineligible".to_string(), "scammed".to_string()]);
    }
}
