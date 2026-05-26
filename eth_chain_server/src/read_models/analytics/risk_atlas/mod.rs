use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use alloy_primitives::B256;
use chrono::Utc;
use eth_risk_atlas::ingest::report::RiskAtlasReportImport;
use eth_risk_atlas::{EventEvidenceRow, ObservationRow, PoolEligibilityRow, RiskAtlasRun};
use eth_token::pools::SCAM_DIRECT_LP_LIQUIDITY_REMOVAL;
use eth_token::token_analytics::{TokenPoolCurrentObservation, ACTIVE_OBSERVATION_TARGET_HORIZONS};
use reth_chain_query::RethQueryProvider;
use serde_json::{json, to_value, Value};

use crate::ranges::RangeIndexJob;
use crate::read_models::pool::PoolView;

use summary::{
    active_target_summaries, append_observation_distributions, decision_questions, eligible_bucket,
    finite_opt, finite_opt_option, i64_from_u64, i64_from_u64_opt, ineligible_reason,
    model_readiness, numeric_stats, pool_key, time_bucket, DistributionAccumulator,
};

mod summary;

const RISK_ATLAS_SOURCE_KIND: &str = "range_token_analytics";

#[derive(Clone, Debug)]
struct ScamEventMeta {
    mechanism: Option<String>,
    block_number: Option<u64>,
    tx_hash: Option<String>,
}

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
    let mut seen_pool_keys = BTreeSet::<(String, String)>::new();
    let mut eligible_pools = BTreeSet::<(String, String)>::new();
    let mut direct_lp_removal_blocks = BTreeMap::<(String, String), u64>::new();
    let mut scam_events = BTreeMap::<(String, String), ScamEventMeta>::new();

    for token in state.processor.registry.tokens.values() {
        for pool in PoolView::from_token_pool_summaries(token) {
            let key = pool_key(&pool.token_address, &pool.pool_address);
            seen_pool_keys.insert(key.clone());
            pool_count += 1;
            let eligible = pool.pool_classification.eligible;
            if eligible {
                eligible_count += 1;
                eligible_pools.insert(key.clone());
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
                    let bucket = time_bucket(age);
                    scam_ages.push(age);
                    distributions.add("time_to_scam_buckets", bucket, 1);
                    distributions.add(
                        "time_to_scam_by_protocol",
                        format!("{}|{}", pool.protocol, bucket),
                        1,
                    );
                }
                scam_events.insert(
                    key.clone(),
                    ScamEventMeta {
                        mechanism: pool
                            .scam_mechanism
                            .clone()
                            .or_else(|| pool.scam_label.clone()),
                        block_number: pool.liquidity_removal_block,
                        tx_hash: pool.liquidity_removal_tx_hash.clone(),
                    },
                );
            }

            if pool.scam_mechanism.as_deref() == Some(SCAM_DIRECT_LP_LIQUIDITY_REMOVAL) {
                direct_lp_rows += 1;
                if let Some(block) = pool.liquidity_removal_block {
                    direct_lp_removal_blocks.insert(key.clone(), block);
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
    append_observation_only_pool_summaries(
        &state.observations,
        &seen_pool_keys,
        &mut distributions,
        &mut pool_eligibility,
        &mut eligible_pools,
        &mut direct_lp_removal_blocks,
        &mut scam_events,
        &mut scam_ages,
        &mut direct_lp_approval_leads,
        &mut pool_count,
        &mut eligible_count,
        &mut ineligible_count,
        &mut scam_count,
        &mut direct_lp_rows,
    );

    let observation_rows = observation_rows(
        &state.observations,
        &eligible_pools,
        &direct_lp_removal_blocks,
        &scam_events,
    )?;
    let (observation_rows, event_evidence) = observation_rows;
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
            "target_active_observation_delta": ACTIVE_OBSERVATION_TARGET_HORIZONS,
        }),
    };

    Ok(RiskAtlasReportImport {
        run: run_row,
        distributions: distributions.finish(),
        pool_eligibility,
        event_evidence,
        observations: observation_rows,
        numeric_stats,
        active_targets,
        decision_questions,
        review_examples: Vec::new(),
        model_readiness: model_readiness(),
    })
}

