use crate::erc20::ERC20Token;
use crate::pools::BasePool;
use crate::token_analytics::PoolActivityFeatures;

use super::utils::{activity_has_signal_for_denom, volume_for_denom};

pub(super) fn apply_cumulative_activity_features(
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
