use alloy_primitives::{Address, U256};
use reth_chain_query::to_checksum_address;
use serde_json::{json, Value};

use crate::simulator::{ExactVaultBuySimulationResult, SimulationResult};

use super::{
    context::SignalPoolContext,
    formatting::{
        denom_symbol_for_address, format_f64_decimal, normalize_evidence_protocol, parse_address,
        pool_type_label, tax_value, u256_to_decimal_string,
    },
};

pub(super) fn build_mempool_entry_evidence(
    result: &SimulationResult,
    pool_context: Option<&SignalPoolContext>,
) -> Option<Value> {
    let pool_result = result.pool_viability_result.as_ref()?;
    let protocol = result
        .pool_type
        .as_deref()
        .map(normalize_evidence_protocol)
        .unwrap_or_else(|| pool_type_label(pool_result.pool_type).to_string());
    let denom_address = pool_context
        .map(|context| context.denom_address.clone())
        .or_else(|| projected_v2_denom_address(pool_result));
    let denom_decimals = pool_context
        .and_then(|context| context.denom_decimals)
        .or_else(|| {
            denom_address
                .as_deref()
                .and_then(|address| super::formatting::known_denom_decimals("WETH", address))
        })
        .unwrap_or(18);
    let token_decimals = exact_entry_token_decimals(result)
        .or_else(|| pool_context.and_then(|context| context.token_decimals))?;
    let denom_symbol = pool_context
        .map(|context| context.denom_currency.clone())
        .or_else(|| {
            denom_address
                .as_deref()
                .map(|address| denom_symbol_for_address(address))
        });
    let (projected_denom_reserve, projected_token_reserve) = projected_v2_reserves(
        pool_result,
        denom_address.as_deref(),
        denom_decimals,
        token_decimals,
    );
    let denom_reserve = projected_denom_reserve
        .or_else(|| pool_context.map(|context| format_f64_decimal(context.denom_reserve)))
        .unwrap_or_else(|| "0".to_string());
    let token_reserve = projected_token_reserve
        .or_else(|| pool_context.map(|context| format_f64_decimal(context.token_reserve)))
        .unwrap_or_else(|| "0".to_string());
    let latest_block = pool_context
        .map(|context| context.latest_block.max(pool_result.block_number))
        .unwrap_or(pool_result.block_number);
    let dependency_hashes = dependency_tx_hashes(pool_result, &result.request.tx.hash);
    let (pool_creation_block, pool_creation_block_source) =
        projected_pool_creation_block(pool_context, pool_result);

    let (vault_buy_simulation, audit_exact_vault_calldata) =
        if let Some(exact) = result.exact_vault_buy_result.as_ref() {
            (exact_vault_buy_simulation_json(exact), true)
        } else {
            (
                json!({
                    "route": "pool_buy_sell_probe",
                    "would_revert": !pool_result.can_buy,
                    "gas_used": pool_result.buy_transaction.fees.gas_used,
                    "eth_spent_wei": pool_result.denom_spent.to_string(),
                    "tokens_received_raw": pool_result.tokens_received.to_string(),
                    "metadata": {
                        "exact_vault_calldata": false,
                        "source": "mempool_processor.pool_buy_sell_result",
                        "failure_reason": pool_result.failure_reason
                    }
                }),
                false,
            )
        };

    Some(json!({
        "evidence_version": "mempool_entry_evidence_v1",
        "base_block": pool_result.block_number,
        "simulated_block": pool_result.block_number,
        "simulated_at": chrono::Utc::now().to_rfc3339(),
        "dependency_tx_hashes": dependency_hashes,
        "dependency_fee_metadata": {
            "tail_after_tx_hash": result.request.tx.hash,
            "dependency_max_fee_per_gas_wei": tx_fee_field_decimal_string(&result.request.tx.data, "maxFeePerGas"),
            "dependency_priority_fee_wei": tx_fee_field_decimal_string(&result.request.tx.data, "maxPriorityFeePerGas"),
            "dependency_gas_price_wei": result.request.tx.gas_price.as_ref().map(|value| value.to_string()),
            "dependency_value_wei": result.request.tx.value.to_string()
        },
        "projected_pool": {
            "protocol": protocol,
            "denom_address": denom_address,
            "denom_symbol": denom_symbol,
            "token_decimals": token_decimals,
            "denom_reserve": denom_reserve,
            "token_reserve": token_reserve,
            "price_ratio_to_initial": Value::Null,
            "pool_creation_block": pool_creation_block,
            "pool_creation_block_source": pool_creation_block_source,
            "latest_block": latest_block,
            "can_buy": pool_result.can_buy && pool_context.map(|context| context.can_buy).unwrap_or(true),
            "can_sell": pool_result.can_sell && pool_context.map(|context| context.can_sell).unwrap_or(true),
            "is_scam": pool_context.map(|context| context.is_scam).unwrap_or(false)
        },
        "viability": {
            "can_buy": pool_result.can_buy,
            "can_approve": pool_result.can_approve,
            "can_sell": pool_result.can_sell,
            "buy_tax_percent": tax_value(pool_result.buy_tax_percent),
            "sell_tax_percent": tax_value(pool_result.sell_tax_percent)
        },
        "vault_buy_simulation": vault_buy_simulation,
        "strategy_neutral_flags": {
            "can_buy": pool_result.can_buy,
            "can_approve": pool_result.can_approve,
            "can_sell": pool_result.can_sell
        },
        "audit": {
            "source": "mempool_processor",
            "pool_buy_sell_probe": true,
            "exact_vault_calldata": audit_exact_vault_calldata,
            "prior_transaction_count": pool_result.prior_transactions.len()
        }
    }))
}

