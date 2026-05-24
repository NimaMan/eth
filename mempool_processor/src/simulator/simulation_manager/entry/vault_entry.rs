use alloy_primitives::{keccak256, Address, B256, U256};
use eyre::{eyre, Result, WrapErr};
use serde_json::json;
use tx_processor::{PoolBuySellParameters, PoolType, ProcessedTransaction, UnsignedTxBuilder};
use tx_simulator::{
    tx_builders::build_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens, FullSimulationResult,
    UnsignedTransaction, UnsignedTxChainSimulation,
};

use crate::config::{
    alpha_live_entry_buy_wei_from_config, alpha_live_uniswap_v2_vault_buy_gas_limit_from_config,
    uniswap_v2_trading_vault_from_config, uniswap_v2_trading_vault_owner_from_config,
};

use super::{types::ExactVaultBuySimulationResult, SimulationManager, TxSimulationJob};

const CHAIN_ID_MAINNET: u64 = 1;
const DEFAULT_ALPHA_LIVE_ENTRY_BUY_WEI: &str = "10000000000000000";
const DEFAULT_UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT: u64 = 300_000;
const BOUGHT_V2_SIGNATURE: &str = "BoughtV2(address,uint256,uint256,uint256)";
const SLIPPAGE_BPS_DENOMINATOR: u64 = 10_000;

#[derive(Clone, Copy, Debug)]
struct ExactVaultEntryConfig {
    vault_address: Address,
    owner_address: Address,
    buy_amount_wei: U256,
    buy_gas_limit: u64,
    chain_id: u64,
}

#[derive(Clone, Copy, Debug)]
struct BoughtV2Fill {
    eth_spent: U256,
    tokens_received: U256,
}

impl SimulationManager {
    pub(in crate::simulator::simulation_manager) async fn simulate_exact_vault_entry_buy(
        &self,
        request: &TxSimulationJob,
        config: &PoolBuySellParameters,
        replay_sequence: &[ProcessedTransaction],
    ) -> Option<ExactVaultBuySimulationResult> {
        if !matches!(config.pool_type, PoolType::UniswapV2) {
            return None;
        }

        match self
            .simulate_exact_vault_entry_buy_inner(request, config, replay_sequence)
            .await
        {
            Ok(result) => Some(result),
            Err(err) => {
                tracing::warn!(
                    target: "mempool_exact_vault_entry",
                    tx_hash = %request.tx.hash,
                    token = %config.token_address,
                    pool = %config.pool_address,
                    error = %err,
                    "exact V2 vault entry simulation unavailable"
                );
                None
            }
        }
    }

