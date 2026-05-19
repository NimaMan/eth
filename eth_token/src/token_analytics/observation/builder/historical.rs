use std::collections::BTreeMap;

use crate::erc20::ERC20Token;
use crate::pools::{BasePool, PoolLifecycle, PoolRuntimeState};
use crate::token_analytics::{ActiveObservationReason, TokenPoolCurrentObservation};

use super::current::build_current_observation;
use super::events::{event_block_number, event_timestamp};
use super::reasons::append_reason;
use super::utils::{activity_has_signal_for_denom, normalize_address};

pub fn build_historical_observations_for_token(
    token: &ERC20Token,
) -> Vec<TokenPoolCurrentObservation> {
    let mut observations = Vec::new();
    for pool in token.all_pool_bases() {
        observations.extend(build_historical_observations_for_pool(token, pool));
    }
    observations.sort_by(|left, right| {
        left.key
            .pool_address
            .cmp(&right.key.pool_address)
            .then(left.context.block_number.cmp(&right.context.block_number))
            .then(
                left.context
                    .active_observation_index
                    .cmp(&right.context.active_observation_index),
            )
    });
    observations
}

pub fn build_historical_observations_for_pool(
    token: &ERC20Token,
    pool: &BasePool,
) -> Vec<TokenPoolCurrentObservation> {
    let seeds = historical_observation_seeds(token, pool);
    let mut observations = Vec::with_capacity(seeds.len());
    for (index, seed) in seeds.into_values().enumerate() {
        let active_index = (index + 1) as u64;
        let activity = token.activity.blocks.get(&seed.block_number);
        let pool_as_of = pool_as_of_block(pool, seed.block_number);
        observations.push(build_current_observation(
            token,
            &pool_as_of,
            active_index,
            seed.block_number,
            seed.timestamp
                .or_else(|| activity.and_then(|row| row.timestamp)),
            seed.reasons,
            activity,
        ));
    }
    observations
}

#[derive(Clone, Debug, Default)]
struct HistoricalObservationSeed {
    block_number: u64,
    timestamp: Option<u64>,
    reasons: Vec<ActiveObservationReason>,
}

fn historical_observation_seeds(
    token: &ERC20Token,
    pool: &BasePool,
) -> BTreeMap<u64, HistoricalObservationSeed> {
    let mut seeds = BTreeMap::new();
    let denom = pool.identity.denom_address.to_ascii_lowercase();

    for activity in token
        .activity
        .blocks
        .values()
        .filter(|activity| activity_has_signal_for_denom(activity, &denom))
    {
        touch_historical_seed(&mut seeds, activity.block_number, activity.timestamp, None);
    }

    for snapshot in &pool.reserve_tracker.reserve_history {
        touch_historical_seed(
            &mut seeds,
            snapshot.block_number,
            Some(snapshot.timestamp),
            Some(ActiveObservationReason::PoolLiquidityChange),
        );
    }

    for (block_number, _) in &pool.price_history {
        touch_historical_seed(
            &mut seeds,
            *block_number,
            None,
            Some(ActiveObservationReason::PoolPriceChange),
        );
    }

    for event in pool
        .swap_events
        .iter()
        .chain(pool.mint_events.iter())
        .chain(pool.burn_events.iter())
        .chain(pool.sync_events.iter())
    {
        if let Some(block) = event_block_number(event) {
            touch_historical_seed(&mut seeds, block, event_timestamp(event), None);
        }
    }

    touch_historical_seed(
        &mut seeds,
        pool.creation_block,
        pool.creation_timestamp,
        Some(ActiveObservationReason::PoolLiquidityChange),
    );
    touch_historical_seed(
        &mut seeds,
        pool.can_buy_block,
        pool.can_buy_timestamp,
        Some(ActiveObservationReason::TradingStatusChange),
    );
    touch_historical_seed(
        &mut seeds,
        pool.tax_check_block,
        None,
        Some(ActiveObservationReason::TradingStatusChange),
    );
    touch_historical_seed(
        &mut seeds,
        pool.scam_block,
        None,
        Some(ActiveObservationReason::ScamStatusChange),
    );

    for status in &pool.trading_status_history {
        touch_historical_seed(
            &mut seeds,
            status.block_number,
            None,
            Some(ActiveObservationReason::TradingStatusChange),
        );
    }

    for event in &token.authority_tracker.owner_events {
        touch_historical_seed(
            &mut seeds,
            event.block_number,
            None,
            Some(ActiveObservationReason::ControlAddressActivity),
        );
    }

    for approval in &token.transfer_tracker.approvals {
        if approval
            .token_address
            .eq_ignore_ascii_case(&token.contract_address)
        {
            touch_historical_seed(
                &mut seeds,
                approval.block_number,
                None,
                Some(ActiveObservationReason::TokenApproval),
            );
        }
    }

    touch_lp_event_seeds(token, &pool.identity.pool_address, &mut seeds);
    seeds
}

