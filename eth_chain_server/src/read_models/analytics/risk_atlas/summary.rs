use std::collections::{BTreeMap, BTreeSet};

use eth_risk_atlas::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, ModelReadinessItem, NumericStat,
    ObservationRow,
};
use serde_json::{json, Value};

use crate::read_models::pool::PoolView;

pub(super) fn active_target_summaries(rows: &[ObservationRow]) -> Vec<ActiveTargetSummary> {
    let mut summaries = Vec::new();
    let unique_pools = rows
        .iter()
        .map(|row| (row.token_address.clone(), row.pool_address.clone()))
        .collect::<BTreeSet<_>>()
        .len() as i64;
    summaries.push(ActiveTargetSummary {
        row_kind: "active_pool_observation".to_string(),
        active_observation_delta: None,
        rows: rows.len() as i64,
        unique_pools: Some(unique_pools),
        positives: None,
        negatives: None,
        sort_order: 0,
    });

    for (sort_order, horizon) in [1, 2, 3, 5, 10].into_iter().enumerate() {
        let values = rows
            .iter()
            .filter_map(|row| target_for_horizon(row, horizon))
            .collect::<Vec<_>>();
        let positives = values.iter().filter(|value| **value).count() as i64;
        let negatives = values.len() as i64 - positives;
        let target_pools = rows
            .iter()
            .filter(|row| target_for_horizon(row, horizon).is_some())
            .map(|row| (row.token_address.clone(), row.pool_address.clone()))
            .collect::<BTreeSet<_>>()
            .len() as i64;
        summaries.push(ActiveTargetSummary {
            row_kind: "direct_lp_removal_within_active_observation_delta".to_string(),
            active_observation_delta: Some(horizon),
            rows: values.len() as i64,
            unique_pools: Some(target_pools),
            positives: Some(positives),
            negatives: Some(negatives),
            sort_order: sort_order as i32 + 1,
        });
    }

    summaries
}

fn target_for_horizon(row: &ObservationRow, horizon: i32) -> Option<bool> {
    match horizon {
        1 => row.direct_lp_target_1,
        2 => row.direct_lp_target_2,
        3 => row.direct_lp_target_3,
        5 => row.direct_lp_target_5,
        10 => row.direct_lp_target_10,
        _ => None,
    }
}

pub(super) fn append_observation_distributions(
    distributions: &mut DistributionAccumulator,
    rows: &[ObservationRow],
) {
    for row in rows {
        distributions.add("observation_trading_state", trading_bucket(row), 1);
        distributions.add("observation_primary_reason", primary_reason(row), 1);
        if row.direct_lp_removal_as_of {
            distributions.add("observation_direct_lp_as_of", "true", 1);
        }
        for horizon in [1, 2, 3, 5, 10] {
            if let Some(value) = target_for_horizon(row, horizon) {
                distributions.add(
                    format!("direct_lp_target_{horizon}_active_observation_delta"),
                    value.to_string(),
                    1,
                );
            }
        }
    }
}

pub(super) fn numeric_stats(
    scam_ages: Vec<f64>,
    direct_lp_approval_leads: Vec<f64>,
) -> Vec<NumericStat> {
    let mut stats = Vec::new();
    if let Some(stat) = numeric_stat(
        "scam_age",
        "Trading Enabled To Label Chain Block Delta",
        scam_ages,
        0,
    ) {
        stats.push(stat);
    }
    if let Some(stat) = numeric_stat(
        "direct_lp_age_approval",
        "Last Pre-Removal LP Approval To Removal Chain Block Delta",
        direct_lp_approval_leads,
        0,
    ) {
        stats.push(stat);
    }
    stats
}

fn numeric_stat(
    section: &str,
    metric: &str,
    mut values: Vec<f64>,
    sort_order: i32,
) -> Option<NumericStat> {
    values.retain(|value| value.is_finite());
    values.sort_by(f64::total_cmp);
    if values.is_empty() {
        return None;
    }
    Some(NumericStat {
        section: section.to_string(),
        metric: metric.to_string(),
        count: values.len() as i64,
        min: values.first().copied(),
        p25: percentile(&values, 0.25),
        median: percentile(&values, 0.5),
        p75: percentile(&values, 0.75),
        p90: percentile(&values, 0.90),
        p95: percentile(&values, 0.95),
        max: values.last().copied(),
        sort_order,
    })
}

