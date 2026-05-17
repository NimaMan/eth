use std::collections::BTreeMap;

use eth_token::erc20::ERC20Token;
use eth_token::pools::uniswap::v2::LPHolderSnapshot;
use eth_token::pools::{BasePool, TaxBucket};
use eth_token::token_activity::TokenBlockActivity;
use eth_token::token_analytics::{
    ActiveObservationReason, FeatureEvidenceBlocks, LpControlFeatures, ObservationBlockActivity,
    ObservationBlockActivitySource, ObservationBlockEventFlags, ObservationPoolTradingState,
    ObservationTransactionSummary, PoolActivityFeatures, PoolLiquidityFeatures, PoolMarketFeatures,
    TokenAuthorityFeatures, TokenNetworkFeatures, TokenPoolCurrentObservation,
    TokenPoolObservationContext, TokenPoolObservationFeatures, TokenPoolObservationKey,
    TokenStaticFeatures,
};
use eth_token::tracking::TokenBlockUpdateReport;
use serde_json::Value;

use crate::ranges::RangeIndexState;

pub fn collect_observations(state: &mut RangeIndexState, report: &TokenBlockUpdateReport) {
    let mut touched = BTreeMap::<(String, String), Vec<ActiveObservationReason>>::new();

    for update in &report.token_updates {
        for pool_address in update
            .discovered_known_v2_pools
            .iter()
            .chain(update.discovered_uniswap_v3_pools.iter())
            .chain(update.discovered_uniswap_v4_pools.iter())
        {
            push_reason(
                &mut touched,
                &update.token_address,
                pool_address,
                ActiveObservationReason::PoolLiquidityChange,
            );
        }

        for pool_address in update
            .updated_known_v2_pools
            .iter()
            .chain(update.updated_uniswap_v3_pools.iter())
            .chain(update.updated_uniswap_v4_pools.iter())
        {
            push_reason(
                &mut touched,
                &update.token_address,
                pool_address,
                ActiveObservationReason::PoolSync,
            );
        }

        for pool_address in update
            .simulated_known_v2_pools
            .iter()
            .chain(update.simulated_uniswap_v3_pools.iter())
            .chain(update.simulated_uniswap_v4_pools.iter())
        {
            push_reason(
                &mut touched,
                &update.token_address,
                pool_address,
                ActiveObservationReason::TradingStatusChange,
            );
        }
    }

    for ((token_address, pool_address), seed_reasons) in touched {
        let Some(token) = state.processor.registry.token(&token_address) else {
            continue;
        };
        let Some(pool) = token.pool_base(&pool_address) else {
            continue;
        };
        let pool_key = observation_pool_key(&token_address, &pool_address);
        let active_index = {
            let count = state
                .active_observation_counts_by_pool
                .entry(pool_key)
                .or_default();
            *count += 1;
            *count
        };
        let activity = token.activity.blocks.get(&report.block_number);
        let observation = build_current_observation(
            token,
            pool,
            active_index,
            report.block_number,
            Some(report.block_timestamp),
            seed_reasons,
            activity,
        );
        state.observations.push(observation);
    }
}

