use crate::erc20::ERC20Token;
use crate::pools::BasePool;
use crate::state::{TokenTransferFromCallRecord, TokenTransferRecord};
use crate::token_analytics::{ObservationTokenPoolMovement, TokenControlFeatures};

use super::roles::{
    address_role, observation_role_context, ObservationAddressRole, ObservationRoleContext,
};
use super::utils::{
    finite_non_negative, is_lp_burn_holder, pool_token_reserve_ratio_denominator, ratio_if_positive,
};

pub(super) fn token_control_features(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
    token_pool_movement: &ObservationTokenPoolMovement,
) -> TokenControlFeatures {
    let context = observation_role_context(token, pool);
    let renouncement_block = token.authority_tracker.renouncement_block;
    let total_supply = token.total_supply_scaled();
    let pool_reserve = pool_token_reserve_ratio_denominator(token, pool, block_number);
    let mut features = TokenControlFeatures {
        control_transfer_from_count_as_of: Some(0),
        control_transfer_from_count_in_block: Some(0),
        control_transfer_from_seen_as_of: Some(false),
        control_transfer_from_in_block: Some(false),
        control_transfer_from_after_renounce_seen_as_of: Some(false),
        control_transfer_from_after_renounce_in_block: Some(false),
        control_transfer_from_holder_to_burn_seen_as_of: Some(false),
        control_transfer_from_holder_to_burn_in_block: Some(false),
        control_transfer_from_pair_seen_as_of: Some(false),
        control_transfer_from_pair_in_block: Some(false),
        control_transfer_from_without_transfer_log_seen_as_of: Some(false),
        control_transfer_from_without_transfer_log_in_block: Some(false),
        pair_token_to_control_seen_as_of: Some(false),
        pair_token_to_control_in_block: Some(token_pool_movement.pool_to_control > 0.0),
        pair_token_to_control_amount_in_block: Some(token_pool_movement.pool_to_control),
        pair_token_to_control_to_total_supply_ratio: token_pool_movement
            .pool_to_control_to_total_supply_ratio,
        pair_token_to_control_to_pool_reserve_ratio: token_pool_movement
            .pool_to_control_to_pool_reserve_ratio,
        pair_balance_backdoor_signal_seen_as_of: Some(false),
        pair_balance_backdoor_signal_in_block: Some(false),
        feature_scope: Some("control_transfer_from_calls_and_pair_token_transfers".to_string()),
        ..Default::default()
    };

    for call in token
        .transfer_tracker
        .transfer_from_calls
        .iter()
        .filter(|call| call.block_number <= block_number)
    {
        if !caller_is_control(call, &context) {
            continue;
        }

        let in_block = call.block_number == block_number;
        bump_count(&mut features.control_transfer_from_count_as_of);
        if in_block {
            bump_count(&mut features.control_transfer_from_count_in_block);
        }
        set_bool(&mut features.control_transfer_from_seen_as_of);
        set_if(&mut features.control_transfer_from_in_block, in_block);
        features.last_control_transfer_from_block =
            max_block(features.last_control_transfer_from_block, call.block_number);

        let after_renounce =
            renouncement_block.is_some_and(|renounced| call.block_number >= renounced);
        let holder_to_burn = is_lp_burn_holder(&call.to_address);
        let from_pair = address_role(&call.from_address, &context).is_pool();
        let to_control =
            address_role(&call.to_address, &context) == ObservationAddressRole::Control;
        let from_pair_to_control = from_pair && to_control;
        let without_transfer_log = call.emitted_transfer_count == 0;

        mark_signal(
            after_renounce,
            in_block,
            &mut features.control_transfer_from_after_renounce_seen_as_of,
            &mut features.control_transfer_from_after_renounce_in_block,
        );
        mark_signal(
            holder_to_burn,
            in_block,
            &mut features.control_transfer_from_holder_to_burn_seen_as_of,
            &mut features.control_transfer_from_holder_to_burn_in_block,
        );
        mark_signal(
            from_pair_to_control,
            in_block,
            &mut features.control_transfer_from_pair_seen_as_of,
            &mut features.control_transfer_from_pair_in_block,
        );
        mark_signal(
            without_transfer_log,
            in_block,
            &mut features.control_transfer_from_without_transfer_log_seen_as_of,
            &mut features.control_transfer_from_without_transfer_log_in_block,
        );

        let backdoor_signal =
            after_renounce || holder_to_burn || from_pair_to_control || without_transfer_log;
        mark_backdoor_signal(&mut features, backdoor_signal, in_block, call.block_number);
    }

    for record in token
        .transfer_tracker
        .erc20_transfers
        .values()
        .flatten()
        .filter(|record| record.block_number <= block_number)
    {
        if !pair_token_to_control(record, &context) {
            continue;
        }
        set_bool(&mut features.pair_token_to_control_seen_as_of);
        features.last_pair_token_to_control_block = max_block(
            features.last_pair_token_to_control_block,
            record.block_number,
        );
        mark_backdoor_signal(
            &mut features,
            true,
            record.block_number == block_number,
            record.block_number,
        );
    }

    let pool_transfer_amount = finite_non_negative(token_pool_movement.pool_to_control);
    features.pair_token_to_control_amount_in_block = Some(pool_transfer_amount);
    features.pair_token_to_control_to_total_supply_ratio =
        ratio_if_positive(pool_transfer_amount, total_supply);
    features.pair_token_to_control_to_pool_reserve_ratio =
        ratio_if_positive(pool_transfer_amount, pool_reserve);
    features.with_as_of_offsets(block_number)
}

fn caller_is_control(call: &TokenTransferFromCallRecord, context: &ObservationRoleContext) -> bool {
    address_role(&call.caller, context) == ObservationAddressRole::Control
}

fn pair_token_to_control(record: &TokenTransferRecord, context: &ObservationRoleContext) -> bool {
    address_role(&record.from_address, context).is_pool()
        && address_role(&record.to_address, context) == ObservationAddressRole::Control
        && finite_non_negative(record.amount) > 0.0
}

fn bump_count(value: &mut Option<u32>) {
    *value = Some(value.unwrap_or_default().saturating_add(1));
}

fn set_bool(value: &mut Option<bool>) {
    *value = Some(true);
}

fn set_if(value: &mut Option<bool>, condition: bool) {
    if condition {
        set_bool(value);
    }
}

fn mark_signal(
    signal: bool,
    in_block: bool,
    seen_as_of: &mut Option<bool>,
    seen_in_block: &mut Option<bool>,
) {
    if signal {
        set_bool(seen_as_of);
        set_if(seen_in_block, in_block);
    }
}

fn mark_backdoor_signal(
    features: &mut TokenControlFeatures,
    signal: bool,
    in_block: bool,
    block_number: u64,
) {
    if !signal {
        return;
    }
    set_bool(&mut features.pair_balance_backdoor_signal_seen_as_of);
    set_if(
        &mut features.pair_balance_backdoor_signal_in_block,
        in_block,
    );
    features.last_pair_balance_backdoor_signal_block = max_block(
        features.last_pair_balance_backdoor_signal_block,
        block_number,
    );
}

fn max_block(current: Option<u64>, candidate: u64) -> Option<u64> {
    Some(current.map_or(candidate, |current| current.max(candidate)))
}
