use crate::erc20::ERC20Token;
use crate::pools::uniswap::v2::LPHolderSnapshot;
use crate::token_analytics::LpControlFeatures;
use serde_json::Value;

use super::events::{
    event_amount, event_block_number, event_bool, event_string, event_timestamp,
    event_transfer_from_address, event_transfer_to_address, event_transfers_to_burn_address,
    event_tx_hash,
};
use super::utils::{is_lp_burn_holder, normalize_address, same_optional_address};

pub(super) fn lp_control_features(
    token: &ERC20Token,
    pool_address: &str,
    as_of_block: u64,
) -> LpControlFeatures {
    let pool_address = normalize_address(pool_address);
    if let Some(pool) = token.v2_pools.get(&pool_address) {
        let holders = pool.lp_holders();
        let holder_split = split_lp_holders(&holders);
        let top_holder = top_regular_lp_holder(&holder_split.regular);
        let total_supply = pool.lp_tracker.total_supply;
        let regular_balance = lp_holder_balance_sum(&holder_split.regular);
        let burned_balance = lp_holder_balance_sum(&holder_split.burned);
        let regular_share = lp_holder_share_sum(&holder_split.regular);
        let burned_share = lp_holder_share_sum(&holder_split.burned);
        let router_approved = lp_router_approved_amount(&holder_split.regular);
        let burn_transfer_stats = lp_burn_transfer_stats(&pool.lp_tracker.transfers, as_of_block);
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
        let max_approval_amount = max_lp_approval_amount(holder_split.regular.iter().copied());
        return LpControlFeatures {
            lp_total_supply: Some(total_supply),
            lp_holder_count: Some(holders.len() as u32),
            lp_regular_holder_count: Some(holder_split.regular.len() as u32),
            lp_burn_holder_count: Some(holder_split.burned.len() as u32),
            lp_regular_holder_balance: Some(regular_balance),
            lp_burned_balance: Some(burned_balance),
            lp_regular_holder_share_pct: Some(regular_share),
            lp_burned_share_pct: Some(burned_share),
            lp_top_holder_share_pct: top_holder.map(|holder| holder.share),
            lp_top_regular_holder_share_pct: top_holder.map(|holder| holder.share),
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
            lp_holders_with_approvals_count: Some(lp_holders_with_approvals_count(
                &holder_split.regular,
            )),
            lp_approved_spender_count_as_of: Some(lp_approval_spender_count(&holder_split.regular)),
            lp_approved_pct_as_of: lp_approved_pct_from_max(total_supply, max_approval_amount),
            lp_approved_to_router: Some(router_approved),
            lp_approved_to_router_pct: Some(lp_pct_of_total(total_supply, router_approved)),
            lp_router_approved_pct_as_of: Some(lp_pct_of_total(total_supply, router_approved)),
            lp_router_approval_seen_as_of: Some(router_approved > 0.0),
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
            lp_burn_transfer_count_as_of: Some(burn_transfer_stats.count_as_of),
            lp_burn_transfer_count_in_block: Some(burn_transfer_stats.count_in_block),
            lp_burn_transfer_amount_as_of: Some(burn_transfer_stats.amount_as_of),
            lp_burn_transfer_amount_in_block: Some(burn_transfer_stats.amount_in_block),
            last_lp_burn_block: burn_transfer_stats.last_block,
            last_lp_burn_timestamp: burn_transfer_stats.last_timestamp,
            last_lp_burn_tx: burn_transfer_stats.last_tx_hash,
            last_lp_burn_from: burn_transfer_stats.last_from,
            last_lp_burn_to: burn_transfer_stats.last_to,
            feature_scope: Some("uniswap_v2_lp_tracker".to_string()),
            ..Default::default()
        }
        .with_as_of_offsets(as_of_block);
    }

    if let Some(pool) = token.v3_pools.get(&pool_address) {
        let holders = pool.lp_holders();
        let holder_split = split_lp_holders(&holders);
        let top_holder = top_regular_lp_holder(&holder_split.regular);
        let burn_transfer_stats =
            lp_burn_transfer_stats(&pool.liquidity_position_events, as_of_block);
        return LpControlFeatures {
            lp_total_supply: Some(pool.lp_total_supply()),
            lp_holder_count: Some(holders.len() as u32),
            lp_regular_holder_count: Some(holder_split.regular.len() as u32),
            lp_burn_holder_count: Some(holder_split.burned.len() as u32),
            lp_regular_holder_balance: Some(lp_holder_balance_sum(&holder_split.regular)),
            lp_burned_balance: Some(lp_holder_balance_sum(&holder_split.burned)),
            lp_regular_holder_share_pct: Some(lp_holder_share_sum(&holder_split.regular)),
            lp_burned_share_pct: Some(lp_holder_share_sum(&holder_split.burned)),
            lp_top_holder_share_pct: top_holder.map(|holder| holder.share),
            lp_top_regular_holder_share_pct: top_holder.map(|holder| holder.share),
            lp_top_holder_is_creator: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.creator_address.as_deref())
            }),
            lp_top_holder_is_owner: top_holder.and_then(|holder| {
                same_optional_address(Some(&holder.address), token.current_owner().as_deref())
            }),
            lp_burn_transfer_count_as_of: Some(burn_transfer_stats.count_as_of),
            lp_burn_transfer_count_in_block: Some(burn_transfer_stats.count_in_block),
            lp_burn_transfer_amount_as_of: Some(burn_transfer_stats.amount_as_of),
            lp_burn_transfer_amount_in_block: Some(burn_transfer_stats.amount_in_block),
            last_lp_burn_block: burn_transfer_stats.last_block,
            last_lp_burn_timestamp: burn_transfer_stats.last_timestamp,
            last_lp_burn_tx: burn_transfer_stats.last_tx_hash,
            last_lp_burn_from: burn_transfer_stats.last_from,
            last_lp_burn_to: burn_transfer_stats.last_to,
            feature_scope: Some("uniswap_v3_positions".to_string()),
            ..Default::default()
        }
        .with_as_of_offsets(as_of_block);
    }

    if let Some(pool) = token.v4_pools.get(&pool_address) {
        let holders = pool.lp_holders();
        let holder_split = split_lp_holders(&holders);
        let top_holder = top_regular_lp_holder(&holder_split.regular);
        let burn_transfer_stats =
            lp_burn_transfer_stats(&pool.liquidity_position_events, as_of_block);
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
        let max_approval_amount = max_lp_approval_amount(holder_split.regular.iter().copied());
        let router_approved = lp_router_approved_amount(&holder_split.regular);
        return LpControlFeatures {
            lp_total_supply: Some(lp_total_supply),
            lp_holder_count: Some(holders.len() as u32),
            lp_regular_holder_count: Some(holder_split.regular.len() as u32),
            lp_burn_holder_count: Some(holder_split.burned.len() as u32),
            lp_regular_holder_balance: Some(lp_holder_balance_sum(&holder_split.regular)),
            lp_burned_balance: Some(lp_holder_balance_sum(&holder_split.burned)),
            lp_regular_holder_share_pct: Some(lp_holder_share_sum(&holder_split.regular)),
            lp_burned_share_pct: Some(lp_holder_share_sum(&holder_split.burned)),
            lp_top_holder_share_pct: top_holder.map(|holder| holder.share),
            lp_top_regular_holder_share_pct: top_holder.map(|holder| holder.share),
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
            lp_holders_with_approvals_count: Some(lp_holders_with_approvals_count(
                &holder_split.regular,
            )),
            lp_approved_spender_count_as_of: Some(lp_approval_spender_count(&holder_split.regular)),
            lp_approved_pct_as_of: lp_approved_pct_from_max(lp_total_supply, max_approval_amount),
            lp_approved_to_router: Some(router_approved),
            lp_approved_to_router_pct: Some(lp_pct_of_total(lp_total_supply, router_approved)),
            lp_router_approved_pct_as_of: Some(lp_pct_of_total(lp_total_supply, router_approved)),
            lp_router_approval_seen_as_of: Some(router_approved > 0.0),
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
            lp_burn_transfer_count_as_of: Some(burn_transfer_stats.count_as_of),
            lp_burn_transfer_count_in_block: Some(burn_transfer_stats.count_in_block),
            lp_burn_transfer_amount_as_of: Some(burn_transfer_stats.amount_as_of),
            lp_burn_transfer_amount_in_block: Some(burn_transfer_stats.amount_in_block),
            last_lp_burn_block: burn_transfer_stats.last_block,
            last_lp_burn_timestamp: burn_transfer_stats.last_timestamp,
            last_lp_burn_tx: burn_transfer_stats.last_tx_hash,
            last_lp_burn_from: burn_transfer_stats.last_from,
            last_lp_burn_to: burn_transfer_stats.last_to,
            feature_scope: Some("uniswap_v4_positions".to_string()),
            ..Default::default()
        }
        .with_as_of_offsets(as_of_block);
    }

    LpControlFeatures::default().with_as_of_offsets(as_of_block)
}