fn push_reason(
    touched: &mut BTreeMap<(String, String), Vec<ActiveObservationReason>>,
    token_address: &str,
    pool_address: &str,
    reason: ActiveObservationReason,
) {
    let reasons = touched
        .entry((
            normalize_address(token_address),
            normalize_address(pool_address),
        ))
        .or_default();
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn build_current_observation(
    token: &ERC20Token,
    pool: &BasePool,
    active_index: u64,
    block_number: u64,
    timestamp: Option<u64>,
    seed_reasons: Vec<ActiveObservationReason>,
    block_activity: Option<&TokenBlockActivity>,
) -> TokenPoolCurrentObservation {
    let activity = block_activity
        .map(ObservationBlockActivity::from)
        .unwrap_or_else(|| ObservationBlockActivity {
            source: ObservationBlockActivitySource::TokenActivityTracker,
            metrics_complete: true,
            ..Default::default()
        });
    let event_flags = event_flags(token, pool, block_number);
    let mut reasons = seed_reasons;
    append_activity_reasons(&activity, &pool.identity.denom_address, &mut reasons);
    append_event_reasons(&event_flags, &mut reasons);
    if reasons.is_empty() {
        reasons.push(ActiveObservationReason::Manual);
    }

    let mut activity_features = PoolActivityFeatures::from(&activity);
    let denom = pool.identity.denom_address.as_str();
    activity_features.set_block_volume(
        activity.buy_volume_for_denom(denom),
        activity.sell_volume_for_denom(denom),
    );
    activity_features.set_token_transfer_volume_context(
        activity.token_transfer_volume,
        token.total_supply_scaled(),
        Some(pool.token_reserve()),
    );
    apply_cumulative_activity_features(
        &mut activity_features,
        token,
        pool,
        active_index,
        block_number,
    );

    let trading = ObservationPoolTradingState {
        lifecycle: Some(pool.state.lifecycle),
        can_buy: pool.state.can_buy,
        can_sell: pool.state.can_sell,
        effective_can_buy: pool.effective_can_buy(),
        effective_can_sell: pool.effective_can_sell(),
        buy_tax: pool.buy_tax,
        sell_tax: pool.sell_tax,
        tax_check_block: pool.tax_check_block,
        tax_check_tx: pool.tax_check_tx.clone(),
        last_trading_failure_class: pool.last_trading_failure_class.clone(),
        liquidity_removed_as_of: pool.has_liquidity_removal(),
        liquidity_removal_block_as_of: pool.scam_block,
    };

    let mut token_features =
        TokenStaticFeatures::from_blocks(token.creation_block, pool.creation_block, block_number);
    token_features.decimals = Some(token.decimals);
    token_features.total_supply_scaled = token.total_supply_scaled();
    token_features.total_supply_from_transfers = Some(token.total_supply_from_transfers());

    let mut authority_features = TokenAuthorityFeatures::with_owner_context(
        token.creator_address.clone(),
        token.current_owner(),
        token.ownership_renounced(),
    );
    authority_features.renouncement_block = token.authority_tracker.renouncement_block;
    authority_features.control_address_count = Some(token.token_control_addresses.len() as u32);
    authority_features.control_address_tx_count_in_block =
        Some(pool.latest_block_control_address_txs.len() as u32);

    let scam_mechanism = pool.inferred_scam_mechanism();
    let mut market_features =
        PoolMarketFeatures::with_ages(pool.creation_block, pool.can_buy_block, block_number);
    market_features.lifecycle = Some(pool.state.lifecycle);
    market_features.can_buy = pool.state.can_buy;
    market_features.can_sell = pool.state.can_sell;
    market_features.effective_can_buy = pool.effective_can_buy();
    market_features.effective_can_sell = pool.effective_can_sell();
    market_features.buy_tax = pool.buy_tax;
    market_features.sell_tax = pool.sell_tax;
    market_features.tax_bucket = Some(tax_bucket_key(TaxBucket::combined(
        display_tax(pool.buy_tax),
        display_tax(pool.sell_tax),
    )));
    market_features.last_trading_failure_class = pool.last_trading_failure_class.clone();
    market_features.liquidity_removed_as_of = pool.has_liquidity_removal();
    market_features.liquidity_removal_block_as_of = pool.scam_block;
    market_features.scam_mechanism_as_of = scam_mechanism
        .as_ref()
        .filter(|_| pool.has_liquidity_removal())
        .map(|mechanism| mechanism.mechanism.clone());
    market_features.scam_label_as_of = pool
        .scam_label
        .clone()
        .or_else(|| {
            scam_mechanism
                .as_ref()
                .map(|mechanism| mechanism.label.clone())
        })
        .filter(|_| pool.has_liquidity_removal());

    let liquidity_features = liquidity_features(pool);
    let lp_control = lp_control_features(token, &pool.identity.pool_address, block_number)
        .with_last_approval_offsets(pool.creation_block, pool.can_buy_block);

    let evidence_blocks = FeatureEvidenceBlocks {
        token_static_latest_block: token.creation_block,
        authority_latest_block: token
            .authority_tracker
            .owner_events
            .iter()
            .map(|event| event.block_number)
            .max(),
        market_latest_block: [
            pool.creation_block,
            pool.can_buy_block,
            pool.tax_check_block,
            pool.scam_block,
            Some(block_number),
        ]
        .into_iter()
        .flatten()
        .max(),
        liquidity_latest_block: pool
            .reserve_tracker
            .latest_snapshot
            .as_ref()
            .map(|snapshot| snapshot.block_number),
        lp_control_latest_block: lp_control.last_lp_approval_block,
        activity_latest_block: Some(block_number),
        network_latest_block: None,
    };

    let transactions = token
        .activity
        .transactions_by_hash
        .values()
        .filter(|transaction| transaction.block_number == block_number)
        .map(ObservationTransactionSummary::from)
        .collect();

    TokenPoolCurrentObservation {
        key: TokenPoolObservationKey::new(
            &pool.identity.token_address,
            &pool.identity.pool_address,
            &pool.identity.denom_address,
            &pool.identity.protocol,
        )
        .with_chain("ethereum"),
        context: TokenPoolObservationContext::new(active_index, block_number, timestamp)
            .with_reasons(reasons),
        activity,
        event_flags,
        trading,
        features: TokenPoolObservationFeatures {
            token: token_features,
            authority: authority_features,
            market: market_features,
            liquidity: liquidity_features,
            lp_control,
            activity: activity_features,
            network: TokenNetworkFeatures {
                unique_address_count: Some(token.unique_addresses().len() as u32),
                feature_scope: Some("token_activity_tracker".to_string()),
                ..Default::default()
            },
            evidence_blocks,
        },
        transactions,
    }
}

fn event_flags(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
) -> ObservationBlockEventFlags {
    let pool_swap_count_in_block = event_count_at_block(&pool.swap_events, block_number);
    let pool_mint_count_in_block = event_count_at_block(&pool.mint_events, block_number);
    let pool_burn_count_in_block = event_count_at_block(&pool.burn_events, block_number);
    let pool_sync_count_in_block = event_count_at_block(&pool.sync_events, block_number);
    let (lp_transfer_count_in_block, lp_approval_count_in_block) =
        lp_event_counts_at_block(token, &pool.identity.pool_address, block_number);
    let trading_status_changed_in_block = pool
        .trading_status_history
        .iter()
        .any(|status| status.block_number == block_number);
    let liquidity_removal_in_block = pool.scam_block == Some(block_number);

    ObservationBlockEventFlags {
        token_created_in_block: token.creation_block == Some(block_number),
        pool_created_in_block: pool.creation_block == Some(block_number),
        trading_enabled_in_block: pool.can_buy_block == Some(block_number),
        pool_swap_count_in_block,
        pool_mint_count_in_block,
        pool_burn_count_in_block,
        pool_sync_count_in_block,
        lp_transfer_count_in_block,
        lp_approval_count_in_block,
        liquidity_updated_in_block: pool_sync_count_in_block > 0
            || pool_mint_count_in_block > 0
            || pool_burn_count_in_block > 0
            || pool
                .reserve_tracker
                .latest_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.block_number == block_number),
        price_updated_in_block: pool
            .price_history
            .iter()
            .any(|(price_block, _)| *price_block == block_number),
        tax_checked_in_block: pool.tax_check_block == Some(block_number),
        trading_status_changed_in_block,
        scam_status_changed_in_block: liquidity_removal_in_block,
        liquidity_removal_in_block,
        liquidity_removed_as_of: pool.has_liquidity_removal(),
    }
}

