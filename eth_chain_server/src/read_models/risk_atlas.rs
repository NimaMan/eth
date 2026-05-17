use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use eth_token::pools::SCAM_DIRECT_LP_LIQUIDITY_REMOVAL;
use eth_token::token_analytics::{TokenPoolCurrentObservation, ACTIVE_OBSERVATION_TARGET_HORIZONS};
use serde_json::{json, to_value, Value};
use token_lab_scam_risk_atlas::ingest::report::RiskAtlasReportImport;
use token_lab_scam_risk_atlas::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, ModelReadinessItem, NumericStat,
    ObservationRow, PoolEligibilityRow, RiskAtlasRun,
};

use crate::ranges::RangeIndexJob;
use crate::read_models::pool::PoolView;

const RISK_ATLAS_SOURCE_KIND: &str = "range_token_analytics";

pub async fn range_import(run: &RangeIndexJob) -> eyre::Result<RiskAtlasReportImport> {
    let state = run.state.read().await;
    let mut distributions = DistributionAccumulator::default();
    let mut pool_eligibility = Vec::new();
    let mut scam_ages = Vec::new();
    let mut direct_lp_approval_leads = Vec::new();
    let mut direct_lp_rows = 0_i64;
    let mut scam_count = 0_i64;
    let mut eligible_count = 0_i64;
    let mut ineligible_count = 0_i64;
    let mut pool_count = 0_i64;
    let mut eligible_pools = BTreeSet::<(String, String)>::new();
    let mut direct_lp_removal_blocks = BTreeMap::<(String, String), u64>::new();

    for token in state.processor.registry.tokens.values() {
        for pool in PoolView::from_token_pool_summaries(token) {
            pool_count += 1;
            let eligible = pool.pool_classification.eligible;
            if eligible {
                eligible_count += 1;
                eligible_pools.insert(pool_key(&pool.token_address, &pool.pool_address));
            } else {
                ineligible_count += 1;
            }
            if pool.is_scam {
                scam_count += 1;
            }

            distributions.add("pool_eligibility", eligible_bucket(eligible), 1);
            distributions.add("all_pool_protocols", pool.protocol.clone(), 1);
            distributions.add("all_pool_quotes", pool.currency.clone(), 1);
            distributions.add(
                "pool_ineligible_reasons",
                ineligible_reason(&pool),
                if eligible { 0 } else { 1 },
            );

            if eligible {
                distributions.add("eligible_protocols", pool.protocol.clone(), 1);
                distributions.add("eligible_quotes", pool.currency.clone(), 1);
                distributions.add("eligible_liquidity_levels", pool.liquidity_label.clone(), 1);
                distributions.add("eligible_tax_buckets", format!("{:?}", pool.tax_bucket), 1);
            }

            if pool.is_scam {
                distributions.add("scam_protocols", pool.protocol.clone(), 1);
                distributions.add(
                    "scam_mechanisms",
                    pool.scam_mechanism
                        .clone()
                        .or_else(|| pool.scam_label.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                    1,
                );
                distributions.add("scam_liquidity_at_label", pool.liquidity_label.clone(), 1);
                if let (Some(label_block), Some(trading_block)) =
                    (pool.liquidity_removal_block, pool.can_buy_block)
                {
                    let age = label_block.saturating_sub(trading_block) as f64;
                    scam_ages.push(age);
                    distributions.add("time_to_scam_buckets", time_bucket(age), 1);
                }
            }

            if pool.scam_mechanism.as_deref() == Some(SCAM_DIRECT_LP_LIQUIDITY_REMOVAL) {
                direct_lp_rows += 1;
                if let Some(block) = pool.liquidity_removal_block {
                    direct_lp_removal_blocks
                        .insert(pool_key(&pool.token_address, &pool.pool_address), block);
                }
                let pre_approval = pool
                    .lp_last_approval_block
                    .zip(pool.liquidity_removal_block)
                    .is_some_and(|(approval, removal)| approval < removal);
                distributions.add(
                    "direct_lp_approval_pre_removal",
                    pre_approval.to_string(),
                    1,
                );
                if let (Some(approval), Some(removal)) =
                    (pool.lp_last_approval_block, pool.liquidity_removal_block)
                {
                    if approval < removal {
                        direct_lp_approval_leads.push(removal.saturating_sub(approval) as f64);
                    }
                }
            }

            pool_eligibility.push(PoolEligibilityRow {
                token_address: pool.token_address,
                pool_address: pool.pool_address,
                protocol: Some(pool.protocol),
                quote_symbol: Some(pool.currency),
                eligible,
                eligibility_block: pool.can_buy_block.map(i64_from_u64),
                eligibility_liquidity: Some(pool.total_liquidity),
                first_observed_block: pool.creation_block.map(i64_from_u64),
                last_observed_block: pool.latest_block_number.map(i64_from_u64),
            });
        }
    }

    let observation_rows =
        observation_rows(&state.observations, &eligible_pools, &direct_lp_removal_blocks)?;
    append_observation_distributions(&mut distributions, &observation_rows);
    let active_targets = active_target_summaries(&observation_rows);
    let numeric_stats = numeric_stats(scam_ages, direct_lp_approval_leads);
    let decision_questions = decision_questions(
        pool_count,
        eligible_count,
        ineligible_count,
        scam_count,
        direct_lp_rows,
        &active_targets,
        &numeric_stats,
        &distributions,
    );

    let run_row = RiskAtlasRun {
        run_id: format!("risk-atlas-{}", run.id),
        chain: "ethereum".to_string(),
        source_kind: RISK_ATLAS_SOURCE_KIND.to_string(),
        source_ref: Some(run.id.clone()),
        start_block: Some(i64_from_u64(run.request.start_block)),
        end_block: Some(i64_from_u64(run.request.end_block)),
        block_count: Some(i64_from_u64(run.request.block_count())),
        token_count: Some(state.processor.registry.tokens.len() as i64),
        pool_count: Some(pool_count),
        scam_label_count: Some(scam_count),
        direct_lp_feature_row_count: Some(direct_lp_rows),
        active_observation_row_count: Some(observation_rows.len() as i64),
        status: format!("{:?}", state.progress.status).to_ascii_lowercase(),
        generated_at: Utc::now(),
        metadata: json!({
            "source_run_id": run.id,
            "row_source": "eth_token::token_analytics::TokenPoolCurrentObservation",
            "eligible_pools": eligible_count,
            "ineligible_pools": ineligible_count,
            "target_horizons_active_observations": ACTIVE_OBSERVATION_TARGET_HORIZONS,
        }),
    };

    Ok(RiskAtlasReportImport {
        run: run_row,
        distributions: distributions.finish(),
        pool_eligibility,
        observations: observation_rows,
        numeric_stats,
        active_targets,
        decision_questions,
        review_examples: Vec::new(),
        model_readiness: model_readiness(),
    })
}

fn observation_rows(
    observations: &[TokenPoolCurrentObservation],
    eligible_pools: &BTreeSet<(String, String)>,
    direct_lp_removal_blocks: &BTreeMap<(String, String), u64>,
) -> eyre::Result<Vec<ObservationRow>> {
    let mut groups = BTreeMap::<(String, String), Vec<&TokenPoolCurrentObservation>>::new();
    for observation in observations {
        groups
            .entry((
                observation.key.token_address.clone(),
                observation.key.pool_address.clone(),
            ))
            .or_default()
            .push(observation);
    }

    let mut rows = Vec::with_capacity(observations.len());
    for (pool_key, mut group) in groups {
        group.sort_by_key(|observation| observation.context.active_observation_index);
        let eligible = eligible_pools.contains(&pool_key);
        let direct_lp_removal_block = direct_lp_removal_blocks.get(&pool_key).copied();
        let direct_lp_removal_index = direct_lp_removal_block.and_then(|removal_block| {
            group
                .iter()
                .find(|observation| observation.context.block_number >= removal_block)
                .map(|observation| observation.context.active_observation_index)
        });

        for observation in group {
            let index = observation.context.active_observation_index;
            let target = |horizon: u16| {
                if !eligible {
                    return None;
                }
                match direct_lp_removal_index {
                    Some(removal_index) if index < removal_index => {
                        Some(removal_index - index <= u64::from(horizon))
                    }
                    Some(_) => None,
                    None => Some(false),
                }
            };
            let direct_lp_as_of = direct_lp_removal_block
                .is_some_and(|removal_block| observation.context.block_number >= removal_block);
            let direct_lp_in_block = direct_lp_removal_block
                .is_some_and(|removal_block| observation.context.block_number == removal_block);
            rows.push(ObservationRow {
                token_address: observation.key.token_address.clone(),
                pool_address: observation.key.pool_address.clone(),
                denom_address: observation.key.denom_address.clone(),
                protocol: observation.key.protocol.clone(),
                active_observation_index: i64_from_u64(index),
                block_number: i64_from_u64(observation.context.block_number),
                timestamp: observation.context.timestamp.map(i64_from_u64),
                active_reasons: to_value(&observation.context.active_reasons)?,
                tx_count: observation.activity.tx_count as i32,
                token_transfer_count: observation.activity.token_transfer_count as i32,
                denom_transfer_count: observation.activity.denom_transfer_count as i32,
                buy_volume_denom: finite_opt(observation.activity.buy_volume_for_denom(
                    &observation.key.denom_address,
                )),
                sell_volume_denom: finite_opt(observation.activity.sell_volume_for_denom(
                    &observation.key.denom_address,
                )),
                total_bribe_eth: finite_opt(observation.activity.total_bribe_eth),
                can_buy: observation.trading.can_buy,
                can_sell: observation.trading.can_sell,
                effective_can_buy: observation.trading.effective_can_buy,
                effective_can_sell: observation.trading.effective_can_sell,
                buy_tax: finite_opt_option(observation.trading.buy_tax),
                sell_tax: finite_opt_option(observation.trading.sell_tax),
                liquidity_removed_as_of: observation.trading.liquidity_removed_as_of,
                liquidity_removal_in_block: observation.event_flags.liquidity_removal_in_block,
                liquidity_removal_block_as_of: observation
                    .trading
                    .liquidity_removal_block_as_of
                    .map(i64_from_u64),
                direct_lp_removal_as_of: direct_lp_as_of,
                direct_lp_removal_in_block: direct_lp_in_block,
                direct_lp_target_1: target(1),
                direct_lp_target_2: target(2),
                direct_lp_target_3: target(3),
                direct_lp_target_5: target(5),
                direct_lp_target_10: target(10),
                denom_reserve: finite_opt(observation.features.liquidity.denom_reserve),
                token_reserve: finite_opt(observation.features.liquidity.token_reserve),
                total_liquidity_denom: finite_opt(
                    observation.features.liquidity.total_liquidity_denom,
                ),
                price_to_initial_ratio: finite_opt_option(
                    observation.features.liquidity.price_to_initial_ratio,
                ),
                lp_approved_pct_as_of: finite_opt_option(
                    observation.features.lp_control.lp_approved_pct_as_of,
                ),
                token_transfer_to_total_supply_ratio: finite_opt_option(
                    observation
                        .features
                        .activity
                        .block_token_transfer_to_total_supply_ratio,
                ),
                token_transfer_to_pool_token_reserve_ratio: finite_opt_option(
                    observation
                        .features
                        .activity
                        .block_token_transfer_to_pool_token_reserve_ratio,
                ),
                observation: to_value(observation)?,
                features: to_value(&observation.features)?,
            });
        }
    }

    Ok(rows)
}

fn active_target_summaries(rows: &[ObservationRow]) -> Vec<ActiveTargetSummary> {
    let mut summaries = Vec::new();
    let unique_pools = rows
        .iter()
        .map(|row| (row.token_address.clone(), row.pool_address.clone()))
        .collect::<BTreeSet<_>>()
        .len() as i64;
    summaries.push(ActiveTargetSummary {
        row_kind: "active_pool_observation".to_string(),
        horizon_active_observations: None,
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
            row_kind: "direct_lp_removal_within_active_observations".to_string(),
            horizon_active_observations: Some(horizon),
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

fn append_observation_distributions(
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
                    format!("direct_lp_target_{horizon}_active_observations"),
                    value.to_string(),
                    1,
                );
            }
        }
    }
}