pub(super) fn decision_questions(
    pool_count: i64,
    eligible_count: i64,
    ineligible_count: i64,
    scam_count: i64,
    direct_lp_count: i64,
    active_targets: &[ActiveTargetSummary],
    numeric_stats: &[NumericStat],
    distributions: &DistributionAccumulator,
) -> Vec<DecisionQuestion> {
    let target_1 = active_targets
        .iter()
        .find(|row| row.active_observation_delta == Some(1));
    let target_10 = active_targets
        .iter()
        .find(|row| row.active_observation_delta == Some(10));
    let target_horizons = active_targets
        .iter()
        .filter(|row| row.active_observation_delta.is_some())
        .collect::<Vec<_>>();
    let scam_age = numeric_stats.iter().find(|row| row.section == "scam_age");
    let direct_lp_approval_lead = numeric_stats
        .iter()
        .find(|row| row.section == "direct_lp_age_approval");
    let pre_removal_approval_count =
        distributions.count_for_section("direct_lp_approval_pre_removal", "true");
    let direct_lp_approval_rows = distributions
        .section_total("direct_lp_approval_pre_removal")
        .max(direct_lp_count);
    let economic_buy_sell =
        distributions.count_for_section("observation_trading_state", "economic_buy_sell");
    let active_observation_rows = active_targets
        .iter()
        .find(|row| row.active_observation_delta.is_none())
        .map(|row| row.rows)
        .unwrap_or_default();
    vec![
        question(
            "eligible_pool_filter",
            "Launch filter",
            "Which pools enter the eligible cohort?",
            format!("{} eligible", fmt_count(eligible_count)),
            format!(
                "{} of {} pools became eligible. {} are filtered before scam analytics, model rows, and trading-candidate analysis.",
                fmt_count(eligible_count),
                fmt_count(pool_count),
                fmt_count(ineligible_count)
            ),
            Some("pool rows"),
            Some(pool_count),
            json!({
                "rows": distributions.rows_for_section("pool_eligibility"),
                "buckets": distributions.rows_for_section("pool_ineligible_reasons"),
                "use": usage(
                    "This is the first denominator for every downstream scam-rate and target question.",
                    "We exclude ineligible pools from model targets and from strategy research because these are pools we would not consider trading."
                ),
            }),
            0,
        ),
        question(
            "ineligible_reason_mix",
            "Launch filter",
            "Why were pools excluded before modeling?",
            format!("{} ineligible", fmt_count(ineligible_count)),
            format!(
                "{} pools were excluded. The reason mix tells us whether the filter is mostly low-liquidity noise, unsupported quotes, or pools that were not economically buyable/sellable.",
                fmt_count(ineligible_count)
            ),
            Some("ineligible pools"),
            Some(ineligible_count),
            json!({
                "rows": distributions.rows_for_section("pool_ineligible_reasons"),
                "use": usage(
                    "This shows whether the launch filter is removing expected junk or hiding pools we may want to support later.",
                    "We use it to decide whether to improve pool classification before trusting scam-rate or model statistics."
                ),
            }),
            1,
        ),
        question(
            "eligible_protocol_mix",
            "Launch surface",
            "Which protocols dominate the eligible cohort?",
            format!("{} eligible pools", fmt_count(eligible_count)),
            "Protocol mix matters because the available execution path, scam mechanism mix, and LP-approval semantics differ by AMM family.".to_string(),
            Some("eligible pools"),
            Some(eligible_count),
            json!({
                "rows": distributions.rows_for_section("eligible_protocols"),
                "use": usage(
                    "This tells us which protocol families need model coverage and simulator coverage first.",
                    "We currently backtest the Risk Atlas chain-sim strategy on V2 only because V3/V4 route metadata is not persisted yet."
                ),
            }),
            2,
        ),
        question(
            "scam_mechanism_mix",
            "Scam labels",
            "Which scam mechanisms dominate eligible scam labels?",
            format!("{} scam labels", fmt_count(scam_count)),
            format!(
                "{} pools are scam-labeled; {} are direct LP removals in the feature export.",
                fmt_count(scam_count),
                fmt_count(direct_lp_count)
            ),
            Some("scam labels"),
            Some(scam_count),
            json!({
                "rows": distributions.rows_for_section("scam_mechanisms"),
                "use": usage(
                    "This tells us which failure mechanisms need separate labels and separate predictors.",
                    "We start with direct LP removal because it has a concrete mined-chain warning family, then separate pair-balance and reserve-drain models."
                ),
            }),
            3,
        ),
        question(
            "scam_time_from_trading_enabled",
            "Timing",
            "How fast do scam pools fail after trading becomes enabled?",
            scam_age
                .and_then(|stat| stat.median)
                .map(|median| format!("{median:.0} chain_block_delta median"))
                .unwrap_or_else(|| "no timing rows".to_string()),
            scam_age
                .map(|stat| {
                    format!(
                        "Median label age is {}; P90 is {}. The target model should therefore be near-future and active_observation_delta based, not a lifetime scam/no-scam label.",
                        fmt_optional(stat.median),
                        fmt_optional(stat.p90)
                    )
                })
                .unwrap_or_else(|| "No scam timing rows were present in this range.".to_string()),
            Some("scam labels"),
            scam_age.map(|stat| stat.count),
            json!({
                "stats": scam_age,
                "buckets": distributions.rows_for_section("time_to_scam_buckets"),
                "use": usage(
                    "This helps choose hold windows and target horizons around when scams actually happen.",
                    "We use active_observation_delta values of 1, 2, 3, 5, and 10 so similar launches can diverge as their state evolves."
                ),
            }),
            4,
        ),
        question(
            "direct_lp_approval_coverage",
            "Direct LP warning",
            "How often is LP approval visible before direct LP liquidity removal?",
            format!(
                "{} pre-approved",
                fmt_count(pre_removal_approval_count)
            ),
            format!(
                "{} of {} direct LP approval-timing rows had pre-removal LP approval ({}).",
                fmt_count(pre_removal_approval_count),
                fmt_count(direct_lp_approval_rows),
                fmt_ratio_percent(pre_removal_approval_count, direct_lp_approval_rows)
            ),
            Some("direct LP rows"),
            Some(direct_lp_approval_rows),
            json!({
                "rows": distributions.rows_for_section("direct_lp_approval_pre_removal"),
                "use": usage(
                    "A mined LP approval can be an exit warning or a no-entry condition before liquidity is removed.",
                    "We use it as a risk gate and exit cap, not as the profit source. The latest backtest showed the PnL came mostly from active-hold launch momentum."
                ),
            }),
            5,
        ),
        question(
            "direct_lp_approval_lead",
            "Direct LP warning",
            "How much confirmed-chain warning does LP approval give before removal?",
            direct_lp_approval_lead
                .and_then(|stat| stat.median)
                .map(|median| format!("{median:.0} chain_block_delta median"))
                .unwrap_or_else(|| "no lead rows".to_string()),
            direct_lp_approval_lead
                .map(|stat| {
                    format!(
                        "Pre-removal LP approval appears a median {} chain_block_delta before direct removal; P25 is {}, P90 is {}.",
                        fmt_optional(stat.median),
                        fmt_optional(stat.p25),
                        fmt_optional(stat.p90)
                    )
                })
                .unwrap_or_else(|| "No LP approval lead rows were present in this range.".to_string()),
            Some("pre-approved removals"),
            direct_lp_approval_lead.map(|stat| stat.count),
            json!({
                "stats": direct_lp_approval_lead,
                "use": usage(
                    "Lead time tells us whether a mined approval can realistically trigger a sell before removal.",
                    "We use this to separate approvals with enough confirmed-block warning from same-block or ordering-dependent cases."
                ),
            }),
            6,
        ),
        question(
            "direct_lp_active_horizons",
            "Near-future target",
            "What is the direct LP removal base rate across active_observation_delta horizons?",
            target_headline(target_10),
            target_answer(target_1, 1) + " " + &target_answer(target_10, 10),
            Some("target rows"),
            target_10.map(|row| row.rows),
            json!({
                "horizons": target_horizons,
                "use": usage(
                    "These base rates are the baseline any risk model must beat.",
                    "We use active_observation_delta values of 1, 2, 3, 5, and 10 as the first supervised labels for direct LP removal risk."
                ),
            }),
            7,
        ),
        question(
            "observation_sellability_state",
            "Execution state",
            "How often are active observations economically buyable and sellable?",
            format!("{} economic", fmt_count(economic_buy_sell)),
            format!(
                "{} active rows are economically buyable and sellable out of {} active observation rows.",
                fmt_count(economic_buy_sell),
                fmt_count(active_observation_rows)
            ),
            Some("active rows"),
            Some(active_observation_rows),
            json!({
                "rows": distributions.rows_for_section("observation_trading_state"),
                "use": usage(
                    "Sellability state controls whether a warning can actually be acted on.",
                    "We use effective buy/sell state in the simulator and should split high-tax sells from impossible sells."
                ),
            }),
            8,
        ),
        question(
            "row_level_model_rows",
            "Model rows",
            "How much row-level data is available for training?",
            format!("{} active rows", fmt_count(active_observation_rows)),
            format!(
                "Risk Atlas stores one row per active token/pool observation with as-of features and Risk Atlas near-future target columns."
            ),
            Some("active rows"),
            Some(active_observation_rows),
            json!({
                "row_kinds": active_targets.iter().filter(|row| row.active_observation_delta.is_none()).collect::<Vec<_>>(),
                "use": usage(
                    "This is the training surface for probabilistic risk models.",
                    "We use only as-of observation features for model inputs; future labels are added only as target columns."
                ),
            }),
            9,
        ),
        question(
            "current_trading_use",
            "Trading use",
            "How are we using these answers right now?",
            "risk gate first".to_string(),
            "The current use is to replay eligible V2 pools, block entries after mined LP approval, exit on LP approval or removal, and sweep active-hold windows. Strategy PnL attribution stays separate from this strategy-neutral atlas.".to_string(),
            None,
            None,
            json!({
                "use": usage(
                    "The page should explain which signals are warnings, which are labels, and which are model targets.",
                    "We are using LP approval as a risk gate/exit cap and active_observation_delta timing to choose hold windows; we are not treating LP approval as the standalone profit driver."
                ),
            }),
            10,
        ),
    ]
}