    async fn simulate_exact_vault_entry_buy_inner(
        &self,
        request: &TxSimulationJob,
        config: &PoolBuySellParameters,
        replay_sequence: &[ProcessedTransaction],
    ) -> Result<ExactVaultBuySimulationResult> {
        let vault_config = exact_vault_entry_config()?;
        let block = config.block_number.ok_or_else(|| {
            eyre!("exact V2 vault entry simulation requires an explicit base block")
        })?;
        let dependency_hashes = replay_sequence
            .iter()
            .map(|tx| format!("{:#x}", tx.hash))
            .collect::<Vec<_>>();
        let tail_after_tx_hash = Some(request.tx.hash.clone());
        let mut projected_chain = self
            .projected_chain_after_replay(block, replay_sequence)
            .await
            .wrap_err("failed to build projected chain for exact V2 vault entry")?;
        let base_fee = projected_chain.block_base_fee().unwrap_or(1);

        let quote_tx = build_vault_buy_tx(
            &vault_config,
            config.token_address,
            U256::ZERO,
            u64::MAX,
            base_fee,
        );
        let quote_result = projected_chain.clone().step_with_trace(quote_tx).await;
        let quote_result = match quote_result {
            Ok(result) => result,
            Err(err) => {
                return Ok(failed_exact_vault_result(
                    vault_config,
                    block,
                    dependency_hashes,
                    tail_after_tx_hash,
                    U256::ZERO,
                    None,
                    None,
                    Some(format!("quote_simulation_error: {err}")),
                ));
            }
        };

        let quote_fill = if quote_result.success {
            extract_bought_v2_fill(
                &quote_result,
                vault_config.vault_address,
                config.token_address,
            )?
        } else {
            None
        };
        let Some(quote_fill) = quote_fill else {
            return Ok(failed_exact_vault_result(
                vault_config,
                block,
                dependency_hashes,
                tail_after_tx_hash,
                U256::ZERO,
                Some(quote_result.gas_used),
                quote_result.revert_reason.clone(),
                Some("quote simulation did not emit BoughtV2".to_string()),
            ));
        };

        let min_tokens_out =
            min_tokens_out_from_quote(quote_fill.tokens_received, config.slippage_tolerance)?;
        if min_tokens_out.is_zero() {
            return Ok(failed_exact_vault_result(
                vault_config,
                block,
                dependency_hashes,
                tail_after_tx_hash,
                min_tokens_out,
                Some(quote_result.gas_used),
                None,
                Some("derived exact V2 vault minTokensOut is zero".to_string()),
            ));
        }

        let final_tx = build_vault_buy_tx(
            &vault_config,
            config.token_address,
            min_tokens_out,
            u64::MAX,
            base_fee,
        );
        let final_result = projected_chain.step_with_trace(final_tx).await;
        let final_result = match final_result {
            Ok(result) => result,
            Err(err) => {
                return Ok(failed_exact_vault_result(
                    vault_config,
                    block,
                    dependency_hashes,
                    tail_after_tx_hash,
                    min_tokens_out,
                    None,
                    None,
                    Some(format!("final_simulation_error: {err}")),
                ));
            }
        };

        let fill = if final_result.success {
            extract_bought_v2_fill(
                &final_result,
                vault_config.vault_address,
                config.token_address,
            )?
        } else {
            None
        };
        let would_revert = !final_result.success || fill.is_none();
        let (eth_spent_wei, tokens_received_raw) = fill
            .map(|fill| (fill.eth_spent, fill.tokens_received))
            .unwrap_or((U256::ZERO, U256::ZERO));
        let failure_reason = if final_result.success && fill.is_none() {
            Some("final simulation did not emit BoughtV2".to_string())
        } else {
            final_result.revert_reason.clone()
        };

        Ok(ExactVaultBuySimulationResult {
            route: "uniswap_v2_trading_vault".to_string(),
            vault_address: vault_config.vault_address,
            owner_address: vault_config.owner_address,
            chain_id: vault_config.chain_id,
            simulated_block: block,
            dependency_tx_hashes: dependency_hashes,
            tail_after_tx_hash,
            buy_amount_wei: vault_config.buy_amount_wei,
            min_tokens_out,
            expected_tokens_raw: Some(quote_fill.tokens_received),
            tokens_received_raw,
            eth_spent_wei,
            gas_used: Some(final_result.gas_used),
            would_revert,
            revert_reason: failure_reason.clone(),
            metadata: json!({
                "provider": "mempool_processor.reth_exact_calldata_uniswap_v2_trading_vault",
                "event": if would_revert { serde_json::Value::Null } else { json!("BoughtV2") },
                "exact_vault_calldata": true,
                "calldata_builder": "tx_simulator.uniswap_v2_trading_vault.buyV2ExactEthForTokens",
                "quote_gas_used": quote_result.gas_used,
                "quote_tokens_received_raw": quote_fill.tokens_received.to_string(),
                "failure_reason": failure_reason,
                "deadline_policy": "mempool_probe_max_deadline",
                "slippage_tolerance_percent": config.slippage_tolerance,
                "min_tokens_out": min_tokens_out.to_string(),
            }),
        })
    }

