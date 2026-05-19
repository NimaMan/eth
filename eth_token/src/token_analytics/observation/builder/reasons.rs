use crate::token_analytics::{
    ActiveObservationReason, ObservationBlockActivity, ObservationBlockEventFlags,
};

pub(super) fn append_activity_reasons(
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

pub(super) fn append_event_reasons(
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
    if flags.lp_burn_transfer_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::LpBurn);
    }
    if flags.lp_transfer_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::LpTransfer);
    }
    if flags.token_approval_count_in_block > 0 {
        append_reason(reasons, ActiveObservationReason::TokenApproval);
    }
    if flags.trading_enabled_in_block || flags.trading_status_changed_in_block {
        append_reason(reasons, ActiveObservationReason::TradingStatusChange);
    }
    if flags.scam_status_changed_in_block {
        append_reason(reasons, ActiveObservationReason::ScamStatusChange);
    }
}

pub(super) fn append_reason(
    reasons: &mut Vec<ActiveObservationReason>,
    reason: ActiveObservationReason,
) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}
