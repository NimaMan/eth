use crate::erc20::ERC20Token;
use crate::pools::BasePool;
use crate::token_analytics::ObservationBlockEventFlags;

use super::events::{event_count_at_block, lp_event_counts_at_block};
use super::transfers::token_approval_count_at_block;

pub(super) fn event_flags(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
) -> ObservationBlockEventFlags {
    let pool_swap_count_in_block = event_count_at_block(&pool.swap_events, block_number);
    let pool_mint_count_in_block = event_count_at_block(&pool.mint_events, block_number);
    let pool_burn_count_in_block = event_count_at_block(&pool.burn_events, block_number);
    let pool_sync_count_in_block = event_count_at_block(&pool.sync_events, block_number);
    let (lp_transfer_count_in_block, lp_burn_transfer_count_in_block, lp_approval_count_in_block) =
        lp_event_counts_at_block(token, &pool.identity.pool_address, block_number);
    let token_approval_count_in_block = token_approval_count_at_block(token, block_number);
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
        lp_burn_transfer_count_in_block,
        lp_approval_count_in_block,
        token_approval_count_in_block,
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