    async fn projected_chain_after_replay(
        &self,
        block: u64,
        replay_sequence: &[ProcessedTransaction],
    ) -> Result<UnsignedTxChainSimulation> {
        let mut chain = self
            .mempool_simulator
            .get_tx_simulator()
            .start_simulation_chain(Some(block))
            .await?;
        let base_fee = chain.block_base_fee().unwrap_or(1);

        for prior_tx in replay_sequence {
            let mut unsigned = UnsignedTxBuilder::build_unsigned_from_processed_tx(prior_tx);
            unsigned.nonce = Some(prior_tx.nonce);
            normalize_replay_fee(&mut unsigned, base_fee);
            let simulation = chain.step_with_trace(unsigned).await.wrap_err_with(|| {
                format!(
                    "failed replaying dependency tx={} at block={block}",
                    prior_tx.hash
                )
            })?;
            if !simulation.success {
                return Err(eyre!(
                    "dependency tx={} reverted before exact V2 vault entry simulation: {:?}",
                    prior_tx.hash,
                    simulation.revert_reason
                ));
            }
        }

        Ok(chain)
    }
}

fn exact_vault_entry_config() -> Result<ExactVaultEntryConfig> {
    let vault_address = parse_address_config(
        "ETH_MAINNET_UNISWAP_V2_TRADING_VAULT",
        uniswap_v2_trading_vault_from_config(),
    )?;
    let owner_address = parse_address_config(
        "ETH_MAINNET_UNISWAP_V2_TRADING_VAULT_OWNER",
        uniswap_v2_trading_vault_owner_from_config(),
    )?;
    let buy_amount_wei = alpha_live_entry_buy_wei_from_config()
        .unwrap_or_else(|| DEFAULT_ALPHA_LIVE_ENTRY_BUY_WEI.to_string());
    let buy_amount_wei = U256::from_str_radix(buy_amount_wei.trim(), 10)
        .map_err(|err| eyre!("invalid ALPHA_LIVE_ENTRY_BUY_WEI value {buy_amount_wei:?}: {err}"))?;
    let buy_gas_limit = alpha_live_uniswap_v2_vault_buy_gas_limit_from_config()
        .unwrap_or(DEFAULT_UNISWAP_V2_TRADING_VAULT_BUY_GAS_LIMIT);

    Ok(ExactVaultEntryConfig {
        vault_address,
        owner_address,
        buy_amount_wei,
        buy_gas_limit,
        chain_id: CHAIN_ID_MAINNET,
    })
}

fn parse_address_config(key: &str, value: Option<String>) -> Result<Address> {
    let value = value.ok_or_else(|| eyre!("{key} must be set in config.env"))?;
    value
        .trim()
        .parse::<Address>()
        .map_err(|err| eyre!("invalid {key} address {value:?}: {err}"))
}

fn build_vault_buy_tx(
    config: &ExactVaultEntryConfig,
    token_address: Address,
    min_tokens_out: U256,
    deadline: u64,
    base_fee: u128,
) -> UnsignedTransaction {
    let mut tx = build_uniswap_v2_trading_vault_buy_v2_exact_eth_for_tokens(
        config.vault_address,
        config.owner_address,
        token_address,
        config.buy_amount_wei,
        min_tokens_out,
        deadline,
    );
    tx.gas = Some(config.buy_gas_limit);
    tx.gas_price = None;
    tx.max_fee_per_gas = Some(base_fee);
    tx.max_priority_fee_per_gas = Some(0);
    tx.nonce = None;
    tx
}

fn normalize_replay_fee(tx: &mut UnsignedTransaction, base_fee: u128) {
    if let Some(gas_price) = tx.gas_price {
        if gas_price < base_fee {
            tx.gas_price = Some(base_fee);
        }
        return;
    }

    match tx.max_fee_per_gas {
        Some(max_fee) if max_fee < base_fee => {
            tx.max_fee_per_gas = Some(base_fee);
            if tx
                .max_priority_fee_per_gas
                .map(|priority| priority > base_fee)
                .unwrap_or(false)
            {
                tx.max_priority_fee_per_gas = Some(0);
            }
        }
        Some(_) => {}
        None => {
            tx.max_fee_per_gas = Some(base_fee);
            tx.max_priority_fee_per_gas = Some(0);
        }
    }
}

