use std::time::Instant;

use alloy_primitives::{Address, U256};
use eyre::{eyre, Result as EyreResult, WrapErr};
use reqwest::Client;
use serde_json::{json, Value};
use tx_processor::tx_processor::TxProcessor;
use tx_processor::ProcessedTransaction;
use tx_simulator::UnsignedTransaction;

use crate::tx_router::TransactionCategory;

use super::{
    mempool_tx_to_unsigned_tx,
    pending_nonce_dependencies::{sender_nonce, source_hash},
    SimulationManager, TxSimulationJob,
};

const MAX_MINED_DEPENDENCY_LOOKAHEAD_BLOCKS: u64 = 16;

pub(super) struct ProcessedWithNonceDependencies {
    pub transaction: ProcessedTransaction,
    pub dependencies: Vec<ProcessedTransaction>,
}

impl SimulationManager {
    pub(super) async fn build_processed_transaction_with_nonce_dependencies(
        &self,
        request: &TxSimulationJob,
        retry_on_missing_header: bool,
    ) -> EyreResult<ProcessedWithNonceDependencies> {
        let unsigned_tx = mempool_tx_to_unsigned_tx(&request.tx)?;
        let block_number = self.resolve_live_simulation_block().await;

        let direct_result = self
            .liquidity_removal_simulator
            .process_with_optional_retry(unsigned_tx.clone(), block_number, retry_on_missing_header)
            .await;

        let direct_error = match direct_result {
            Ok(transaction) => {
                return Ok(ProcessedWithNonceDependencies {
                    transaction,
                    dependencies: Vec::new(),
                });
            }
            Err(err) => err,
        };

        if should_replay_funding_dependencies(&direct_error, &request.category) {
            return self
                .replay_funding_dependencies_and_current(
                    request,
                    unsigned_tx,
                    &direct_error,
                    block_number,
                )
                .await;
        }

        if !is_nonce_too_high_error(&direct_error) {
            return Err(direct_error);
        }

        let sender = unsigned_tx
            .from
            .ok_or_else(|| eyre!("pending_nonce_dependency_gap: transaction has no sender"))?;
        let target_nonce = unsigned_tx.nonce.ok_or_else(|| {
            eyre!("pending_nonce_dependency_gap: transaction has no explicit nonce")
        })?;
        let expected_nonce = parse_expected_nonce(&direct_error).ok_or_else(|| {
            eyre!(
                "pending_nonce_dependency_gap: failed to parse expected nonce from {}",
                direct_error
            )
        })?;

        if target_nonce.saturating_sub(expected_nonce) as usize > 64 {
            return Err(eyre!(
                "pending_nonce_dependency_gap: sender {sender:#x} target_nonce {target_nonce} expected_nonce {expected_nonce} is too wide; original_error={direct_error}"
            ));
        }

        let lookup = self
            .pending_nonce_dependencies
            .dependencies_for(sender, expected_nonce, target_nonce, Instant::now())
            .await;

        let dependency_txs = if lookup.missing_nonce.is_none() {
            lookup.transactions
        } else {
            match fetch_recent_mined_nonce_dependencies(
                sender,
                expected_nonce,
                target_nonce,
                block_number,
            )
            .await
            {
                Ok(mined_txs) => merge_dependency_sources(
                    lookup.transactions,
                    mined_txs,
                    expected_nonce,
                    target_nonce,
                )?,
                Err(err) => {
                    let missing_nonce = lookup.missing_nonce.unwrap_or(expected_nonce);
                    return Err(eyre!(
                        "pending_nonce_dependency_gap: sender {sender:#x} target_nonce {target_nonce} expected_nonce {expected_nonce} missing_nonce {missing_nonce}; mined_backfill_error={err}; original_error={direct_error}"
                    ));
                }
            }
        };

        if let Some(missing_nonce) =
            first_missing_dependency_nonce(&dependency_txs, expected_nonce, target_nonce)
        {
            return Err(eyre!(
                "pending_nonce_dependency_gap: sender {sender:#x} target_nonce {target_nonce} expected_nonce {expected_nonce} missing_nonce {missing_nonce}; original_error={direct_error}"
            ));
        }

        self.replay_nonce_dependencies_and_current(
            request,
            unsigned_tx,
            sender,
            expected_nonce,
            target_nonce,
            block_number,
            dependency_txs,
        )
        .await
    }

    async fn resolve_live_simulation_block(&self) -> Option<u64> {
        match self.mempool_simulator.latest_simulation_block().await {
            Ok(number) => Some(number),
            Err(err) => {
                tracing::error!(
                    "Failed to resolve simulation block for nonce dependency replay: {}",
                    err
                );
                None
            }
        }
    }