fn projected_pool_creation_block(
    pool_context: Option<&SignalPoolContext>,
    pool_result: &tx_processor::PoolBuySellSimulationResult,
) -> (Option<u64>, Option<&'static str>) {
    projected_pool_creation_block_from_parts(
        pool_context.and_then(|context| context.pool_creation_block),
        v2_pair_creation_block(pool_result),
        trading_enabled_block(pool_result),
    )
}

fn projected_pool_creation_block_from_parts(
    tracked_block: Option<u64>,
    pending_pair_created_block: Option<u64>,
    pending_trading_enabled_block: Option<u64>,
) -> (Option<u64>, Option<&'static str>) {
    if let Some(block) = tracked_block {
        return (Some(block), Some("tracked_pool_trading_enabled_block"));
    }
    if let Some(block) = pending_pair_created_block {
        return (Some(block), Some("pending_uniswap_v2_pair_created_event"));
    }
    if let Some(block) = pending_trading_enabled_block {
        return (Some(block), Some("pending_trading_enabled_event"));
    }
    (None, None)
}

fn exact_vault_buy_simulation_json(result: &ExactVaultBuySimulationResult) -> Value {
    json!({
        "route": result.route,
        "vault_address": to_checksum_address(&result.vault_address),
        "owner_address": to_checksum_address(&result.owner_address),
        "chain_id": result.chain_id,
        "would_revert": result.would_revert,
        "gas_used": result.gas_used,
        "eth_spent_wei": result.eth_spent_wei.to_string(),
        "tokens_received_raw": result.tokens_received_raw.to_string(),
        "metadata": {
            "exact_vault_calldata": true,
            "source": "mempool_processor.exact_vault_buy_result",
            "provider": result.metadata.get("provider").cloned(),
            "event": result.metadata.get("event").cloned(),
            "failure_reason": result.revert_reason,
            "simulated_block": result.simulated_block,
            "dependency_tx_hashes": result.dependency_tx_hashes,
            "tail_after_tx_hash": result.tail_after_tx_hash,
            "buy_amount_wei": result.buy_amount_wei.to_string(),
            "min_tokens_out": result.min_tokens_out.to_string(),
            "expected_tokens_raw": result.expected_tokens_raw.map(|value| value.to_string()),
            "calldata_builder": result.metadata.get("calldata_builder").cloned(),
            "deadline_policy": result.metadata.get("deadline_policy").cloned(),
            "quote_gas_used": result.metadata.get("quote_gas_used").cloned(),
            "quote_tokens_received_raw": result.metadata.get("quote_tokens_received_raw").cloned(),
            "token_decimals": result.metadata.get("token_decimals").cloned(),
            "denom_decimals": result.metadata.get("denom_decimals").cloned(),
        }
    })
}

fn projected_v2_denom_address(
    pool_result: &tx_processor::PoolBuySellSimulationResult,
) -> Option<String> {
    let (token0, token1) = v2_pair_tokens(pool_result)?;
    let denom = if token0 == pool_result.token_address {
        token1
    } else {
        token0
    };
    Some(to_checksum_address(&denom))
}

