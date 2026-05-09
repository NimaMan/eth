use tx_processor::ProcessedTransaction;

use crate::pools::uniswap::concentrated::scale_i128;
use crate::pools::uniswap::v2::{
    LPApprovalEvent, LPTransferEvent, UniswapV2BurnEvent, UniswapV2MintEvent, UniswapV2SwapEvent,
    UniswapV2SyncEvent, UniswapV2TransactionEvents,
};

use super::super::address::{address_string, hash_string, same_address};

pub(super) fn v2_events_from_processed_transaction(
    transaction: &ProcessedTransaction,
    pool_address: &str,
) -> UniswapV2TransactionEvents {
    UniswapV2TransactionEvents {
        syncs: transaction
            .uniswap_v2_syncs
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2SyncEvent {
                pair_address: address_string(&event.pair_address),
                reserve0: event.reserve0.to_string(),
                reserve1: event.reserve1.to_string(),
            })
            .collect(),
        swaps: transaction
            .uniswap_v2_swaps
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2SwapEvent {
                pair_address: address_string(&event.pair_address),
                sender: Some(address_string(&event.sender)),
                to: Some(address_string(&event.to)),
                amount0_in: event.amount0_in.to_string(),
                amount1_in: event.amount1_in.to_string(),
                amount0_out: event.amount0_out.to_string(),
                amount1_out: event.amount1_out.to_string(),
            })
            .collect(),
        mints: transaction
            .uniswap_v2_mints
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2MintEvent {
                pair_address: address_string(&event.pair_address),
                to: Some(address_string(&event.sender)),
                amount: Some(event.amount0.to_string()),
            })
            .collect(),
        burns: transaction
            .uniswap_v2_burns
            .iter()
            .filter(|event| same_address(&event.pair_address, pool_address))
            .map(|event| UniswapV2BurnEvent {
                pair_address: address_string(&event.pair_address),
                sender: Some(address_string(&event.sender)),
                amount0: event.amount0.to_string(),
                amount1: event.amount1.to_string(),
            })
            .collect(),
    }
}

pub(super) fn lp_transfers_from_processed_transaction(
    transaction: &ProcessedTransaction,
    pool_address: &str,
) -> Vec<LPTransferEvent> {
    transaction
        .erc20_transfers
        .iter()
        .filter(|event| same_address(&event.token_address, pool_address))
        .map(|event| LPTransferEvent {
            from_address: address_string(&event.from_address),
            to_address: address_string(&event.to_address),
            amount: event.amount.to_string(),
            block_number: Some(transaction.block_number),
            tx_hash: Some(hash_string(&transaction.hash)),
            log_index: Some(event.log_index),
        })
        .collect()
}

pub(super) fn lp_approvals_from_processed_transaction(
    transaction: &ProcessedTransaction,
    pool_address: &str,
) -> Vec<LPApprovalEvent> {
    transaction
        .erc20_approval_events
        .iter()
        .filter(|event| same_address(&event.token_address, pool_address))
        .map(|event| LPApprovalEvent {
            owner: address_string(&event.owner),
            spender: address_string(&event.spender),
            amount: event.amount.to_string(),
            block_number: Some(transaction.block_number),
            tx_hash: Some(hash_string(&transaction.hash)),
            block_timestamp: Some(transaction.block_timestamp),
        })
        .collect()
}

pub(super) fn signed_denom_swap_amounts(
    amount0: i128,
    amount1: i128,
    token_decimals: u8,
    denom_decimals: u8,
    token1_is_denom: bool,
) -> (f64, f64) {
    let token0_decimals = if token1_is_denom {
        token_decimals
    } else {
        denom_decimals
    };
    let token1_decimals = if token1_is_denom {
        denom_decimals
    } else {
        token_decimals
    };
    let token0_amount = scale_i128(amount0, token0_decimals);
    let token1_amount = scale_i128(amount1, token1_decimals);
    let denom_amount = if token1_is_denom {
        token1_amount
    } else {
        token0_amount
    };

    (denom_amount.max(0.0), (-denom_amount).max(0.0))
}