    async fn replay_nonce_dependencies_and_current(
        &self,
        request: &TxSimulationJob,
        current_unsigned: UnsignedTransaction,
        sender: Address,
        expected_nonce: u64,
        target_nonce: u64,
        block_number: Option<u64>,
        dependency_txs: Vec<crate::mempool_fetcher::MempoolTransaction>,
    ) -> EyreResult<ProcessedWithNonceDependencies> {
        let block = block_number.ok_or_else(|| {
            eyre!("pending_nonce_dependency_gap: simulation block is unavailable")
        })?;
        let mut chain = self
            .mempool_simulator
            .get_tx_simulator()
            .start_simulation_chain(Some(block))
            .await
            .wrap_err_with(|| {
                format!(
                    "pending_nonce_dependency_replay_failed: failed to start chain at block {block}"
                )
            })?;
        let processor = TxProcessor::new();
        let mut dependencies = Vec::with_capacity(dependency_txs.len());

        for (idx, dependency_tx) in dependency_txs.iter().enumerate() {
            let (dependency_sender, dependency_nonce) =
                sender_nonce(dependency_tx).ok_or_else(|| {
                    eyre!(
                        "pending_nonce_dependency_replay_failed: dependency {} has no sender/nonce",
                        dependency_tx.hash
                    )
                })?;
            if dependency_sender != sender {
                return Err(eyre!(
                    "pending_nonce_dependency_replay_failed: dependency {} sender {dependency_sender:#x} does not match {sender:#x}",
                    dependency_tx.hash
                ));
            }
            let expected_dependency_nonce = expected_nonce + idx as u64;
            if dependency_nonce != expected_dependency_nonce {
                return Err(eyre!(
                    "pending_nonce_dependency_replay_failed: dependency {} nonce {} does not match expected {}",
                    dependency_tx.hash,
                    dependency_nonce,
                    expected_dependency_nonce
                ));
            }

            let dependency_unsigned = mempool_tx_to_unsigned_tx(dependency_tx)?;
            let simulation = chain
                .step_with_trace(dependency_unsigned.clone())
                .await
                .wrap_err_with(|| {
                    format!(
                        "pending_nonce_dependency_replay_failed: failed replaying dependency tx={} nonce={} block={}",
                        dependency_tx.hash, dependency_nonce, block
                    )
                })?;
            let processed = processor
                .process_transaction_from_simulation_result(
                    &dependency_unsigned,
                    &simulation,
                    block,
                    idx as u64,
                )
                .await
                .wrap_err_with(|| {
                    format!(
                        "pending_nonce_dependency_replay_failed: failed processing dependency tx={} nonce={} block={}",
                        dependency_tx.hash, dependency_nonce, block
                    )
                })?;

            if !simulation.success {
                return Err(eyre!(
                    "pending_nonce_dependency_replay_failed: dependency tx={} nonce={} reverted before target tx={} target_nonce={} revert={:?}",
                    dependency_tx.hash,
                    dependency_nonce,
                    request.tx.hash,
                    target_nonce,
                    simulation.revert_reason
                ));
            }

            dependencies.push(processed);
        }

        let current_simulation = chain
            .step_with_trace(current_unsigned.clone())
            .await
            .wrap_err_with(|| {
                format!(
                    "pending_nonce_dependency_replay_failed: failed replaying target tx={} sender={sender:#x} expected_nonce={} target_nonce={} block={}",
                    request.tx.hash, expected_nonce, target_nonce, block
                )
            })?;
        let current = processor
            .process_transaction_from_simulation_result(
                &current_unsigned,
                &current_simulation,
                block,
                dependency_txs.len() as u64,
            )
            .await
            .wrap_err_with(|| {
                format!(
                    "pending_nonce_dependency_replay_failed: failed processing target tx={} block={}",
                    request.tx.hash, block
                )
            })?;

        tracing::info!(
            tx_hash = %request.tx.hash,
            source_hash = ?source_hash(&request.tx),
            sender = %format!("{sender:#x}"),
            expected_nonce,
            target_nonce,
            dependency_count = dependencies.len(),
            block,
            "replayed pending nonce dependencies before target transaction"
        );

        Ok(ProcessedWithNonceDependencies {
            transaction: current,
            dependencies,
        })
    }