pub(super) fn model_readiness() -> Vec<ModelReadinessItem> {
    vec![
        ModelReadinessItem {
            name: "row_level_observation_export".to_string(),
            status: "ready".to_string(),
            detail: Some(
                "Range runs now export eth_token::token_analytics observations into Risk Atlas."
                    .to_string(),
            ),
            sort_order: 0,
        },
        ModelReadinessItem {
            name: "direct_lp_active_targets".to_string(),
            status: "ready".to_string(),
            detail: Some(
                "Targets are active_observation_delta based for 1, 2, 3, 5, and 10.".to_string(),
            ),
            sort_order: 1,
        },
    ]
}

fn question(
    question_id: &str,
    category: &str,
    question_text: &str,
    headline: String,
    answer: String,
    denominator_label: Option<&str>,
    denominator_count: Option<i64>,
    payload: Value,
    sort_order: i32,
) -> DecisionQuestion {
    DecisionQuestion {
        question_id: question_id.to_string(),
        category: category.to_string(),
        question: question_text.to_string(),
        headline: Some(headline),
        answer: Some(answer),
        status: "answered".to_string(),
        denominator_label: denominator_label.map(str::to_string),
        denominator_count,
        payload,
        sort_order,
    }
}

fn target_headline(target: Option<&ActiveTargetSummary>) -> String {
    let Some(target) = target else {
        return "no target rows".to_string();
    };
    let positives = target.positives.unwrap_or_default();
    format!("{} positives", fmt_count(positives))
}

