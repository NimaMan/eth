use serde::{Deserialize, Serialize};

use crate::db::schema::{ActiveTargetSummary, DistributionBucket, NumericStat, RiskAtlasRun};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskAtlasPageStory {
    pub headline: String,
    pub narrative: String,
    pub range_label: String,
    pub source_label: String,
    pub steps: Vec<RiskAtlasPageStoryStep>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskAtlasPageStoryStep {
    pub id: String,
    pub eyebrow: String,
    pub title: String,
    pub headline: String,
    pub body: String,
    pub metric_label: String,
    pub metric_value: String,
    pub metric_detail: Option<String>,
    pub tone: String,
}

pub fn build_page_story(
    run: &RiskAtlasRun,
    distributions: &[DistributionBucket],
    numeric_stats: &[NumericStat],
    active_targets: &[ActiveTargetSummary],
) -> RiskAtlasPageStory {
    let pool_count = run.pool_count.unwrap_or_default();
    let token_count = run.token_count.unwrap_or_default();
    let scam_labels = run.scam_label_count.unwrap_or_default();
    let active_rows = run.active_observation_row_count.unwrap_or_default();
    let eligible = bucket_count(distributions, "pool_eligibility", "eligible");
    let ineligible = bucket_count(distributions, "pool_eligibility", "ineligible")
        .max(pool_count.saturating_sub(eligible));
    let scam_share = share(scam_labels, pool_count);
    let top_mechanism = top_bucket(distributions, "scam_mechanism_labels")
        .or_else(|| top_bucket(distributions, "scam_mechanisms"));
    let direct_lp_labels = bucket_count(
        distributions,
        "scam_mechanism_labels",
        "Direct LP Liquidity Removal",
    )
    .max(bucket_count(
        distributions,
        "scam_mechanisms",
        "direct_lp_liquidity_removal",
    ));
    let direct_lp_approval_rows = section_total(distributions, "direct_lp_approval_pre_removal")
        .max(run.direct_lp_feature_row_count.unwrap_or_default());
    let median_scam_chain_block_delta = stat_median(
        numeric_stats,
        "scam_age",
        "Trading Enabled To Label Chain Block Delta",
    );
    let median_scam_minutes = stat_median(
        numeric_stats,
        "scam_age",
        "Trading Enabled To Label Minutes",
    );
    let fast_scams = bucket_count(distributions, "time_to_scam_buckets", "same_block")
        + bucket_count(distributions, "time_to_scam_buckets", "fast_1_to_10");
    let fast_scams = fast_scams.max(
        bucket_count(distributions, "time_to_scam_buckets", "0_1_blocks")
            + bucket_count(distributions, "time_to_scam_buckets", "2_10_blocks"),
    );
    let direct_lp_preapproved =
        bucket_count(distributions, "direct_lp_approval_pre_removal", "true");
    let direct_lp_preapproved_share = share(direct_lp_preapproved, direct_lp_approval_rows);
    let target_1 = active_target_for_horizon(active_targets, 1);
    let target_10 = active_target_for_horizon(active_targets, 10);
    let active_positive = target_1
        .and_then(|target| target.positives)
        .unwrap_or_else(|| bucket_count(distributions, "active_target", "true"));
    let active_target_rows = target_1.map(|target| target.rows).unwrap_or(active_rows);
    let active_positive_share = share(active_positive, active_target_rows);
    let horizon_count = active_targets
        .iter()
        .filter(|item| item.active_observation_delta.is_some())
        .count();

    RiskAtlasPageStory {
        headline: "From token launch to tradable scam-risk target".to_string(),
        narrative: format!(
            "This snapshot follows {} tokens and {} pools across {} blocks. The page starts with the launch surface, checks scam-label coverage, narrows into direct LP removal signals, and ends at active_observation_delta targets that can become model rows.",
            fmt_count(token_count),
            fmt_count(pool_count),
            run.block_count.map(fmt_count).unwrap_or_else(|| "-".to_string()),
        ),
        range_label: match (run.start_block, run.end_block) {
            (Some(start), Some(end)) => format!("{} - {}", fmt_count(start), fmt_count(end)),
            _ => "-".to_string(),
        },
        source_label: run
            .source_ref
            .as_deref()
            .unwrap_or("risk atlas database")
            .to_string(),
        steps: vec![
            RiskAtlasPageStoryStep {
                id: "launch_surface".to_string(),
                eyebrow: "01 Launch".to_string(),
                title: "Launch Surface".to_string(),
                headline: format!(
                    "{} pools entered the {}-block surface",
                    fmt_count(pool_count),
                    run.block_count
                        .map(fmt_count)
                        .unwrap_or_else(|| "-".to_string()),
                ),
                body: format!(
                    "{} pools became eligible for downstream analysis. {} were filtered out before scam rates, targets, and model rows are computed.",
                    fmt_count(eligible),
                    fmt_count(ineligible),
                ),
                metric_label: "Eligible pools".to_string(),
                metric_value: fmt_percent(share(eligible, pool_count)),
                metric_detail: Some(format!("{} of {} pools", fmt_count(eligible), fmt_count(pool_count))),
                tone: "attention".to_string(),
            },
            RiskAtlasPageStoryStep {
                id: "scam_landscape".to_string(),
                eyebrow: "02 Labels".to_string(),
                title: "Scam Landscape".to_string(),
                headline: top_mechanism
                    .as_ref()
                    .map(|bucket| format!("{} is the largest labeled mechanism", labelize(&bucket.bucket)))
                    .unwrap_or_else(|| "Scam mechanism labels need more coverage".to_string()),
                body: format!(
                    "{} pools have scam labels. Direct LP removal accounts for {} labeled scams; pair-balance and reserve drains form the next label families.",
                    fmt_count(scam_labels),
                    fmt_count(direct_lp_labels),
                ),
                metric_label: "Scam-labeled pools".to_string(),
                metric_value: fmt_percent(scam_share),
                metric_detail: Some(format!("{} of {} pools", fmt_count(scam_labels), fmt_count(pool_count))),
                tone: "attention".to_string(),
            },
            RiskAtlasPageStoryStep {
                id: "time_to_scam".to_string(),
                eyebrow: "03 Timing".to_string(),
                title: "Time To Scam".to_string(),
                headline: format!(
                    "Median scam label arrives after {} active-chain blocks",
                    median_scam_chain_block_delta
                        .map(fmt_number)
                        .unwrap_or_else(|| "-".to_string()),
                ),
                body: format!(
                    "{} labels happen within the first 10 blocks after trading is enabled; median wall time is about {} minutes.",
                    fmt_count(fast_scams),
                    median_scam_minutes
                        .map(fmt_number)
                        .unwrap_or_else(|| "-".to_string()),
                ),
                metric_label: "Early scams".to_string(),
                metric_value: fmt_count(fast_scams),
                metric_detail: Some("0-10 blocks after trading enabled".to_string()),
                tone: "neutral".to_string(),
            },
            RiskAtlasPageStoryStep {
                id: "direct_lp_predictability".to_string(),
                eyebrow: "04 Signal".to_string(),
                title: "Direct LP Predictability".to_string(),
                headline: format!(
                    "{} of direct LP removals had pre-removal LP approval",
                    fmt_percent(direct_lp_preapproved_share),
                ),
                body: "This is the first concrete warning family: LP approval timing can be joined with the active observation state before the removal event.".to_string(),
                metric_label: "Pre-approved removals".to_string(),
                metric_value: fmt_count(direct_lp_preapproved),
                metric_detail: Some(format!("{} direct LP approval-timing rows", fmt_count(direct_lp_approval_rows))),
                tone: "ready".to_string(),
            },
            RiskAtlasPageStoryStep {
                id: "near_future_targets".to_string(),
                eyebrow: "05 Target".to_string(),
                title: "Near-Future Targets".to_string(),
                headline: format!(
                    "{} positive active_observation_delta rows are available",
                    fmt_count(active_positive),
                ),
                body: format!(
                    "The model target is direct LP removal within active_observation_delta values of 1, 2, 3, 5, or 10. The current snapshot has {} horizon families; active_observation_delta 10 has {} positives.",
                    fmt_count(horizon_count as i64),
                    fmt_count(target_10.and_then(|target| target.positives).unwrap_or_default()),
                ),
                metric_label: "One-step positive share".to_string(),
                metric_value: fmt_percent(active_positive_share),
                metric_detail: Some(format!("{} of {} target rows", fmt_count(active_positive), fmt_count(active_target_rows))),
                tone: "ready".to_string(),
            },
            RiskAtlasPageStoryStep {
                id: "trading_path".to_string(),
                eyebrow: "06 Use".to_string(),
                title: "Trading Path".to_string(),
                headline: "The first model should be a risk gate, not a trade selector".to_string(),
                body: "The immediate use is to reduce exposure to pools with near-future removal risk. Once labels and review queues are tighter, the same read model can feed trading guardrails and backtest filters.".to_string(),
                metric_label: "Model scope".to_string(),
                metric_value: "Risk gate".to_string(),
                metric_detail: Some("direct LP removal first".to_string()),
                tone: "neutral".to_string(),
            },
        ],
    }
}

fn bucket_count(distributions: &[DistributionBucket], section: &str, bucket: &str) -> i64 {
    distributions
        .iter()
        .find(|item| item.section == section && item.bucket == bucket)
        .map(|item| item.count)
        .unwrap_or_default()
}

fn section_total(distributions: &[DistributionBucket], section: &str) -> i64 {
    distributions
        .iter()
        .filter(|item| item.section == section)
        .map(|item| item.count)
        .sum()
}

fn top_bucket<'a>(
    distributions: &'a [DistributionBucket],
    section: &str,
) -> Option<&'a DistributionBucket> {
    distributions
        .iter()
        .filter(|item| item.section == section && item.bucket != "(empty)")
        .max_by_key(|item| item.count)
}