    async fn replay_funding_dependencies_and_current(
        &self,
        request: &TxSimulationJob,
        current_unsigned: UnsignedTransaction,
        direct_error: &eyre::Report,
        block_number: Option<u64>,
    ) -> EyreResult<ProcessedWithNonceDependencies> {
        let sender = current_unsigned
            .from
            .ok_or_else(|| eyre!("funding_dependency_gap: transaction has no sender"))?;
        let required_value = parse_lack_of_funds_required_value(direct_error);
        let lookup = self
            .pending_funding_dependencies
            .funding_for(sender, required_value, Instant::now())
            .await;

        if lookup.transactions.is_empty()
            || lookup
                .required_value
                .map(|required| lookup.total_value < required)
                .unwrap_or(false)
        {
            return Err(eyre!(
                "funding_dependency_wait: sender {sender:#x} required_value={} visible_funding={} funding_txs={} original_error={direct_error}",
                lookup
                    .required_value
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                lookup.total_value,
                lookup.transactions.len()
            ));
        }

        let block = block_number
            .ok_or_else(|| eyre!("funding_dependency_gap: simulation block is unavailable"))?;
        let mut chain = self
            .mempool_simulator
            .get_tx_simulator()
            .start_simulation_chain(Some(block))
            .await
            .wrap_err_with(|| {
                format!(
                    "funding_dependency_gap: failed to start funding replay chain at block {block}"
                )
            })?;
        let processor = TxProcessor::new();
        let mut dependencies = Vec::with_capacity(lookup.transactions.len());

        for (idx, funding_tx) in lookup.transactions.iter().enumerate() {
            let funding_unsigned = mempool_tx_to_unsigned_tx(funding_tx)?;
            let simulation = chain
                .step_with_trace(funding_unsigned.clone())
                .await
                .wrap_err_with(|| {
                    format!(
                        "funding_dependency_gap: failed replaying funding tx={} for target={} block={}",
                        funding_tx.hash, request.tx.hash, block
                    )
                })?;

            if !simulation.success {
                return Err(eyre!(
                    "funding_dependency_gap: funding tx={} reverted before target={} revert={:?}",
                    funding_tx.hash,
                    request.tx.hash,
                    simulation.revert_reason
                ));
            }

            if let Ok(processed) = processor
                .process_transaction_from_simulation_result(
                    &funding_unsigned,
                    &simulation,
                    block,
                    idx as u64,
                )
                .await
            {
                dependencies.push(processed);
            }
        }

        let current_simulation = chain
            .step_with_trace(current_unsigned.clone())
            .await
            .wrap_err_with(|| {
                format!(
                    "funding_dependency_gap: failed replaying target tx={} after {} visible funding txs at block={}",
                    request.tx.hash,
                    lookup.transactions.len(),
                    block
                )
            })?;
        let current = processor
            .process_transaction_from_simulation_result(
                &current_unsigned,
                &current_simulation,
                block,
                lookup.transactions.len() as u64,
            )
            .await
            .wrap_err_with(|| {
                format!(
                    "funding_dependency_gap: failed processing target tx={} after funding replay at block={}",
                    request.tx.hash, block
                )
            })?;

        tracing::info!(
            tx_hash = %request.tx.hash,
            sender = %format!("{sender:#x}"),
            block,
            funding_tx_count = lookup.transactions.len(),
            visible_funding_value = %lookup.total_value,
            required_value = %lookup
                .required_value
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            "replayed visible inbound funding before target transaction"
        );

        Ok(ProcessedWithNonceDependencies {
            transaction: current,
            dependencies,
        })
    }
}

fn is_nonce_too_high_error(error: &eyre::Report) -> bool {
    let message = error.to_string();
    message.contains("transaction validation error: nonce")
        && message.contains("too high")
        && message.contains("expected")
}

fn is_lack_of_funds_error(error: &eyre::Report) -> bool {
    let message = error.to_string();
    message.contains("transaction validation error: lack of funds")
        || message.contains("insufficient funds")
}

fn should_replay_funding_dependencies(
    error: &eyre::Report,
    category: &TransactionCategory,
) -> bool {
    is_lack_of_funds_error(error)
        && matches!(
            category,
            TransactionCategory::ContractCreation { .. }
                | TransactionCategory::CreatorTransaction { .. }
        )
}

fn parse_lack_of_funds_required_value(error: &eyre::Report) -> Option<U256> {
    let message = error.to_string();
    let required = message.split("for max fee (").nth(1)?;
    let digits = required
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    U256::from_str_radix(&digits, 10).ok()
}