struct LpHolderSplit<'a> {
    regular: Vec<&'a LPHolderSnapshot>,
    burned: Vec<&'a LPHolderSnapshot>,
}

fn split_lp_holders(holders: &[LPHolderSnapshot]) -> LpHolderSplit<'_> {
    let mut regular = Vec::new();
    let mut burned = Vec::new();
    for holder in holders {
        if is_lp_burn_holder(&holder.address) {
            burned.push(holder);
        } else {
            regular.push(holder);
        }
    }
    LpHolderSplit { regular, burned }
}

fn top_regular_lp_holder<'a>(holders: &[&'a LPHolderSnapshot]) -> Option<&'a LPHolderSnapshot> {
    holders
        .iter()
        .copied()
        .max_by(|left, right| left.share.total_cmp(&right.share))
}

fn lp_holder_balance_sum(holders: &[&LPHolderSnapshot]) -> f64 {
    let sum = holders
        .iter()
        .map(|holder| holder.balance)
        .filter(|balance| balance.is_finite() && *balance > 0.0)
        .sum::<f64>();
    positive_or_zero(sum)
}

fn lp_holder_share_sum(holders: &[&LPHolderSnapshot]) -> f64 {
    let sum = holders
        .iter()
        .map(|holder| holder.share)
        .filter(|share| share.is_finite() && *share > 0.0)
        .sum::<f64>();
    positive_or_zero(sum).min(100.0)
}