fn liquidity_features(pool: &BasePool) -> PoolLiquidityFeatures {
    let initial = pool.reserve_tracker.reserve_history.first();
    let max_denom_reserve = pool
        .reserve_tracker
        .reserve_history
        .iter()
        .map(|snapshot| snapshot.denom_reserve)
        .filter(|value| value.is_finite())
        .fold(None, |max: Option<f64>, value| {
            Some(max.map_or(value, |current| current.max(value)))
        });

    let mut features = PoolLiquidityFeatures::new(
        pool.denom_reserve(),
        pool.token_reserve(),
        pool.state.total_liquidity,
        pool.price(),
    )
    .with_initial_reserves(
        initial.map(|snapshot| snapshot.denom_reserve),
        initial.map(|snapshot| snapshot.token_reserve),
        initial.map(|snapshot| snapshot.price),
    )
    .with_max_denom_reserve(max_denom_reserve);
    features.reserve_observation_count = pool.reserve_tracker.reserve_history.len() as u32;
    features
}

fn lp_control_features(
    token: &ERC20Token,
    pool_address: &str,
    as_of_block: u64,
) -> LpControlFeatures {
    let pool_address = normalize_address(pool_address);
    if let Some(pool) = token.v2_pools.get(&pool_address) {
        let holders = pool.lp_holders();
        let top_holder = holders
            .iter()
            .max_by(|left, right| left.share.total_cmp(&right.share));
        let last_approval = pool
            .lp_tracker
            .approval_events
            .iter()
            .rev()
            .find(|event| event_block_number(event).is_some_and(|block| block <= as_of_block));
        let first_approval_block = pool
            .lp_tracker
            .approval_events
            .iter()
            .filter_map(event_block_number)
            .filter(|block| *block <= as_of_block)
            .min();
        let max_approval_amount = max_lp_approval_amount(&holders);
        return LpControlFeatures {
            lp_total_supply: Some(pool.lp_tracker.total_supply),
            lp_holder_count: Some(holders.len() as u32),
            lp_top_holder_share_pct: top_holder.map(|holder| holder.share),
            lp_top_holder_is_creator: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.creator_address.as_deref())
            }),
            lp_top_holder_is_owner: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.current_owner().as_deref())
            }),
            lp_approval_count_as_of: Some(
                pool.lp_tracker
                    .approval_events
                    .iter()
                    .filter_map(event_block_number)
                    .filter(|block| *block <= as_of_block)
                    .count() as u32,
            ),
            lp_first_approval_block_as_of: first_approval_block,
            lp_holders_with_approvals_count: Some(pool.holders_with_approvals().len() as u32),
            lp_approved_spender_count_as_of: Some(
                holders
                    .iter()
                    .map(|holder| holder.approvals.len())
                    .sum::<usize>() as u32,
            ),
            lp_approved_pct_as_of: lp_approved_pct_from_max(
                pool.lp_tracker.total_supply,
                max_approval_amount,
            ),
            lp_approved_to_router: Some(pool.total_approved_to_routers()),
            lp_approved_to_router_pct: Some(pool.lp_approved_percentage()),
            lp_router_approved_pct_as_of: Some(pool.lp_approved_percentage()),
            lp_router_approval_seen_as_of: Some(pool.total_approved_to_routers() > 0.0),
            lp_max_approval_amount_as_of: max_approval_amount,
            last_lp_approval_block: last_approval.and_then(event_block_number),
            last_lp_approval_timestamp: last_approval.and_then(event_timestamp),
            last_lp_approval_owner: last_approval.and_then(|event| event_string(event, "owner")),
            last_lp_approval_spender: last_approval
                .and_then(|event| event_string(event, "spender")),
            last_lp_approval_is_router: last_approval
                .and_then(|event| event_bool(event, "is_router")),
            last_lp_approval_owner_is_creator: last_approval
                .and_then(|event| event_string(event, "owner"))
                .and_then(|owner| {
                    same_optional_address(Some(&owner), token.creator_address.as_deref())
                }),
            last_lp_approval_owner_is_current_owner: last_approval
                .and_then(|event| event_string(event, "owner"))
                .and_then(|owner| {
                    same_optional_address(Some(&owner), token.current_owner().as_deref())
                }),
            feature_scope: Some("uniswap_v2_lp_tracker".to_string()),
            ..Default::default()
        }
        .with_as_of_offsets(as_of_block);
    }

    if let Some(pool) = token.v3_pools.get(&pool_address) {
        let holders = pool.lp_holders();
        let top_holder = holders
            .iter()
            .max_by(|left, right| left.share.total_cmp(&right.share));
        return LpControlFeatures {
            lp_total_supply: Some(pool.lp_total_supply()),
            lp_holder_count: Some(holders.len() as u32),
            lp_top_holder_share_pct: top_holder.map(|holder| holder.share),
            lp_top_holder_is_creator: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.creator_address.as_deref())
            }),
            lp_top_holder_is_owner: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.current_owner().as_deref())
            }),
            feature_scope: Some("uniswap_v3_positions".to_string()),
            ..Default::default()
        }
        .with_as_of_offsets(as_of_block);
    }

    if let Some(pool) = token.v4_pools.get(&pool_address) {
        let holders = pool.lp_holders();
        let top_holder = holders
            .iter()
            .max_by(|left, right| left.share.total_cmp(&right.share));
        let last_approval = pool
            .lp_approval_events
            .iter()
            .rev()
            .find(|event| event_block_number(event).is_some_and(|block| block <= as_of_block));
        let first_approval_block = pool
            .lp_approval_events
            .iter()
            .filter_map(event_block_number)
            .filter(|block| *block <= as_of_block)
            .min();
        let lp_total_supply = pool.lp_total_supply();
        let max_approval_amount = max_lp_approval_amount(&holders);
        return LpControlFeatures {
            lp_total_supply: Some(lp_total_supply),
            lp_holder_count: Some(holders.len() as u32),
            lp_top_holder_share_pct: top_holder.map(|holder| holder.share),
            lp_top_holder_is_creator: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.creator_address.as_deref())
            }),
            lp_top_holder_is_owner: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.current_owner().as_deref())
            }),
            lp_approval_count_as_of: Some(
                pool.lp_approval_events
                    .iter()
                    .filter_map(event_block_number)
                    .filter(|block| *block <= as_of_block)
                    .count() as u32,
            ),
            lp_first_approval_block_as_of: first_approval_block,
            lp_holders_with_approvals_count: Some(pool.holders_with_approvals().len() as u32),
            lp_approved_spender_count_as_of: Some(
                holders
                    .iter()
                    .map(|holder| holder.approvals.len())
                    .sum::<usize>() as u32,
            ),
            lp_approved_pct_as_of: lp_approved_pct_from_max(lp_total_supply, max_approval_amount),
            lp_approved_to_router: Some(pool.total_approved_to_routers()),
            lp_approved_to_router_pct: Some(pool.lp_approved_percentage()),
            lp_router_approved_pct_as_of: Some(pool.lp_approved_percentage()),
            lp_router_approval_seen_as_of: Some(pool.total_approved_to_routers() > 0.0),
            lp_max_approval_amount_as_of: max_approval_amount,
            last_lp_approval_block: last_approval.and_then(event_block_number),
            last_lp_approval_timestamp: last_approval.and_then(event_timestamp),
            last_lp_approval_owner: last_approval.and_then(|event| event_string(event, "owner")),
            last_lp_approval_spender: last_approval
                .and_then(|event| event_string(event, "spender")),
            last_lp_approval_is_router: last_approval
                .and_then(|event| event_bool(event, "is_router")),
            last_lp_approval_owner_is_creator: last_approval
                .and_then(|event| event_string(event, "owner"))
                .and_then(|owner| {
                    same_optional_address(Some(&owner), token.creator_address.as_deref())
                }),
            last_lp_approval_owner_is_current_owner: last_approval
                .and_then(|event| event_string(event, "owner"))
                .and_then(|owner| {
                    same_optional_address(Some(&owner), token.current_owner().as_deref())
                }),
            feature_scope: Some("uniswap_v4_positions".to_string()),
            ..Default::default()
        }
        .with_as_of_offsets(as_of_block);
    }

    LpControlFeatures::default().with_as_of_offsets(as_of_block)
}