fn parse_expected_nonce(error: &eyre::Report) -> Option<u64> {
    let message = error.to_string();
    let expected = message.split("expected ").nth(1)?;
    let digits = expected
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn merge_dependency_sources(
    pending_txs: Vec<crate::mempool_fetcher::MempoolTransaction>,
    mined_txs: Vec<crate::mempool_fetcher::MempoolTransaction>,
    expected_nonce: u64,
    target_nonce: u64,
) -> EyreResult<Vec<crate::mempool_fetcher::MempoolTransaction>> {
    let mut by_nonce = std::collections::BTreeMap::new();
    for tx in pending_txs.into_iter().chain(mined_txs) {
        let Some((_sender, nonce)) = sender_nonce(&tx) else {
            continue;
        };
        if (expected_nonce..target_nonce).contains(&nonce) {
            by_nonce.entry(nonce).or_insert(tx);
        }
    }

    let mut merged = Vec::new();
    for nonce in expected_nonce..target_nonce {
        let Some(tx) = by_nonce.remove(&nonce) else {
            return Err(eyre!(
                "pending_nonce_dependency_gap: missing dependency nonce {nonce}"
            ));
        };
        merged.push(tx);
    }
    Ok(merged)
}

fn first_missing_dependency_nonce(
    txs: &[crate::mempool_fetcher::MempoolTransaction],
    expected_nonce: u64,
    target_nonce: u64,
) -> Option<u64> {
    let available = txs
        .iter()
        .filter_map(sender_nonce)
        .map(|(_, nonce)| nonce)
        .collect::<std::collections::HashSet<_>>();
    (expected_nonce..target_nonce).find(|nonce| !available.contains(nonce))
}

async fn fetch_recent_mined_nonce_dependencies(
    sender: Address,
    expected_nonce: u64,
    target_nonce: u64,
    base_block: Option<u64>,
) -> EyreResult<Vec<crate::mempool_fetcher::MempoolTransaction>> {
    let Some(base_block) = base_block else {
        return Err(eyre!("simulation base block unavailable"));
    };
    let latest_block = rpc_block_number().await?;
    if latest_block <= base_block {
        return Err(eyre!(
            "latest RPC block {latest_block} is not ahead of simulation base {base_block}"
        ));
    }

    let end_block = latest_block.min(base_block + MAX_MINED_DEPENDENCY_LOOKAHEAD_BLOCKS);
    let mut by_nonce = std::collections::BTreeMap::new();
    for block in (base_block + 1)..=end_block {
        for tx in rpc_block_transactions(block).await? {
            let Some((tx_sender, nonce)) = sender_nonce(&tx) else {
                continue;
            };
            if tx_sender == sender && (expected_nonce..target_nonce).contains(&nonce) {
                by_nonce.insert(nonce, tx);
            }
        }
    }

    if by_nonce.is_empty() {
        return Err(eyre!(
            "no mined dependencies found for sender {sender:#x} nonces {}..{} in blocks {}..{}",
            expected_nonce,
            target_nonce.saturating_sub(1),
            base_block + 1,
            end_block
        ));
    }

    tracing::info!(
        sender = %format!("{sender:#x}"),
        expected_nonce,
        target_nonce,
        base_block,
        end_block,
        dependency_count = by_nonce.len(),
        "backfilled mined nonce dependencies from recent canonical blocks"
    );

    Ok(by_nonce.into_values().collect())
}

async fn rpc_block_number() -> EyreResult<u64> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_blockNumber",
        "params": []
    });
    let response = rpc_client().post(rpc_url()).json(&payload).send().await?;
    let value: Value = response.json().await?;
    if let Some(error) = value.get("error") {
        return Err(eyre!("RPC eth_blockNumber error: {error}"));
    }
    let number = value
        .get("result")
        .and_then(Value::as_str)
        .ok_or_else(|| eyre!("RPC eth_blockNumber missing result"))?;
    parse_hex_u64(number).ok_or_else(|| eyre!("invalid RPC block number {number}"))
}

async fn rpc_block_transactions(
    block_number: u64,
) -> EyreResult<Vec<crate::mempool_fetcher::MempoolTransaction>> {
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getBlockByNumber",
        "params": [format!("0x{block_number:x}"), true]
    });
    let response = rpc_client().post(rpc_url()).json(&payload).send().await?;
    let value: Value = response.json().await?;
    if let Some(error) = value.get("error") {
        return Err(eyre!(
            "RPC eth_getBlockByNumber error for {block_number}: {error}"
        ));
    }
    let Some(transactions) = value
        .get("result")
        .and_then(|result| result.get("transactions"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };

    Ok(transactions
        .iter()
        .filter_map(rpc_tx_to_mempool_tx)
        .collect())
}