fn stat_median(numeric_stats: &[NumericStat], section: &str, metric: &str) -> Option<f64> {
    numeric_stats
        .iter()
        .find(|item| item.section == section && item.metric == metric)
        .and_then(|item| item.median)
}

fn active_target_for_horizon(
    active_targets: &[ActiveTargetSummary],
    horizon: i32,
) -> Option<&ActiveTargetSummary> {
    active_targets
        .iter()
        .find(|item| item.active_observation_delta == Some(horizon))
}

fn share(part: i64, whole: i64) -> Option<f64> {
    if whole <= 0 {
        None
    } else {
        Some(part as f64 / whole as f64)
    }
}

fn labelize(value: &str) -> String {
    value
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn fmt_count(value: i64) -> String {
    let negative = value < 0;
    let digits = value.abs().to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let mut out: String = out.chars().rev().collect();
    if negative {
        out.insert(0, '-');
    }
    out
}

fn fmt_percent(value: Option<f64>) -> String {
    value
        .map(|number| format!("{:.1}%", number * 100.0))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        fmt_count(value as i64)
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::*;

    #[test]
    fn builds_page_story_from_snapshot_rows() {
        let run = RiskAtlasRun {
            run_id: "test".to_string(),
            chain: "ethereum".to_string(),
            source_kind: "test".to_string(),
            source_ref: Some("source.md".to_string()),
            start_block: Some(1),
            end_block: Some(100),
            block_count: Some(100),
            token_count: Some(10),
            pool_count: Some(20),
            scam_label_count: Some(8),
            direct_lp_feature_row_count: Some(4),
            active_observation_row_count: Some(1000),
            status: "completed".to_string(),
            generated_at: Utc::now(),
            metadata: json!({}),
        };
        let distributions = vec![
            bucket("pool_eligibility", "eligible", 12),
            bucket("pool_eligibility", "ineligible", 8),
            bucket("scam_mechanism_labels", "Direct LP Liquidity Removal", 4),
            bucket("direct_lp_approval_pre_removal", "true", 3),
            bucket("direct_lp_approval_pre_removal", "false", 1),
            bucket("time_to_scam_buckets", "same_block", 1),
            bucket("time_to_scam_buckets", "fast_1_to_10", 2),
        ];
        let numeric_stats = vec![
            stat(
                "scam_age",
                "Trading Enabled To Label Chain Block Delta",
                25.0,
            ),
            stat("scam_age", "Trading Enabled To Label Minutes", 5.0),
        ];
        let active_targets = vec![ActiveTargetSummary {
            row_kind: "direct_lp_removal_within_active_observation_delta".to_string(),
            active_observation_delta: Some(1),
            rows: 500,
            unique_pools: Some(12),
            positives: Some(50),
            negatives: Some(450),
            sort_order: 1,
        }];

        let story = build_page_story(&run, &distributions, &numeric_stats, &active_targets);

        assert_eq!(story.range_label, "1 - 100");
        assert_eq!(story.steps.len(), 6);
        assert_eq!(story.steps[0].metric_value, "60.0%");
        assert_eq!(story.steps[3].metric_value, "3");
        assert_eq!(story.steps[4].metric_value, "10.0%");
    }

    fn bucket(section: &str, bucket: &str, count: i64) -> DistributionBucket {
        DistributionBucket {
            section: section.to_string(),
            bucket: bucket.to_string(),
            count,
            share: None,
            sort_order: 0,
        }
    }

    fn stat(section: &str, metric: &str, median: f64) -> NumericStat {
        NumericStat {
            section: section.to_string(),
            metric: metric.to_string(),
            count: 1,
            min: None,
            p25: None,
            median: Some(median),
            p75: None,
            p90: None,
            p95: None,
            max: None,
            sort_order: 0,
        }
    }
}