fn touch_historical_seed(
    seeds: &mut BTreeMap<u64, HistoricalObservationSeed>,
    block_number: impl Into<Option<u64>>,
    timestamp: Option<u64>,
    reason: Option<ActiveObservationReason>,
) {
    let Some(block_number) = block_number.into() else {
        return;
    };
    let seed = seeds
        .entry(block_number)
        .or_insert_with(|| HistoricalObservationSeed {
            block_number,
            timestamp: None,
            reasons: Vec::new(),
        });
    if timestamp.is_some() {
        seed.timestamp = timestamp;
    }
    if let Some(reason) = reason {
        append_reason(&mut seed.reasons, reason);
    }
}

fn touch_lp_event_seeds(
    token: &ERC20Token,
    pool_address: &str,
    seeds: &mut BTreeMap<u64, HistoricalObservationSeed>,
) {
    let pool_address = normalize_address(pool_address);
    if let Some(pool) = token.v2_pools.get(&pool_address) {
        for event in pool
            .lp_tracker
            .transfers
            .iter()
            .chain(pool.lp_tracker.approval_events.iter())
        {
            if let Some(block) = event_block_number(event) {
                touch_historical_seed(seeds, block, event_timestamp(event), None);
            }
        }
        return;
    }
    if let Some(pool) = token.v3_pools.get(&pool_address) {
        for event in &pool.liquidity_position_events {
            if let Some(block) = event_block_number(event) {
                touch_historical_seed(seeds, block, event_timestamp(event), None);
            }
        }
        return;
    }
    if let Some(pool) = token.v4_pools.get(&pool_address) {
        for event in pool
            .liquidity_position_events
            .iter()
            .chain(pool.lp_approval_events.iter())
        {
            if let Some(block) = event_block_number(event) {
                touch_historical_seed(seeds, block, event_timestamp(event), None);
            }
        }
    }
}

fn pool_as_of_block(pool: &BasePool, block_number: u64) -> BasePool {
    let mut as_of = pool.clone();
    as_of
        .reserve_tracker
        .reserve_history
        .retain(|snapshot| snapshot.block_number <= block_number);
    as_of.reserve_tracker.latest_snapshot = as_of.reserve_tracker.reserve_history.last().cloned();
    as_of
        .price_history
        .retain(|(price_block, _)| *price_block <= block_number);

    apply_reserve_snapshot_as_of(&mut as_of, block_number);
    apply_trading_status_as_of(&mut as_of, block_number);
    apply_liquidity_removal_as_of(pool, &mut as_of, block_number);
    refresh_lifecycle_as_of(&mut as_of);
    as_of
}

fn apply_reserve_snapshot_as_of(pool: &mut BasePool, block_number: u64) {
    let latest = pool.reserve_tracker.latest_snapshot.clone();
    if let Some(snapshot) = latest.as_ref() {
        pool.state.update_reserves(
            snapshot.denom_reserve,
            snapshot.token_reserve,
            snapshot.block_number,
        );
        pool.state.total_liquidity = snapshot.denom_reserve.max(0.0);
        return;
    }

    pool.state = PoolRuntimeState::default();
    if pool
        .creation_block
        .is_some_and(|creation| creation <= block_number)
    {
        pool.state.lifecycle = PoolLifecycle::Discovered;
    }
}