fn rpc_tx_to_mempool_tx(tx: &Value) -> Option<crate::mempool_fetcher::MempoolTransaction> {
    let hash = tx.get("hash")?.as_str()?.to_string();
    let from = parse_address_bytes(tx.get("from")?.as_str()?)?;
    let to = tx
        .get("to")
        .and_then(Value::as_str)
        .and_then(parse_address_bytes);
    let input = tx
        .get("input")
        .and_then(Value::as_str)
        .and_then(parse_hex_bytes)
        .unwrap_or_default();
    let value = tx
        .get("value")
        .and_then(Value::as_str)
        .and_then(parse_u256_hex)
        .unwrap_or(U256::ZERO);
    let gas_price = tx
        .get("gasPrice")
        .and_then(Value::as_str)
        .and_then(parse_u256_hex);

    Some(crate::mempool_fetcher::MempoolTransaction {
        hash,
        data: tx.clone(),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from,
        to,
        input,
        value,
        gas_price,
        functions: Vec::new(),
        function_category: None,
    })
}

fn rpc_client() -> &'static Client {
    static CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(Client::new)
}

fn rpc_url() -> &'static str {
    static URL: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    URL.get_or_init(crate::config::eth_rpc_url_from_env)
        .as_str()
}

fn parse_address_bytes(value: &str) -> Option<Vec<u8>> {
    let address = value.parse::<Address>().ok()?;
    Some(address.as_slice().to_vec())
}

fn parse_hex_bytes(value: &str) -> Option<Vec<u8>> {
    hex::decode(value.trim_start_matches("0x")).ok()
}

fn parse_u256_hex(value: &str) -> Option<U256> {
    U256::from_str_radix(value.trim_start_matches("0x"), 16).ok()
}

fn parse_hex_u64(value: &str) -> Option<u64> {
    u64::from_str_radix(value.trim_start_matches("0x"), 16).ok()
}

pub fn is_pending_nonce_dependency_error(error: &str) -> bool {
    error.contains("pending_nonce_dependency_gap")
        || error.contains("pending_nonce_dependency_replay_failed")
        || error.contains("missing dependency nonce")
        || (error.contains("transaction validation error: nonce")
            && error.contains("too high")
            && error.contains("expected"))
}

pub fn is_funding_dependency_error(error: &str) -> bool {
    error.contains("funding_dependency_wait") || error.contains("funding_dependency_gap")
}

pub(super) fn merge_nonce_dependencies_with_replay_sequence(
    mut dependencies: Vec<ProcessedTransaction>,
    replay_sequence: Vec<ProcessedTransaction>,
) -> Vec<ProcessedTransaction> {
    for tx in replay_sequence {
        let already_present = dependencies.iter().any(|existing| {
            existing.hash == tx.hash
                || (existing.from_address == tx.from_address && existing.nonce == tx.nonce)
        });
        if !already_present {
            dependencies.push(tx);
        }
    }
    dependencies
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function_detector::CreatorFunctionType;

    #[test]
    fn funding_dependency_replay_includes_creator_transactions() {
        let error = eyre::eyre!(
            "transaction validation error: lack of funds (187231779595947) for max fee (344019312000000)"
        );
        let category = TransactionCategory::CreatorTransaction {
            creator: "0x62e5d2ca425d637a5bb737e78c3bb0aa2f9d448d".to_string(),
            target_address: "0xc36442b4a4522e871399cd717abdd847ab11fe88".to_string(),
            target_token: None,
            function_type: CreatorFunctionType::LiquidityRemoval,
        };

        assert!(should_replay_funding_dependencies(&error, &category));
        assert_eq!(
            parse_lack_of_funds_required_value(&error),
            Some(U256::from(344_019_312_000_000u64))
        );
    }

    #[test]
    fn funding_dependency_replay_ignores_regular_transactions() {
        let error = eyre::eyre!("transaction validation error: lack of funds (1) for max fee (2)");
        let category = TransactionCategory::Regular {
            is_transfer: true,
            is_approval: false,
        };

        assert!(!should_replay_funding_dependencies(&error, &category));
    }

    #[test]
    fn missing_dependency_nonce_is_pending_dependency_noise() {
        assert!(is_pending_nonce_dependency_error(
            "Failed to process creator transaction: missing dependency nonce 4"
        ));
        assert!(is_pending_nonce_dependency_error(
            "Failed to process creator transaction: pending_nonce_dependency_gap: missing dependency nonce 4"
        ));
    }
}