fn apply_cumulative_activity_features(
    features: &mut PoolActivityFeatures,
    token: &ERC20Token,
    pool: &BasePool,
    active_index: u64,
    block_number: u64,
) {
    let denom = pool.identity.denom_address.to_ascii_lowercase();
    let mut first_activity_block = None;
    let mut last_activity_block = None;

    for activity in token
        .activity
        .blocks
        .values()
        .filter(|activity| activity.block_number <= block_number)
    {
        if !activity_has_signal_for_denom(activity, &denom) {
            continue;
        }
        first_activity_block.get_or_insert(activity.block_number);
        last_activity_block = Some(activity.block_number);
        features.cumulative_tx_count += u64::from(activity.num_tx);
        features.cumulative_token_transfer_count += u64::from(activity.token_transfer_count);
        features.cumulative_denom_transfer_count += u64::from(activity.denom_transfer_count);
        features.cumulative_buy_volume_denom +=
            volume_for_denom(&activity.buy_volume_by_denom, &denom);
        features.cumulative_sell_volume_denom +=
            volume_for_denom(&activity.sell_volume_by_denom, &denom);
        features.cumulative_total_bribe_eth += activity.total_bribe_eth;
    }

    features.cumulative_active_observation_count = active_index;
    features.first_activity_block = first_activity_block;
    features.last_activity_block = last_activity_block;
    features.blocks_since_last_activity =
        last_activity_block.map(|block| block_number.saturating_sub(block));
    features.net_buy_volume_denom =
        features.cumulative_buy_volume_denom - features.cumulative_sell_volume_denom;
    if features.cumulative_sell_volume_denom > 0.0 {
        features.buy_sell_volume_ratio =
            Some(features.cumulative_buy_volume_denom / features.cumulative_sell_volume_denom);
    }
    if features.cumulative_token_transfer_count > 0 {
        features.denom_token_transfer_ratio = Some(
            features.cumulative_denom_transfer_count as f64
                / features.cumulative_token_transfer_count as f64,
        );
    }
    if let (Some(first), Some(last)) = (first_activity_block, last_activity_block) {
        let span = last.saturating_sub(first).saturating_add(1);
        if span > 0 {
            features.activity_density = Some(active_index as f64 / span as f64);
        }
    }
    if active_index > 0 {
        features.tx_per_active_observation =
            Some(features.cumulative_tx_count as f64 / active_index as f64);
    }

    apply_recent_activity_window(features, token, &denom, block_number, 10);
    apply_recent_activity_window(features, token, &denom, block_number, 50);
    apply_recent_activity_window(features, token, &denom, block_number, 100);
    features.feature_scope = Some("token_pool_observation".to_string());
}

