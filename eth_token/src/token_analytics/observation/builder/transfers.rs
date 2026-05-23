use std::collections::HashSet;

use crate::erc20::ERC20Token;
use crate::pools::BasePool;
use crate::state::TokenTransferRecord;
use crate::token_analytics::{
    ObservationSellFlow, ObservationTokenPoolMovement, ObservationTransferSummary,
};

use super::roles::{address_role, observation_role_context, ObservationAddressRole};
use super::utils::{
    finite_non_negative, is_usd_denom, pool_token_reserve_ratio_denominator, ratio_if_positive,
    volume_for_denom,
};

pub(super) fn block_transfer_summary(
    token: &ERC20Token,
    block_number: u64,
) -> ObservationTransferSummary {
    let mut summary = ObservationTransferSummary::default();

    for record in token
        .transfer_tracker
        .weth_transfers
        .values()
        .flatten()
        .filter(|record| record.block_number == block_number)
    {
        let amount = finite_non_negative(record.amount);
        if amount <= 0.0 {
            continue;
        }
        summary.weth_count = summary.weth_count.saturating_add(1);
        summary.weth_volume += amount;
    }

    for record in token
        .transfer_tracker
        .other_denom_transfers
        .values()
        .flatten()
        .filter(|record| record.block_number == block_number)
    {
        let amount = finite_non_negative(record.amount);
        if amount <= 0.0 {
            continue;
        }
        if is_usd_denom(&record.token_address) {
            summary.usd_count = summary.usd_count.saturating_add(1);
            summary.usd_volume += amount;
        } else {
            summary.other_denom_count = summary.other_denom_count.saturating_add(1);
            summary.other_denom_volume += amount;
        }
    }

    summary
}

pub(super) fn token_pool_movement(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
) -> ObservationTokenPoolMovement {
    let context = observation_role_context(token, pool);
    let mut movement = ObservationTokenPoolMovement::default();

    for record in token_transfer_records_at_block(token, block_number) {
        let amount = finite_non_negative(record.amount);
        if amount <= 0.0 {
            continue;
        }
        let from_role = address_role(&record.from_address, &context);
        let to_role = address_role(&record.to_address, &context);
        let from_pool = from_role.is_pool();
        let to_pool = to_role.is_pool();
        if to_pool {
            movement.pool_in += amount;
        }
        if from_pool {
            movement.pool_out += amount;
        }
        if from_pool && to_pool {
            movement.pool_to_pool += amount;
        }
        if from_pool && to_role == ObservationAddressRole::Control {
            movement.pool_to_control += amount;
        }
        if to_role == ObservationAddressRole::TokenContract {
            movement.contract_intake += amount;
        }
        if from_role == ObservationAddressRole::TokenContract && to_pool {
            movement.contract_to_pool += amount;
        }
    }

    let total_supply = token.total_supply_scaled();
    let pool_reserve = pool_token_reserve_ratio_denominator(token, pool, block_number);
    movement.pool_in_to_total_supply_ratio = ratio_if_positive(movement.pool_in, total_supply);
    movement.pool_out_to_total_supply_ratio = ratio_if_positive(movement.pool_out, total_supply);
    movement.pool_to_control_to_total_supply_ratio =
        ratio_if_positive(movement.pool_to_control, total_supply);
    movement.pool_in_to_pool_reserve_ratio = ratio_if_positive(movement.pool_in, pool_reserve);
    movement.pool_out_to_pool_reserve_ratio = ratio_if_positive(movement.pool_out, pool_reserve);
    movement.pool_to_control_to_pool_reserve_ratio =
        ratio_if_positive(movement.pool_to_control, pool_reserve);
    movement.contract_intake_to_total_supply_ratio =
        ratio_if_positive(movement.contract_intake, total_supply);
    movement.contract_to_pool_to_pool_reserve_ratio =
        ratio_if_positive(movement.contract_to_pool, pool_reserve);

    movement
}

pub(super) fn observed_sell_flow(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
) -> ObservationSellFlow {
    let context = observation_role_context(token, pool);
    let denom = pool.identity.denom_address.to_ascii_lowercase();
    let sell_txs: HashSet<String> = token
        .activity
        .transactions_in_block(block_number)
        .into_iter()
        .filter(|transaction| volume_for_denom(&transaction.sell_volume_by_denom, &denom) > 0.0)
        .map(|transaction| transaction.tx_hash.to_ascii_lowercase())
        .collect();
    let mut flow = ObservationSellFlow {
        sell_tx_count: sell_txs.len() as u32,
        ..Default::default()
    };
    if sell_txs.is_empty() {
        return flow;
    }

    for record in token_transfer_records_at_block(token, block_number) {
        if !sell_txs.contains(&record.tx_hash.to_ascii_lowercase()) {
            continue;
        }
        let amount = finite_non_negative(record.amount);
        if amount <= 0.0 {
            continue;
        }
        let from_role = address_role(&record.from_address, &context);
        let to_role = address_role(&record.to_address, &context);
        let to_pool = to_role.is_pool();

        if from_role.is_seller_candidate() {
            flow.seller_token_out += amount;
            if to_pool {
                flow.seller_token_to_pool += amount;
            } else if to_role == ObservationAddressRole::TokenContract {
                flow.seller_token_to_token_contract += amount;
            } else {
                flow.seller_token_to_other += amount;
            }
        }

        if from_role == ObservationAddressRole::TokenContract && to_pool {
            flow.token_contract_to_pool += amount;
        }
    }

    flow.seller_to_pool_ratio =
        ratio_if_positive(flow.seller_token_to_pool, Some(flow.seller_token_out));
    flow.seller_to_token_contract_ratio = ratio_if_positive(
        flow.seller_token_to_token_contract,
        Some(flow.seller_token_out),
    );
    flow.seller_to_other_ratio =
        ratio_if_positive(flow.seller_token_to_other, Some(flow.seller_token_out));
    flow.token_contract_to_pool_reserve_ratio = ratio_if_positive(
        flow.token_contract_to_pool,
        pool_token_reserve_ratio_denominator(token, pool, block_number),
    );
    flow.token_contract_to_pool_seller_out_ratio =
        ratio_if_positive(flow.token_contract_to_pool, Some(flow.seller_token_out));
    flow.has_taxed_sell_pattern = flow.sell_tx_count > 0
        && (flow.seller_to_pool_ratio.is_some_and(|ratio| ratio < 0.5)
            || flow
                .seller_to_token_contract_ratio
                .is_some_and(|ratio| ratio > 0.2)
            || flow.token_contract_to_pool > 0.0);
    flow
}

pub(super) fn token_transfer_records_at_block(
    token: &ERC20Token,
    block_number: u64,
) -> impl Iterator<Item = &TokenTransferRecord> {
    token
        .transfer_tracker
        .erc20_transfers
        .values()
        .flatten()
        .filter(move |record| record.block_number == block_number)
}

pub(super) fn denom_transfer_records_at_block(
    token: &ERC20Token,
    block_number: u64,
) -> impl Iterator<Item = &TokenTransferRecord> {
    token
        .transfer_tracker
        .weth_transfers
        .values()
        .flatten()
        .chain(
            token
                .transfer_tracker
                .other_denom_transfers
                .values()
                .flatten(),
        )
        .filter(move |record| record.block_number == block_number)
}

pub(super) fn token_approval_count_at_block(token: &ERC20Token, block_number: u64) -> u32 {
    token
        .transfer_tracker
        .approvals
        .iter()
        .filter(|approval| {
            approval.block_number == block_number
                && approval
                    .token_address
                    .eq_ignore_ascii_case(&token.contract_address)
        })
        .count() as u32
}