fn projected_v2_reserves(
    pool_result: &tx_processor::PoolBuySellSimulationResult,
    denom_address: Option<&str>,
    denom_decimals: u8,
    token_decimals: u8,
) -> (Option<String>, Option<String>) {
    let Some((reserve0, reserve1)) = latest_v2_sync(pool_result) else {
        return (None, None);
    };
    let denom = denom_address.and_then(parse_address);
    let denom_is_token0 = denom
        .map(|denom| {
            v2_pair_tokens(pool_result)
                .map(|(token0, _)| token0 == denom)
                .unwrap_or_else(|| denom < pool_result.token_address)
        })
        .unwrap_or(false);
    let (denom_reserve_raw, token_reserve_raw) = if denom_is_token0 {
        (reserve0, reserve1)
    } else {
        (reserve1, reserve0)
    };

    (
        Some(u256_to_decimal_string(denom_reserve_raw, denom_decimals)),
        Some(u256_to_decimal_string(token_reserve_raw, token_decimals)),
    )
}

fn exact_entry_token_decimals(result: &SimulationResult) -> Option<u8> {
    result
        .exact_vault_buy_result
        .as_ref()
        .and_then(|exact| exact.metadata.get("token_decimals"))
        .and_then(|value| value.as_u64())
        .and_then(|value| u8::try_from(value).ok())
}

fn v2_pair_tokens(
    pool_result: &tx_processor::PoolBuySellSimulationResult,
) -> Option<(Address, Address)> {
    for tx in &pool_result.prior_transactions {
        for event in &tx.uniswap_v2_pair_created_events {
            if event.pair_address == pool_result.pool_address {
                return Some((event.token0, event.token1));
            }
        }
    }
    None
}

fn v2_pair_creation_block(pool_result: &tx_processor::PoolBuySellSimulationResult) -> Option<u64> {
    for tx in &pool_result.prior_transactions {
        for event in &tx.uniswap_v2_pair_created_events {
            if event.pair_address == pool_result.pool_address {
                return Some(tx.block_number);
            }
        }
    }
    None
}

fn trading_enabled_block(pool_result: &tx_processor::PoolBuySellSimulationResult) -> Option<u64> {
    for tx in &pool_result.prior_transactions {
        for event in &tx.trading_enabled_events {
            if event.token_address == pool_result.token_address {
                return Some(event.block_number);
            }
        }
    }
    None
}

fn latest_v2_sync(pool_result: &tx_processor::PoolBuySellSimulationResult) -> Option<(U256, U256)> {
    let mut latest = None;
    for tx in &pool_result.prior_transactions {
        for event in &tx.uniswap_v2_syncs {
            if event.pair_address == pool_result.pool_address {
                latest = Some((event.reserve0, event.reserve1));
            }
        }
    }
    latest
}

fn dependency_tx_hashes(
    pool_result: &tx_processor::PoolBuySellSimulationResult,
    fallback_hash: &str,
) -> Vec<String> {
    let mut hashes = pool_result
        .prior_transactions
        .iter()
        .map(|tx| format!("{:#x}", tx.hash))
        .collect::<Vec<_>>();
    if hashes.is_empty() {
        hashes.push(fallback_hash.to_string());
    }
    hashes
}

fn tx_fee_field_decimal_string(tx_data: &Value, key: &str) -> Option<String> {
    let value = tx_data.get(key)?.as_str()?;
    parse_rpc_quantity(value).map(|value| value.to_string())
}

fn parse_rpc_quantity(value: &str) -> Option<U256> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(hex) = trimmed.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).ok()
    } else {
        U256::from_str_radix(trimmed, 10).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::projected_pool_creation_block_from_parts;

    #[test]
    fn projected_pool_creation_block_prefers_tracked_context() {
        assert_eq!(
            projected_pool_creation_block_from_parts(Some(100), Some(101), Some(102)),
            (Some(100), Some("tracked_pool_trading_enabled_block"))
        );
    }

    #[test]
    fn projected_pool_creation_block_uses_pending_pair_created_event() {
        assert_eq!(
            projected_pool_creation_block_from_parts(None, Some(101), Some(102)),
            (Some(101), Some("pending_uniswap_v2_pair_created_event"))
        );
    }

    #[test]
    fn projected_pool_creation_block_uses_pending_trading_enabled_event_without_pair_created() {
        assert_eq!(
            projected_pool_creation_block_from_parts(None, None, Some(102)),
            (Some(102), Some("pending_trading_enabled_event"))
        );
    }
}