fn numeric_stats(scam_ages: Vec<f64>, direct_lp_approval_leads: Vec<f64>) -> Vec<NumericStat> {
    let mut stats = Vec::new();
    if let Some(stat) = numeric_stat(
        "scam_age",
        "Trading Enabled To Label Blocks",
        scam_ages,
        0,
    ) {
        stats.push(stat);
    }
    if let Some(stat) = numeric_stat(
        "direct_lp_age_approval",
        "Last Pre-Removal LP Approval To Removal Blocks",
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

fn decision_questions(
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
        .find(|row| row.horizon_active_observations == Some(1));
    let target_10 = active_targets
        .iter()
        .find(|row| row.horizon_active_observations == Some(10));
    let scam_age = numeric_stats
        .iter()
        .find(|row| row.section == "scam_age");
    vec![
        question(
            "eligible_pool_filter",
            "Launch filter",
            "How much of the observed pool universe is eligible for analytics and training?",
            format!("{} eligible", fmt_count(eligible_count)),
            format!(
                "{} of {} pools are eligible. {} are filtered before scam analytics.",
                fmt_count(eligible_count),
                fmt_count(pool_count),
                fmt_count(ineligible_count)
            ),
            Some("pool rows"),
            Some(pool_count),
            json!({
                "rows": distributions.rows_for_section("pool_eligibility"),
                "buckets": distributions.rows_for_section("pool_ineligible_reasons"),
            }),
            0,
        ),
        question(
            "scam_mechanism_mix",
            "Scam labels",
            "Which scam mechanisms dominate the eligible scam labels?",
            format!("{} scam labels", fmt_count(scam_count)),
            format!(
                "{} pools are currently scam-labeled; {} are direct LP removals.",
                fmt_count(scam_count),
                fmt_count(direct_lp_count)
            ),
            Some("scam labels"),
            Some(scam_count),
            json!({ "rows": distributions.rows_for_section("scam_mechanisms") }),
            1,
        ),
        question(
            "scam_time_from_trading_enabled",
            "Timing",
            "How fast do scam pools fail after trading becomes enabled?",
            scam_age
                .and_then(|stat| stat.median)
                .map(|median| format!("{median:.0} blocks median"))
                .unwrap_or_else(|| "no timing rows".to_string()),
            scam_age
                .map(|stat| {
                    format!(
                        "Median label age is {}; P90 is {}.",
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
            }),
            2,
        ),
        question(
            "direct_lp_next_observation_target",
            "Near-future target",
            "How often does direct LP removal happen in the next active observation?",
            target_headline(target_1),
            target_answer(target_1, 1),
            Some("target rows"),
            target_1.map(|row| row.rows),
            json!({ "active_target": target_1 }),
            3,
        ),
        question(
            "direct_lp_next_10_observations_target",
            "Near-future target",
            "How often does direct LP removal happen within the next 10 active observations?",
            target_headline(target_10),
            target_answer(target_10, 10),
            Some("target rows"),
            target_10.map(|row| row.rows),
            json!({ "active_target": target_10 }),
            4,
        ),
    ]
}

fn model_readiness() -> Vec<ModelReadinessItem> {
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
            detail: Some("Targets are active-observation based for 1, 2, 3, 5, and 10.".to_string()),
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
        "{} of {} eligible pre-removal observations are positive within {horizon} active observations.",
        fmt_count(positives),
        fmt_count(rows)
    )
}

#[derive(Default)]
struct DistributionAccumulator {
    counts: BTreeMap<(String, String), i64>,
}

impl DistributionAccumulator {
    fn add(
        &mut self,
        section: impl Into<String>,
        bucket: impl Into<String>,
        count: i64,
    ) {
        if count == 0 {
            return;
        }
        *self.counts.entry((section.into(), bucket.into())).or_default() += count;
    }

    fn finish(self) -> Vec<DistributionBucket> {
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

    fn section_totals(&self) -> BTreeMap<String, i64> {
        let mut totals = BTreeMap::new();
        for ((section, _), count) in &self.counts {
            *totals.entry(section.clone()).or_default() += *count;
        }
        totals
    }
}

fn eligible_bucket(eligible: bool) -> &'static str {
    if eligible {
        "eligible"
    } else {
        "ineligible"
    }
}

fn pool_key(token_address: &str, pool_address: &str) -> (String, String) {
    (
        token_address.trim().to_ascii_lowercase(),
        pool_address.trim().to_ascii_lowercase(),
    )
}

fn ineligible_reason(pool: &PoolView) -> String {
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

fn time_bucket(age: f64) -> &'static str {
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

fn finite_opt(value: f64) -> Option<f64> {
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
}

fn finite_opt_option(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

fn i64_from_u64(value: u64) -> i64 {
    value.min(i64::MAX as u64) as i64
}

fn fmt_count(value: i64) -> String {
    value.to_string()
}

fn fmt_optional(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.0}"))
        .unwrap_or_else(|| "-".to_string())
}
