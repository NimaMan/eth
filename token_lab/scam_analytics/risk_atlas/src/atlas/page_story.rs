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
    let control_candidates = bucket_count(
        distributions,
        "pool_eligibility",
        "non_scam_control_candidate",
    );
    let ineligible = pool_count.saturating_sub(scam_labels + control_candidates);
    let scam_share = share(scam_labels, pool_count);
    let top_mechanism = top_bucket(distributions, "scam_mechanism_labels")
        .or_else(|| top_bucket(distributions, "scam_mechanisms"));
    let direct_lp = bucket_count(
        distributions,
        "scam_mechanism_labels",
        "Direct LP Liquidity Removal",
    )
    .max(bucket_count(
        distributions,
        "scam_mechanisms",
        "direct_lp_liquidity_removal",
    ));
    let direct_lp_share = share(direct_lp, scam_labels);
    let median_scam_blocks =
        stat_median(numeric_stats, "scam_age", "Trading Enabled To Label Blocks");
    let median_scam_minutes = stat_median(
        numeric_stats,
        "scam_age",
        "Trading Enabled To Label Minutes",
    );
    let fast_scams = bucket_count(distributions, "time_to_scam_buckets", "same_block")
        + bucket_count(distributions, "time_to_scam_buckets", "fast_1_to_10");
    let direct_lp_preapproved =
        bucket_count(distributions, "direct_lp_approval_pre_removal", "true");
    let direct_lp_preapproved_share = share(direct_lp_preapproved, direct_lp);
    let active_positive = bucket_count(distributions, "active_target", "true")
        .max(row_kind_rows(active_targets, "pre_label_positive"));
    let active_positive_share = share(active_positive, active_rows);
    let horizon_count = active_targets
        .iter()
        .filter(|item| item.horizon_active_observations.is_some())
        .count();

    RiskAtlasPageStory {
        headline: "From token launch to tradable scam-risk target".to_string(),
        narrative: format!(
            "This snapshot follows {} tokens and {} pools across {} blocks. The page starts with the launch surface, checks scam-label coverage, narrows into direct LP removal signals, and ends at active-observation targets that can become model rows.",
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
                    "{} pools have scam labels, {} remain control candidates, and {} are currently outside the clean training set.",
                    fmt_count(scam_labels),
                    fmt_count(control_candidates),
                    fmt_count(ineligible),
                ),
                metric_label: "Scam-labeled pools".to_string(),
                metric_value: fmt_percent(scam_share),
                metric_detail: Some(format!("{} of {} pools", fmt_count(scam_labels), fmt_count(pool_count))),
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
                    "Direct LP removal accounts for {} labeled scams; reserve drains and pair-balance drains form the next label families.",
                    fmt_count(direct_lp),
                ),
                metric_label: "Direct LP share".to_string(),
                metric_value: fmt_percent(direct_lp_share),
                metric_detail: Some(format!("{} direct LP rows", fmt_count(direct_lp))),
                tone: "attention".to_string(),
            },
            RiskAtlasPageStoryStep {
                id: "time_to_scam".to_string(),
                eyebrow: "03 Timing".to_string(),
                title: "Time To Scam".to_string(),
                headline: format!(
                    "Median scam label arrives after {} active-chain blocks",
                    median_scam_blocks
                        .map(fmt_number)
                        .unwrap_or_else(|| "-".to_string()),
                ),
                body: format!(
                    "{} labels happen in the same block or within 10 blocks after trading is enabled; median wall time is about {} minutes.",
                    fmt_count(fast_scams),
                    median_scam_minutes
                        .map(fmt_number)
                        .unwrap_or_else(|| "-".to_string()),
                ),
                metric_label: "Fast scams".to_string(),
                metric_value: fmt_count(fast_scams),
                metric_detail: Some("same block or 1-10 blocks".to_string()),
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
                metric_detail: Some(format!("{} direct LP removals total", fmt_count(direct_lp))),
                tone: "ready".to_string(),
            },
            RiskAtlasPageStoryStep {
                id: "near_future_targets".to_string(),
                eyebrow: "05 Target".to_string(),
                title: "Near-Future Targets".to_string(),
                headline: format!(
                    "{} positive active-observation rows are available",
                    fmt_count(active_positive),
                ),
                body: format!(
                    "The model target is direct LP removal within the next 1, 2, 3, 5, or 10 active token/pool observations. The current snapshot has {} horizon families.",
                    fmt_count(horizon_count as i64),
                ),
                metric_label: "Positive row share".to_string(),
                metric_value: fmt_percent(active_positive_share),
                metric_detail: Some(format!("{} of {} active rows", fmt_count(active_positive), fmt_count(active_rows))),
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

fn row_kind_rows(active_targets: &[ActiveTargetSummary], row_kind: &str) -> i64 {
    active_targets
        .iter()
        .find(|item| item.row_kind == row_kind && item.horizon_active_observations.is_none())
        .map(|item| item.rows)
        .unwrap_or_default()
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
            bucket("pool_eligibility", "non_scam_control_candidate", 2),
            bucket("scam_mechanism_labels", "Direct LP Liquidity Removal", 4),
            bucket("direct_lp_approval_pre_removal", "true", 3),
            bucket("time_to_scam_buckets", "same_block", 1),
            bucket("time_to_scam_buckets", "fast_1_to_10", 2),
            bucket("active_target", "true", 50),
        ];
        let numeric_stats = vec![
            stat("scam_age", "Trading Enabled To Label Blocks", 25.0),
            stat("scam_age", "Trading Enabled To Label Minutes", 5.0),
        ];

        let story = build_page_story(&run, &distributions, &numeric_stats, &[]);

        assert_eq!(story.range_label, "1 - 100");
        assert_eq!(story.steps.len(), 6);
        assert_eq!(story.steps[0].metric_value, "40.0%");
        assert_eq!(story.steps[3].metric_value, "3");
        assert_eq!(story.steps[4].metric_value, "5.0%");
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