fn apply_trading_status_as_of(pool: &mut BasePool, block_number: u64) {
    pool.state.can_buy = false;
    pool.state.can_sell = false;
    pool.buy_tax = None;
    pool.sell_tax = None;
    pool.tax_check_block = pool.tax_check_block.filter(|block| *block <= block_number);
    pool.tax_check_tx = pool
        .tax_check_tx
        .clone()
        .filter(|_| pool.tax_check_block.is_some());

    if let Some(status) = pool
        .trading_status_history
        .iter()
        .filter(|status| status.block_number <= block_number)
        .max_by_key(|status| status.block_number)
    {
        pool.state.can_buy = status.can_buy;
        pool.state.can_sell = status.can_sell;
        pool.buy_tax = status.buy_tax;
        pool.sell_tax = status.sell_tax;
        pool.tax_check_block = Some(status.block_number);
        pool.tax_check_tx = Some(status.tx_hash.clone()).filter(|tx| !tx.is_empty());
    } else if pool
        .can_buy_block
        .is_some_and(|block| block <= block_number)
    {
        pool.state.can_buy = true;
    }

    if pool.can_buy_block.is_some_and(|block| block > block_number) {
        pool.can_buy_block = None;
        pool.can_buy_tx = None;
        pool.can_buy_timestamp = None;
    }
}

fn apply_liquidity_removal_as_of(source: &BasePool, pool: &mut BasePool, block_number: u64) {
    let removed = source
        .scam_block
        .is_some_and(|scam_block| scam_block <= block_number)
        || pool
            .reserve_tracker
            .scam_block
            .is_some_and(|scam_block| scam_block <= block_number);

    if removed {
        pool.reserve_tracker.is_scam = true;
        pool.reserve_tracker.scam_block = source.scam_block.or(pool.reserve_tracker.scam_block);
        pool.reserve_tracker.scam_tx_hash = source
            .scam_tx_hash
            .clone()
            .or_else(|| pool.reserve_tracker.scam_tx_hash.clone());
        pool.scam_block = pool.reserve_tracker.scam_block;
        pool.scam_tx_hash = pool.reserve_tracker.scam_tx_hash.clone();
        pool.scam_label = source
            .scam_label
            .clone()
            .or_else(|| pool.reserve_tracker.scam_label.clone());
        pool.state.can_buy = false;
        pool.state.can_sell = false;
        pool.state.lifecycle = PoolLifecycle::LiquidityRemoved;
    } else {
        pool.reserve_tracker.is_scam = false;
        pool.reserve_tracker.scam_block = None;
        pool.reserve_tracker.scam_tx_hash = None;
        pool.scam_block = None;
        pool.scam_tx_hash = None;
        pool.scam_label = None;
    }
}

fn refresh_lifecycle_as_of(pool: &mut BasePool) {
    if pool.reserve_tracker.is_scam {
        pool.state.lifecycle = PoolLifecycle::LiquidityRemoved;
        return;
    }

    let has_seen_reserves = pool.state.last_update_block > 0 || pool.state.last_sync_block > 0;
    if has_seen_reserves && (pool.state.denom_reserve <= 0.0 || pool.state.token_reserve <= 0.0) {
        pool.state.can_buy = false;
        pool.state.can_sell = false;
        pool.state.lifecycle = PoolLifecycle::Drained;
        return;
    }

    pool.state.lifecycle = if pool.state.can_buy && pool.state.can_sell {
        PoolLifecycle::Trading
    } else if pool.state.can_buy {
        PoolLifecycle::CannotSell
    } else if has_seen_reserves {
        PoolLifecycle::LiquidityDeposited
    } else {
        PoolLifecycle::Discovered
    };
}