fn apply_recent_activity_window(
    features: &mut PoolActivityFeatures,
    token: &ERC20Token,
    denom: &str,
    block_number: u64,
    window: u64,
) {
    let lower_bound = block_number.saturating_sub(window.saturating_sub(1));
    let mut active_observations = 0u32;
    let mut tx_count = 0u64;
    for activity in token.activity.blocks.values().filter(|activity| {
        activity.block_number >= lower_bound
            && activity.block_number <= block_number
            && activity_has_signal_for_denom(activity, denom)
    }) {
        active_observations += 1;
        tx_count += u64::from(activity.num_tx);
    }
    let density = if window > 0 {
        Some(active_observations as f64 / window as f64)
    } else {
        None
    };
    let tx_share = if features.cumulative_tx_count > 0 {
        Some(tx_count as f64 / features.cumulative_tx_count as f64)
    } else {
        None
    };

    match window {
        10 => {
            features.active_observations_last_10 = Some(active_observations);
            features.tx_count_last_10 = Some(tx_count);
            features.active_density_last_10 = density;
            features.tx_share_last_10_to_total = tx_share;
        }
        50 => {
            features.active_observations_last_50 = Some(active_observations);
            features.tx_count_last_50 = Some(tx_count);
            features.active_density_last_50 = density;
            features.tx_share_last_50_to_total = tx_share;
        }
        100 => {
            features.active_observations_last_100 = Some(active_observations);
            features.tx_count_last_100 = Some(tx_count);
            features.active_density_last_100 = density;
            features.tx_share_last_100_to_total = tx_share;
        }
        _ => {}
    }
}