fn target_answer(target: Option<&ActiveTargetSummary>, horizon: i32) -> String {
    let Some(target) = target else {
        return format!("No direct LP target rows for horizon {horizon}.");
    };
    let positives = target.positives.unwrap_or_default();
    let rows = target.rows.max(0);
    format!(
        "{} of {} eligible pre-removal observations are positive within {horizon} active_observation_delta ({}).",
        fmt_count(positives),
        fmt_count(rows),
        fmt_ratio_percent(positives, rows)
    )
}

fn usage(can_be_used_for: &str, current_use: &str) -> Value {
    json!({
        "can_be_used_for": can_be_used_for,
        "current_use": current_use,
    })
}

#[derive(Default)]
pub(super) struct DistributionAccumulator {
    counts: BTreeMap<(String, String), i64>,
}

impl DistributionAccumulator {
    pub(super) fn add(
        &mut self,
        section: impl Into<String>,
        bucket: impl Into<String>,
        count: i64,
    ) {
        if count == 0 {
            return;
        }
        *self
            .counts
            .entry((section.into(), bucket.into()))
            .or_default() += count;
    }

    pub(super) fn finish(self) -> Vec<DistributionBucket> {
        let totals = self.section_totals();
        let mut rows = self
            .counts
            .into_iter()
            .map(|((section, bucket), count)| DistributionBucket {
                share: totals.get(&section).and_then(|total| {
                    if *total > 0 {
                        Some(count as f64 / *total as f64)
                    } else {
                        None
                    }
                }),
                section,
                bucket,
                count,
                sort_order: 0,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.section
                .cmp(&right.section)
                .then_with(|| right.count.cmp(&left.count))
                .then_with(|| left.bucket.cmp(&right.bucket))
        });
        for (index, row) in rows.iter_mut().enumerate() {
            row.sort_order = index as i32;
        }
        rows
    }