fn lp_holders_with_approvals_count(holders: &[&LPHolderSnapshot]) -> u32 {
    holders
        .iter()
        .filter(|holder| !holder.approvals.is_empty())
        .count() as u32
}

fn lp_approval_spender_count(holders: &[&LPHolderSnapshot]) -> u32 {
    holders
        .iter()
        .map(|holder| holder.approvals.len())
        .sum::<usize>() as u32
}

fn lp_router_approved_amount(holders: &[&LPHolderSnapshot]) -> f64 {
    let sum = holders
        .iter()
        .map(|holder| {
            holder
                .approvals
                .values()
                .filter(|approval| approval.is_router)
                .map(|approval| approval.amount.min(holder.balance))
                .filter(|amount| amount.is_finite() && *amount > 0.0)
                .sum::<f64>()
        })
        .sum::<f64>();
    positive_or_zero(sum)
}

fn positive_or_zero(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn lp_pct_of_total(total: f64, amount: f64) -> f64 {
    if !total.is_finite() || total <= 0.0 || !amount.is_finite() || amount <= 0.0 {
        0.0
    } else {
        (amount.min(total) / total) * 100.0
    }
}

#[derive(Default)]
struct LpBurnTransferStats {
    count_as_of: u32,
    count_in_block: u32,
    amount_as_of: f64,
    amount_in_block: f64,
    last_block: Option<u64>,
    last_timestamp: Option<u64>,
    last_tx_hash: Option<String>,
    last_from: Option<String>,
    last_to: Option<String>,
}

fn lp_burn_transfer_stats(events: &[Value], as_of_block: u64) -> LpBurnTransferStats {
    let mut stats = LpBurnTransferStats::default();
    for event in events {
        let Some(block) = event_block_number(event) else {
            continue;
        };
        if block > as_of_block || !event_transfers_to_burn_address(event) {
            continue;
        }

        let amount = event_amount(event).unwrap_or(0.0);
        stats.count_as_of = stats.count_as_of.saturating_add(1);
        stats.amount_as_of += amount;
        if block == as_of_block {
            stats.count_in_block = stats.count_in_block.saturating_add(1);
            stats.amount_in_block += amount;
        }
        if stats
            .last_block
            .map_or(true, |last_block| block >= last_block)
        {
            stats.last_block = Some(block);
            stats.last_timestamp = event_timestamp(event);
            stats.last_tx_hash = event_tx_hash(event);
            stats.last_from = event_transfer_from_address(event);
            stats.last_to = event_transfer_to_address(event);
        }
    }
    stats
}

fn max_lp_approval_amount<'a>(
    holders: impl IntoIterator<Item = &'a LPHolderSnapshot>,
) -> Option<f64> {
    holders
        .into_iter()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn splits_burned_lp_from_regular_holders() {
        let holders = vec![
            LPHolderSnapshot {
                address: "0x000000000000000000000000000000000000dEaD".to_string(),
                balance: 95.0,
                share: 95.0,
                approvals: Default::default(),
            },
            LPHolderSnapshot {
                address: "0x1111111111111111111111111111111111111111".to_string(),
                balance: 5.0,
                share: 5.0,
                approvals: Default::default(),
            },
        ];

        let split = split_lp_holders(&holders);
        let top_regular = top_regular_lp_holder(&split.regular).unwrap();

        assert_eq!(split.regular.len(), 1);
        assert_eq!(split.burned.len(), 1);
        assert_eq!(top_regular.address, holders[1].address);
        assert_eq!(lp_holder_share_sum(&split.burned), 95.0);
    }

    #[test]
    fn tracks_lp_burn_transfer_stats() {
        let events = vec![
            json!({
                "block_number": 10,
                "tx_hash": "0xdead",
                "from_address": "0x1111111111111111111111111111111111111111",
                "to_address": "0x000000000000000000000000000000000000dEaD",
                "amount": 7.0
            }),
            json!({
                "block_number": 11,
                "tx_hash": "0xzero",
                "from_address": "0x1111111111111111111111111111111111111111",
                "to_address": "0x0000000000000000000000000000000000000000",
                "amount": 5.0
            }),
        ];

        let stats = lp_burn_transfer_stats(&events, 11);
        assert_eq!(stats.count_as_of, 2);
        assert_eq!(stats.count_in_block, 1);
        assert_eq!(stats.amount_as_of, 12.0);
        assert_eq!(stats.amount_in_block, 5.0);
        assert_eq!(stats.last_tx_hash.as_deref(), Some("0xzero"));
    }
}