fn append_activity_reasons(
    activity: &ObservationBlockActivity,
    denom_address: &str,
    reasons: &mut Vec<ActiveObservationReason>,
) {
    if activity.token_transfer_count > 0 {
        append_reason(reasons, ActiveObservationReason::TokenTransfer);
    }
    if activity.denom_transfer_count > 0 {
        append_reason(reasons, ActiveObservationReason::DenomTransfer);
    }
    if activity.buy_volume_for_denom(denom_address) > 0.0 {
        append_reason(reasons, ActiveObservationReason::BuyVolume);
    }
    if activity.sell_volume_for_denom(denom_address) > 0.0 {
        append_reason(reasons, ActiveObservationReason::SellVolume);
    }
    if activity.total_bribe_eth > 0.0 {
        append_reason(reasons, ActiveObservationReason::Bribe);
    }
}

fn append_event_reasons(
    flags: &ObservationBlockEventFlags,
    reasons: &mut Vec<ActiveObservationReason>,
) {
    if flags.pool_swap_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::PoolSwap);
    }
    if flags.pool_mint_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::PoolMint);
    }
    if flags.pool_burn_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::PoolBurn);
    }
    if flags.pool_sync_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::PoolSync);
    }
    if flags.liquidity_updated_in_block {
        append_reason(reasons, ActiveObservationReason::PoolLiquidityChange);
    }
    if flags.price_updated_in_block {
        append_reason(reasons, ActiveObservationReason::PoolPriceChange);
    }
    if flags.lp_approval_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::LpApproval);
    }
    if flags.lp_transfer_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::LpTransfer);
    }
    if flags.trading_enabled_in_block || flags.trading_status_changed_in_block {
        append_reason(reasons, ActiveObservationReason::TradingStatusChange);
    }
    if flags.scam_status_changed_in_block {
        append_reason(reasons, ActiveObservationReason::ScamStatusChange);
    }
}

