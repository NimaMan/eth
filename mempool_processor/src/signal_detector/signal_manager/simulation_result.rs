use reth_chain_query::to_checksum_address;

use crate::simulator::SimulationResult;

pub(super) fn token_address_from_simulation_result(result: &SimulationResult) -> Option<String> {
    if let Some(token_address) = &result.token_address {
        return Some(to_checksum_address(token_address));
    }

    match &result.request.category {
        crate::tx_router::TransactionCategory::CreatorTransaction { target_token, .. } => {
            target_token.clone()
        }
        crate::tx_router::TransactionCategory::ContractCreation {
            contract_address, ..
        } => Some(contract_address.clone()),
        _ => None,
    }
}

pub(super) fn is_replay_context_mismatch(error: &str) -> bool {
    error.contains("Setup transaction replay failed") && error.contains("mined receipt succeeded")
}

pub(super) fn creator_address_from_simulation_result(result: &SimulationResult) -> String {
    match &result.request.category {
        crate::tx_router::TransactionCategory::CreatorTransaction { creator, .. } => {
            creator.clone()
        }
        _ => "unknown".to_string(),
    }
}

pub(super) fn format_buy_sell_error(result: &SimulationResult, error: &str) -> String {
    format!(
        "tx={} | token={} | pool={} | pool_type={} | can_buy={} | can_approve={} | can_sell={} | error={}",
        result.request.tx.hash,
        token_address_from_simulation_result(result).unwrap_or_else(|| "unknown".to_string()),
        result
            .pool_address
            .map(|addr| to_checksum_address(&addr))
            .unwrap_or_else(|| "unknown".to_string()),
        result.pool_type.as_deref().unwrap_or("unknown"),
        result
            .buy_sell_result()
            .map(|buy_sell| buy_sell.can_buy)
            .unwrap_or(false),
        result
            .buy_sell_result()
            .map(|buy_sell| buy_sell.can_approve)
            .unwrap_or(false),
        result
            .buy_sell_result()
            .map(|buy_sell| buy_sell.can_sell)
            .unwrap_or(false),
        error
    )
}