pub fn attach_mempool_arrivals(
    import: &mut RiskAtlasReportImport,
    provider: &RethQueryProvider,
) -> eyre::Result<()> {
    for event in &mut import.event_evidence {
        let Some(tx_hash) = event.tx_hash.as_deref() else {
            continue;
        };
        let Ok(hash) = B256::from_str(tx_hash) else {
            continue;
        };
        if let Ok(arrival_ms) = provider.get_tx_arrival_ms(hash) {
            event.mempool_first_seen_ms = arrival_ms.and_then(i64_from_u64_opt);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_observation_only_pool_summaries(
    observations: &[TokenPoolCurrentObservation],
    seen_pool_keys: &BTreeSet<(String, String)>,
    distributions: &mut DistributionAccumulator,
    pool_eligibility: &mut Vec<PoolEligibilityRow>,
    eligible_pools: &mut BTreeSet<(String, String)>,
    direct_lp_removal_blocks: &mut BTreeMap<(String, String), u64>,
    scam_events: &mut BTreeMap<(String, String), ScamEventMeta>,
    scam_ages: &mut Vec<f64>,
    direct_lp_approval_leads: &mut Vec<f64>,
    pool_count: &mut i64,
    eligible_count: &mut i64,
    ineligible_count: &mut i64,
    scam_count: &mut i64,
    direct_lp_rows: &mut i64,
) {
    let mut groups = BTreeMap::<(String, String), Vec<&TokenPoolCurrentObservation>>::new();
    for observation in observations {
        let key = pool_key(
            &observation.key.token_address,
            &observation.key.pool_address,
        );
        if seen_pool_keys.contains(&key) {
            continue;
        }
        groups.entry(key).or_default().push(observation);
    }

    for (key, mut group) in groups {
        group.sort_by_key(|observation| observation.context.block_number);
        let Some(latest) = group.last().copied() else {
            continue;
        };
        let first = group.first().copied().unwrap_or(latest);
        let eligible = group.iter().any(|observation| {
            observation.trading.can_buy
                || observation.trading.effective_can_buy
                || observation.event_flags.trading_enabled_in_block
                || observation.features.market.trading_enabled_block.is_some()
        });
        let scam_observation = group.iter().rev().copied().find(|observation| {
            observation.event_flags.liquidity_removal_in_block
                || observation.trading.liquidity_removed_as_of
                || observation.features.market.liquidity_removed_as_of
        });
        let scam_mechanism = scam_observation.and_then(observation_scam_mechanism);
        let scam_block = scam_observation
            .and_then(|observation| observation.trading.liquidity_removal_block_as_of)
            .or_else(|| {
                scam_observation
                    .filter(|observation| observation.event_flags.liquidity_removal_in_block)
                    .map(|observation| observation.context.block_number)
            });

        *pool_count += 1;
        if eligible {
            *eligible_count += 1;
            eligible_pools.insert(key.clone());
        } else {
            *ineligible_count += 1;
        }
        distributions.add("pool_eligibility", eligible_bucket(eligible), 1);
        distributions.add("all_pool_protocols", latest.key.protocol.clone(), 1);
        distributions.add("all_pool_quotes", latest.key.denom_address.clone(), 1);
        distributions.add(
            "pool_ineligible_reasons",
            if eligible {
                "eligible"
            } else {
                "observation_only_unknown_eligibility"
            },
            if eligible { 0 } else { 1 },
        );
        if eligible {
            distributions.add("eligible_protocols", latest.key.protocol.clone(), 1);
            distributions.add("eligible_quotes", latest.key.denom_address.clone(), 1);
            distributions.add("eligible_liquidity_levels", "observation_only", 1);
            distributions.add("eligible_tax_buckets", "observation_only", 1);
        }

        if let Some(scam_block) = scam_block {
            *scam_count += 1;
            let mechanism = scam_mechanism.unwrap_or_else(|| "unknown".to_string());
            distributions.add("scam_protocols", latest.key.protocol.clone(), 1);
            distributions.add("scam_mechanisms", mechanism.clone(), 1);
            distributions.add("scam_liquidity_at_label", "observation_only", 1);
            if let Some(trading_block) = latest.features.market.trading_enabled_block {
                let age = scam_block.saturating_sub(trading_block) as f64;
                let bucket = time_bucket(age);
                scam_ages.push(age);
                distributions.add("time_to_scam_buckets", bucket, 1);
                distributions.add(
                    "time_to_scam_by_protocol",
                    format!("{}|{}", latest.key.protocol, bucket),
                    1,
                );
            }
            scam_events.insert(
                key.clone(),
                ScamEventMeta {
                    mechanism: Some(mechanism.clone()),
                    block_number: Some(scam_block),
                    tx_hash: None,
                },
            );

            if mechanism == SCAM_DIRECT_LP_LIQUIDITY_REMOVAL {
                *direct_lp_rows += 1;
                direct_lp_removal_blocks.insert(key.clone(), scam_block);
                let pre_approval = latest
                    .features
                    .lp_control
                    .last_lp_approval_block
                    .is_some_and(|approval| approval < scam_block);
                distributions.add(
                    "direct_lp_approval_pre_removal",
                    pre_approval.to_string(),
                    1,
                );
                if pre_approval {
                    if let Some(lead) = latest
                        .features
                        .lp_control
                        .last_lp_approval_to_as_of_chain_block_delta
                    {
                        direct_lp_approval_leads.push(lead as f64);
                    }
                }
            }
        }

        pool_eligibility.push(PoolEligibilityRow {
            token_address: latest.key.token_address.clone(),
            pool_address: latest.key.pool_address.clone(),
            protocol: Some(latest.key.protocol.clone()),
            quote_symbol: Some(latest.key.denom_address.clone()),
            eligible,
            eligibility_block: group
                .iter()
                .filter(|observation| {
                    observation.trading.can_buy
                        || observation.trading.effective_can_buy
                        || observation.event_flags.trading_enabled_in_block
                })
                .map(|observation| observation.context.block_number)
                .min()
                .map(i64_from_u64),
            eligibility_liquidity: finite_opt(latest.features.liquidity.total_liquidity_denom),
            first_observed_block: first
                .features
                .market
                .pool_creation_block
                .or(Some(first.context.block_number))
                .map(i64_from_u64),
            last_observed_block: Some(i64_from_u64(latest.context.block_number)),
        });
    }
}

fn observation_scam_mechanism(observation: &TokenPoolCurrentObservation) -> Option<String> {
    observation
        .features
        .market
        .scam_mechanism_as_of
        .clone()
        .or_else(|| observation.features.market.scam_label_as_of.clone())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

fn observation_rows(
    observations: &[TokenPoolCurrentObservation],
    eligible_pools: &BTreeSet<(String, String)>,
    direct_lp_removal_blocks: &BTreeMap<(String, String), u64>,
    scam_events: &BTreeMap<(String, String), ScamEventMeta>,
) -> eyre::Result<(Vec<ObservationRow>, Vec<EventEvidenceRow>)> {
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
    let mut event_evidence = Vec::new();
    for (pool_key, mut group) in groups {
        group.sort_by_key(|observation| observation.context.active_observation_index);
        let eligible = eligible_pools.contains(&pool_key);
        let direct_lp_removal_block = direct_lp_removal_blocks.get(&pool_key).copied();
        let scam_event = scam_events.get(&pool_key);
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
            append_event_evidence_rows(&mut event_evidence, observation, scam_event);
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
                buy_volume_denom: finite_opt(
                    observation
                        .activity
                        .buy_volume_for_denom(&observation.key.denom_address),
                ),
                sell_volume_denom: finite_opt(
                    observation
                        .activity
                        .sell_volume_for_denom(&observation.key.denom_address),
                ),
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
                pooled_token_supply_ratio: finite_opt_option(
                    observation.features.liquidity.pooled_token_supply_ratio,
                ),
                reserve_quality_status: observation
                    .features
                    .liquidity
                    .reserve_quality_status
                    .clone(),
                price_to_initial_ratio_trustworthy: observation
                    .features
                    .liquidity
                    .price_to_initial_ratio_trustworthy,
                total_liquidity_denom: finite_opt(
                    observation.features.liquidity.total_liquidity_denom,
                ),
                price_to_initial_ratio: finite_opt_option(
                    observation.features.liquidity.price_to_initial_ratio,
                ),
                lp_approved_pct_as_of: finite_opt_option(
                    observation.features.lp_control.lp_approved_pct_as_of,
                ),
                token_decimals: observation.features.token.decimals.map(i32::from),
                price_denom_per_token: finite_opt(
                    observation.features.liquidity.price_denom_per_token,
                ),
                initial_price_denom_per_token: finite_opt_option(
                    observation.features.liquidity.initial_price_denom_per_token,
                ),
                lp_approval_count_in_block: observation.event_flags.lp_approval_count_in_block
                    as i32,
                lp_approval_seen_as_of: observation.features.lp_control.lp_approval_seen_as_of,
                lp_total_supply: finite_opt_option(observation.features.lp_control.lp_total_supply),
                lp_max_approval_amount_as_of: finite_opt_option(
                    observation.features.lp_control.lp_max_approval_amount_as_of,
                ),
                lp_max_approval_pct_as_of: finite_opt_option(
                    observation.features.lp_control.lp_max_approval_pct_as_of,
                ),
                lp_removable_pct_as_of: finite_opt_option(
                    observation.features.lp_control.lp_removable_pct_as_of,
                ),
                lp_router_removable_pct_as_of: finite_opt_option(
                    observation
                        .features
                        .lp_control
                        .lp_router_removable_pct_as_of,
                ),
                lp_approval_owner_is_creator: observation
                    .features
                    .lp_control
                    .last_lp_approval_owner_is_creator,
                creator_lp_balance_pct_as_of: finite_opt_option(
                    observation.features.lp_control.creator_lp_balance_pct_as_of,
                ),
                creator_lp_approved_pct_as_of: finite_opt_option(
                    observation
                        .features
                        .lp_control
                        .creator_lp_approved_pct_as_of,
                ),
                creator_lp_removable_pct_as_of: finite_opt_option(
                    observation
                        .features
                        .lp_control
                        .creator_lp_removable_pct_as_of,
                ),
                creator_lp_router_removable_pct_as_of: finite_opt_option(
                    observation
                        .features
                        .lp_control
                        .creator_lp_router_removable_pct_as_of,
                ),
                creator_lp_approved_gt_90_pct_as_of: observation
                    .features
                    .lp_control
                    .creator_lp_approved_gt_90_pct_as_of,
                creator_lp_router_removable_gt_90_pct_as_of: observation
                    .features
                    .lp_control
                    .creator_lp_router_removable_gt_90_pct_as_of,
                last_lp_approval_amount_pct_of_total_supply: finite_opt_option(
                    observation
                        .features
                        .lp_control
                        .last_lp_approval_amount_pct_of_total_supply,
                ),
                first_lp_approval_to_as_of_chain_block_delta: observation
                    .features
                    .lp_control
                    .first_lp_approval_to_as_of_chain_block_delta
                    .map(i64_from_u64),
                last_lp_approval_to_as_of_chain_block_delta: observation
                    .features
                    .lp_control
                    .last_lp_approval_to_as_of_chain_block_delta
                    .map(i64_from_u64),
                pool_creation_to_first_lp_approval_chain_block_delta: observation
                    .features
                    .lp_control
                    .pool_creation_to_first_lp_approval_chain_block_delta,
                pool_creation_to_last_lp_approval_chain_block_delta: observation
                    .features
                    .lp_control
                    .pool_creation_to_last_lp_approval_chain_block_delta,
                trading_enabled_to_first_lp_approval_chain_block_delta: observation
                    .features
                    .lp_control
                    .trading_enabled_to_first_lp_approval_chain_block_delta,
                trading_enabled_to_last_lp_approval_chain_block_delta: observation
                    .features
                    .lp_control
                    .trading_enabled_to_last_lp_approval_chain_block_delta,
                control_transfer_from_after_renounce_seen_as_of: observation
                    .features
                    .token_control
                    .control_transfer_from_after_renounce_seen_as_of,
                control_transfer_from_after_renounce_in_block: observation
                    .features
                    .token_control
                    .control_transfer_from_after_renounce_in_block,
                control_transfer_from_holder_to_burn_seen_as_of: observation
                    .features
                    .token_control
                    .control_transfer_from_holder_to_burn_seen_as_of,
                control_transfer_from_holder_to_burn_in_block: observation
                    .features
                    .token_control
                    .control_transfer_from_holder_to_burn_in_block,
                control_transfer_from_pair_seen_as_of: observation
                    .features
                    .token_control
                    .control_transfer_from_pair_seen_as_of,
                control_transfer_from_pair_in_block: observation
                    .features
                    .token_control
                    .control_transfer_from_pair_in_block,
                control_transfer_from_without_transfer_log_seen_as_of: observation
                    .features
                    .token_control
                    .control_transfer_from_without_transfer_log_seen_as_of,
                control_transfer_from_without_transfer_log_in_block: observation
                    .features
                    .token_control
                    .control_transfer_from_without_transfer_log_in_block,
                pair_token_to_control_seen_as_of: observation
                    .features
                    .token_control
                    .pair_token_to_control_seen_as_of,
                pair_token_to_control_in_block: observation
                    .features
                    .token_control
                    .pair_token_to_control_in_block,
                pair_token_to_control_to_pool_reserve_ratio: finite_opt_option(
                    observation
                        .features
                        .token_control
                        .pair_token_to_control_to_pool_reserve_ratio,
                ),
                pair_balance_backdoor_signal_seen_as_of: observation
                    .features
                    .token_control
                    .pair_balance_backdoor_signal_seen_as_of,
                pair_balance_backdoor_signal_in_block: observation
                    .features
                    .token_control
                    .pair_balance_backdoor_signal_in_block,
                last_pair_balance_backdoor_signal_to_as_of_chain_block_delta: observation
                    .features
                    .token_control
                    .last_pair_balance_backdoor_signal_to_as_of_chain_block_delta
                    .map(i64_from_u64),
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
                observation: compact_observation_json(observation),
                features: compact_features_json(),
            });
        }
    }

    for (index, row) in event_evidence.iter_mut().enumerate() {
        row.sort_order = index as i32;
    }

    Ok((rows, event_evidence))
}

fn compact_observation_json(observation: &TokenPoolCurrentObservation) -> Value {
    json!({
        "compact": true,
        "block_number": observation.context.block_number,
        "active_observation_index": observation.context.active_observation_index,
        "active_reasons": &observation.context.active_reasons,
        "protocol": &observation.key.protocol,
    })
}

fn compact_features_json() -> Value {
    json!({ "compact": true })
}

fn append_event_evidence_rows(
    rows: &mut Vec<EventEvidenceRow>,
    observation: &TokenPoolCurrentObservation,
    scam_event: Option<&ScamEventMeta>,
) {
    let block_number = Some(i64_from_u64(observation.context.block_number));
    let block_timestamp = observation.context.timestamp.map(i64_from_u64);

    for action in &observation.block_actions {
        let event_kind = match action.key.as_str() {
            "lp_approval" => "lp_approval",
            "token_control_transfer_from_pair"
            | "token_control_transfer_from_holder_to_burn"
            | "token_control_transfer_from_without_transfer"
            | "token_control_transfer_from_after_renounce"
            | "token_control_transfer_from" => "token_control_signal",
            _ => continue,
        };
        if action.tx_hashes.is_empty() {
            rows.push(event_evidence_row(
                observation,
                event_kind,
                None,
                block_number,
                block_timestamp,
                None,
                &action.key,
            ));
            continue;
        }
        for tx_hash in &action.tx_hashes {
            rows.push(event_evidence_row(
                observation,
                event_kind,
                None,
                block_number,
                block_timestamp,
                Some(tx_hash),
                &action.key,
            ));
        }
    }

    if let Some(scam) = scam_event {
        if scam.block_number == Some(observation.context.block_number) {
            rows.push(event_evidence_row(
                observation,
                "scam",
                scam.mechanism.as_deref(),
                block_number,
                block_timestamp,
                scam.tx_hash.as_deref(),
                "pool_scam_label",
            ));
        }
    }
}

fn event_evidence_row(
    observation: &TokenPoolCurrentObservation,
    event_kind: &str,
    mechanism: Option<&str>,
    block_number: Option<i64>,
    block_timestamp: Option<i64>,
    tx_hash: Option<&str>,
    source: &str,
) -> EventEvidenceRow {
    EventEvidenceRow {
        token_address: observation.key.token_address.clone(),
        pool_address: observation.key.pool_address.clone(),
        protocol: observation.key.protocol.clone(),
        event_kind: event_kind.to_string(),
        mechanism: mechanism
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty()),
        block_number,
        block_timestamp,
        tx_hash: tx_hash
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty()),
        mempool_first_seen_ms: None,
        source: Some(source.to_string()),
        sort_order: 0,
    }
}
