use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use alloy_primitives::B256;
use chrono::Utc;
use eth_risk_atlas::ingest::report::RiskAtlasReportImport;
use eth_risk_atlas::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, EventEvidenceRow,
    ModelReadinessItem, NumericStat, ObservationRow, PoolEligibilityRow, RiskAtlasRun,
};
use eth_token::pools::SCAM_DIRECT_LP_LIQUIDITY_REMOVAL;
use eth_token::token_analytics::{TokenPoolCurrentObservation, ACTIVE_OBSERVATION_TARGET_HORIZONS};
use reth_chain_query::RethQueryProvider;
use serde_json::{json, to_value, Value};

use crate::ranges::RangeIndexJob;
use crate::read_models::pool::PoolView;

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

fn active_target_summaries(rows: &[ObservationRow]) -> Vec<ActiveTargetSummary> {
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
                    format!("direct_lp_target_{horizon}_active_observation_delta"),
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
                "Risk Atlas stores one row per active token/pool observation with as-of features and lab-owned near-future target columns."
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
struct DistributionAccumulator {
    counts: BTreeMap<(String, String), i64>,
}

impl DistributionAccumulator {
    fn add(&mut self, section: impl Into<String>, bucket: impl Into<String>, count: i64) {
        if count == 0 {
            return;
        }
        *self
            .counts
            .entry((section.into(), bucket.into()))
            .or_default() += count;
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

fn i64_from_u64_opt(value: u64) -> Option<i64> {
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