fn min_tokens_out_from_quote(expected_tokens: U256, slippage_tolerance: f64) -> Result<U256> {
    if !(0.0..100.0).contains(&slippage_tolerance) {
        return Err(eyre!(
            "slippage_tolerance must be in [0, 100), got {slippage_tolerance}"
        ));
    }
    let slippage_bps = (slippage_tolerance * 100.0).round() as u64;
    let keep_bps = SLIPPAGE_BPS_DENOMINATOR
        .checked_sub(slippage_bps)
        .ok_or_else(|| eyre!("slippage bps overflow"))?;
    Ok(expected_tokens * U256::from(keep_bps) / U256::from(SLIPPAGE_BPS_DENOMINATOR))
}

fn failed_exact_vault_result(
    config: ExactVaultEntryConfig,
    simulated_block: u64,
    dependency_tx_hashes: Vec<String>,
    tail_after_tx_hash: Option<String>,
    min_tokens_out: U256,
    gas_used: Option<u64>,
    revert_reason: Option<String>,
    failure_reason: Option<String>,
) -> ExactVaultBuySimulationResult {
    ExactVaultBuySimulationResult {
        route: "uniswap_v2_trading_vault".to_string(),
        vault_address: config.vault_address,
        owner_address: config.owner_address,
        chain_id: config.chain_id,
        simulated_block,
        dependency_tx_hashes,
        tail_after_tx_hash,
        buy_amount_wei: config.buy_amount_wei,
        min_tokens_out,
        expected_tokens_raw: None,
        tokens_received_raw: U256::ZERO,
        eth_spent_wei: U256::ZERO,
        gas_used,
        would_revert: true,
        revert_reason: revert_reason.clone().or_else(|| failure_reason.clone()),
        metadata: json!({
            "provider": "mempool_processor.reth_exact_calldata_uniswap_v2_trading_vault",
            "exact_vault_calldata": true,
            "calldata_builder": "tx_simulator.uniswap_v2_trading_vault.buyV2ExactEthForTokens",
            "event": serde_json::Value::Null,
            "failure_reason": failure_reason.or(revert_reason),
            "deadline_policy": "mempool_probe_max_deadline",
            "min_tokens_out": min_tokens_out.to_string(),
        }),
    }
}

fn extract_bought_v2_fill(
    result: &FullSimulationResult,
    vault_address: Address,
    token_address: Address,
) -> Result<Option<BoughtV2Fill>> {
    let event_topic = event_signature_topic(BOUGHT_V2_SIGNATURE);
    let expected_token_topic = indexed_address_topic(token_address);
    for log in &result.logs {
        if log.address != vault_address {
            continue;
        }
        let topics = log.data.topics();
        if topics.first().copied() != Some(event_topic) {
            continue;
        }
        if topics.get(1).copied() != Some(expected_token_topic) {
            continue;
        }
        let words = decode_event_words(log.data.data.as_ref(), 3)?;
        return Ok(Some(BoughtV2Fill {
            eth_spent: words[0],
            tokens_received: words[1],
        }));
    }
    Ok(None)
}

fn event_signature_topic(signature: &str) -> B256 {
    keccak256(signature.as_bytes())
}

fn indexed_address_topic(address: Address) -> B256 {
    let mut topic = [0u8; 32];
    topic[12..].copy_from_slice(address.as_slice());
    B256::from(topic)
}

fn decode_event_words(data: &[u8], count: usize) -> Result<Vec<U256>> {
    let expected_len = count * 32;
    if data.len() < expected_len {
        return Err(eyre!(
            "event data too short: got {} bytes, expected at least {}",
            data.len(),
            expected_len
        ));
    }
    (0..count)
        .map(|idx| {
            let start = idx * 32;
            let end = start + 32;
            Ok(U256::from_be_slice(&data[start..end]))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::min_tokens_out_from_quote;
    use alloy_primitives::U256;

    #[test]
    fn derives_min_tokens_out_from_slippage_percent() {
        let min = min_tokens_out_from_quote(U256::from(1_000_000u64), 5.0).unwrap();
        assert_eq!(min, U256::from(950_000u64));
    }

    #[test]
    fn rejects_full_slippage_for_exact_vault_entry() {
        let err = min_tokens_out_from_quote(U256::from(1_000_000u64), 100.0)
            .expect_err("full slippage must not produce production calldata");
        assert!(err.to_string().contains("slippage_tolerance"));
    }
}