fn append_reason(reasons: &mut Vec<ActiveObservationReason>, reason: ActiveObservationReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn lp_event_counts_at_block(
    token: &ERC20Token,
    pool_address: &str,
    block_number: u64,
) -> (u32, u32) {
    let pool_address = normalize_address(pool_address);
    if let Some(pool) = token.v2_pools.get(&pool_address) {
        return (
            event_count_at_block(&pool.lp_tracker.transfers, block_number),
            event_count_at_block(&pool.lp_tracker.approval_events, block_number),
        );
    }
    if let Some(pool) = token.v3_pools.get(&pool_address) {
        return (
            event_count_at_block(&pool.liquidity_position_events, block_number),
            0,
        );
    }
    if let Some(pool) = token.v4_pools.get(&pool_address) {
        return (
            event_count_at_block(&pool.liquidity_position_events, block_number),
            event_count_at_block(&pool.lp_approval_events, block_number),
        );
    }
    (0, 0)
}

fn event_count_at_block(events: &[Value], block_number: u64) -> u32 {
    events
        .iter()
        .filter(|event| event_block_number(event) == Some(block_number))
        .count() as u32
}

fn event_block_number(event: &Value) -> Option<u64> {
    event
        .get("block")
        .or_else(|| event.get("block_number"))
        .and_then(Value::as_u64)
}

fn event_timestamp(event: &Value) -> Option<u64> {
    event
        .get("timestamp")
        .or_else(|| event.get("block_timestamp"))
        .and_then(Value::as_u64)
}

fn event_string(event: &Value, key: &str) -> Option<String> {
    event
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.to_ascii_lowercase())
}

fn event_bool(event: &Value, key: &str) -> Option<bool> {
    event.get(key).and_then(Value::as_bool)
}

fn activity_has_signal_for_denom(activity: &TokenBlockActivity, denom: &str) -> bool {
    activity.num_tx > 0
        || activity.token_transfer_count > 0
        || activity.denom_transfer_count > 0
        || volume_for_denom(&activity.buy_volume_by_denom, denom) > 0.0
        || volume_for_denom(&activity.sell_volume_by_denom, denom) > 0.0
        || activity.total_bribe_eth > 0.0
}

fn volume_for_denom(volumes: &BTreeMap<String, f64>, denom: &str) -> f64 {
    volumes
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(denom))
        .map(|(_, value)| *value)
        .unwrap_or(0.0)
}

fn tax_bucket_key(bucket: TaxBucket) -> String {
    match bucket {
        TaxBucket::Unknown => "unknown",
        TaxBucket::NoTax => "no_tax",
        TaxBucket::LowTax => "low_tax",
        TaxBucket::ModerateTax => "moderate_tax",
        TaxBucket::HighTax => "high_tax",
        TaxBucket::ExtremeTax => "extreme_tax",
    }
    .to_string()
}

fn display_tax(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

fn same_optional_address(left: Option<&str>, right: Option<&str>) -> Option<bool> {
    left.zip(right)
        .map(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn max_lp_approval_amount(holders: &[LPHolderSnapshot]) -> Option<f64> {
    holders
        .iter()
        .flat_map(|holder| holder.approvals.values().map(|approval| approval.amount))
        .filter(|amount| amount.is_finite() && *amount > 0.0)
        .max_by(|left, right| left.total_cmp(right))
}

fn lp_approved_pct_from_max(total_supply: f64, max_approval_amount: Option<f64>) -> Option<f64> {
    if !total_supply.is_finite() || total_supply <= 0.0 {
        return None;
    }
    max_approval_amount.map(|amount| (amount.min(total_supply) / total_supply) * 100.0)
}

fn observation_pool_key(token_address: &str, pool_address: &str) -> String {
    format!(
        "{}:{}",
        normalize_address(token_address),
        normalize_address(pool_address)
    )
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