    fn rows_for_section(&self, section: &str) -> Vec<Value> {
        let total = self
            .counts
            .iter()
            .filter(|((candidate, _), _)| candidate == section)
            .map(|(_, count)| *count)
            .sum::<i64>();
        let mut rows = self
            .counts
            .iter()
            .filter(|((candidate, _), _)| candidate == section)
            .map(|((_, bucket), count)| {
                json!({
                    "bucket": bucket,
                    "count": count,
                    "share": if total > 0 { Some(*count as f64 / total as f64) } else { None },
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right
                .get("count")
                .and_then(Value::as_i64)
                .cmp(&left.get("count").and_then(Value::as_i64))
        });
        rows
    }

    fn count_for_section(&self, section: &str, bucket: &str) -> i64 {
        self.counts
            .get(&(section.to_string(), bucket.to_string()))
            .copied()
            .unwrap_or_default()
    }

    fn section_total(&self, section: &str) -> i64 {
        self.counts
            .iter()
            .filter(|((candidate, _), _)| candidate == section)
            .map(|(_, count)| *count)
            .sum()
    }

    fn section_totals(&self) -> BTreeMap<String, i64> {
        let mut totals = BTreeMap::new();
        for ((section, _), count) in &self.counts {
            *totals.entry(section.clone()).or_default() += *count;
        }
        totals
    }
}

pub(super) fn eligible_bucket(eligible: bool) -> &'static str {
    if eligible {
        "eligible"
    } else {
        "ineligible"
    }
}

pub(super) fn pool_key(token_address: &str, pool_address: &str) -> (String, String) {
    (
        token_address.trim().to_ascii_lowercase(),
        pool_address.trim().to_ascii_lowercase(),
    )
}

pub(super) fn ineligible_reason(pool: &PoolView) -> String {
    pool.pool_classification
        .reason_key
        .unwrap_or("unknown")
        .to_string()
}

fn trading_bucket(row: &ObservationRow) -> &'static str {
    if row.liquidity_removed_as_of {
        "liquidity_removed"
    } else if row.effective_can_buy && row.effective_can_sell {
        "economic_buy_sell"
    } else if row.can_buy && row.can_sell {
        "simulated_buy_sell"
    } else if row.can_buy {
        "can_buy_only"
    } else {
        "not_tradeable"
    }
}

fn primary_reason(row: &ObservationRow) -> String {
    row.active_reasons
        .as_array()
        .and_then(|reasons| reasons.first())
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}

pub(super) fn time_bucket(age: f64) -> &'static str {
    if age <= 1.0 {
        "0_1_blocks"
    } else if age <= 10.0 {
        "2_10_blocks"
    } else if age <= 50.0 {
        "11_50_blocks"
    } else if age <= 100.0 {
        "51_100_blocks"
    } else if age <= 500.0 {
        "101_500_blocks"
    } else {
        "gt_500_blocks"
    }
}

fn percentile(values: &[f64], q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let index = ((values.len().saturating_sub(1)) as f64 * q).round() as usize;
    values.get(index).copied()
}

pub(super) fn finite_opt(value: f64) -> Option<f64> {
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
}

pub(super) fn finite_opt_option(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

pub(super) fn i64_from_u64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

pub(super) fn i64_from_u64_opt(value: u64) -> Option<i64> {
    i64::try_from(value).ok()
}

fn fmt_count(value: i64) -> String {
    value.to_string()
}

fn fmt_optional(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_ratio_percent(part: i64, whole: i64) -> String {
    if whole <= 0 {
        return "-".to_string();
    }
    format!("{:.2}%", part as f64 / whole as f64 * 100.0)
}
