use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::{Address, B256};
use eyre::Result as EyreResult;
use lazy_static::lazy_static;
use reqwest::Client;
use reth_chain_query::provider::RethQueryProvider;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::debug;

use crate::tx_router::TransactionCategory;
use reth_chain_query::to_checksum_address;
use tx_processor::ProcessedTransaction;

/// Maximum number of helper transactions we keep per (creator, token) sequence.
const PENDING_SEQUENCE_LIMIT: usize = 6;
/// Time-to-live for helpers that remain unmined.
///
/// This is a mempool-only bridge for same-window helper txs. If the sequence is
/// still not usable after a couple of seconds, confirmed block processing should
/// provide the later context instead of keeping stale pending state alive.
const PENDING_SEQUENCE_TTL: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SequenceKey {
    pub creator: String,
    pub token: String,
}

#[derive(Debug, Clone)]
struct PendingSequenceEntry {
    tx: ProcessedTransaction,
    source_hash: B256,
    first_seen: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MinedSenderNonce {
    sender: Address,
    nonce: u64,
}

#[derive(Debug, Default)]
struct MinedTransactionKeys {
    hashes: HashSet<B256>,
    sender_nonces: HashSet<MinedSenderNonce>,
}

/// Thread-safe pending sequence store keyed by (creator, token).
#[derive(Clone, Default)]
pub struct PendingSequences {
    inner: Arc<Mutex<HashMap<SequenceKey, VecDeque<PendingSequenceEntry>>>>,
}

impl PendingSequences {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn record_transaction(
        &self,
        key: SequenceKey,
        source_hash: B256,
        processed: ProcessedTransaction,
        now: Instant,
    ) -> Vec<ProcessedTransaction> {
        let mut guard = self.inner.lock().await;
        let entry = guard.entry(key).or_insert_with(VecDeque::new);
        Self::prune_sequence(entry, now);

        if let Some(pos) = entry
            .iter()
            .position(|tx| tx.source_hash == source_hash || tx.tx.hash == processed.hash)
        {
            entry.remove(pos);
        } else if let Some(pos) = entry.iter().position(|tx| {
            tx.tx.from_address == processed.from_address && tx.tx.nonce == processed.nonce
        }) {
            entry.remove(pos);
        }

        let insert_pos = entry
            .iter()
            .position(|tx| tx.tx.nonce > processed.nonce)
            .unwrap_or(entry.len());
        entry.insert(
            insert_pos,
            PendingSequenceEntry {
                tx: processed,
                source_hash,
                first_seen: now,
            },
        );

        while entry.len() > PENDING_SEQUENCE_LIMIT {
            entry.pop_front();
        }

        entry.iter().map(|item| item.tx.clone()).collect()
    }

    pub async fn prune_expired(&self, now: Instant) {
        let mut guard = self.inner.lock().await;
        guard.retain(|_, deque| {
            Self::prune_sequence(deque, now);
            !deque.is_empty()
        });
    }

    pub async fn prune_block(
        &self,
        provider: Arc<RethQueryProvider>,
        block_number: u64,
    ) -> EyreResult<()> {
        let mined_keys = match provider.get_block_transactions(block_number).await {
            Ok(block) => {
                let mut keys = MinedTransactionKeys::default();
                for tx in &block.transactions {
                    keys.hashes.insert(tx.tx_metadata.hash);
                    keys.sender_nonces.insert(MinedSenderNonce {
                        sender: tx.tx_metadata.from,
                        nonce: tx.tx_metadata.nonce,
                    });
                }
                keys
            }
            Err(err) => {
                let err_msg = err.to_string();
                if err_msg.contains("No header for block") {
                    let keys = Self::fetch_block_keys_via_rpc(block_number).await?;
                    if keys.is_empty() {
                        debug!(
                            block_number,
                            "RPC fallback returned no transactions; block not yet persisted"
                        );
                        return Ok(());
                    }
                    keys
                } else {
                    return Err(err);
                }
            }
        };

        if mined_keys.is_empty() {
            return Ok(());
        }

        self.prune_mined_keys(&mined_keys).await;

        Ok(())
    }

    async fn prune_mined_keys(&self, mined_keys: &MinedTransactionKeys) {
        let mut guard = self.inner.lock().await;
        guard.retain(|_, deque| {
            deque.retain(|entry| !entry.matches_mined_keys(mined_keys));
            !deque.is_empty()
        });
    }

    fn prune_sequence(deque: &mut VecDeque<PendingSequenceEntry>, now: Instant) {
        deque.retain(|entry| now.duration_since(entry.first_seen) <= PENDING_SEQUENCE_TTL);
    }

    async fn fetch_block_keys_via_rpc(block_number: u64) -> EyreResult<MinedTransactionKeys> {
        lazy_static! {
            static ref RPC_CLIENT: Client = Client::new();
            static ref RPC_URL: String = crate::config::eth_rpc_url_from_env();
        }

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBlockByNumber",
            "params": [format!("0x{:x}", block_number), true]
        });

        let response = RPC_CLIENT.post(&*RPC_URL).json(&payload).send().await?;
        let value: Value = response.json().await?;

        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown RPC error");
            return Err(eyre::eyre!(
                "RPC error fetching block {}: {}",
                block_number,
                message
            ));
        }

        let Some(result) = value.get("result") else {
            return Ok(MinedTransactionKeys::default());
        };

        if result.is_null() {
            return Ok(MinedTransactionKeys::default());
        }

        let transactions = result
            .get("transactions")
            .and_then(|txs| txs.as_array())
            .ok_or_else(|| {
                eyre::eyre!(
                    "RPC response for block {} missing transactions array",
                    block_number
                )
            })?;

        let mut keys = MinedTransactionKeys::default();
        for tx in transactions {
            let hash = parse_b256_field(tx, "hash", block_number)?;
            let sender = parse_address_field(tx, "from", block_number)?;
            let nonce = parse_u64_hex_field(tx, "nonce", block_number)?;
            keys.hashes.insert(hash);
            keys.sender_nonces
                .insert(MinedSenderNonce { sender, nonce });
        }

        Ok(keys)
    }
}

impl PendingSequenceEntry {
    fn matches_mined_keys(&self, mined_keys: &MinedTransactionKeys) -> bool {
        mined_keys.hashes.contains(&self.source_hash)
            || mined_keys.hashes.contains(&self.tx.hash)
            || mined_keys.sender_nonces.contains(&MinedSenderNonce {
                sender: self.tx.from_address,
                nonce: self.tx.nonce,
            })
    }
}

impl MinedTransactionKeys {
    fn is_empty(&self) -> bool {
        self.hashes.is_empty() && self.sender_nonces.is_empty()
    }
}

fn parse_b256_field(tx: &Value, field: &str, block_number: u64) -> EyreResult<B256> {
    let value = tx
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre::eyre!("RPC tx in block {} missing {}", block_number, field))?;
    let trimmed = value.trim_start_matches("0x");
    let bytes = hex::decode(trimmed).map_err(|err| {
        eyre::eyre!(
            "invalid {} {} for block {}: {}",
            field,
            value,
            block_number,
            err
        )
    })?;
    if bytes.len() != 32 {
        return Err(eyre::eyre!(
            "invalid {} length {} for block {}",
            field,
            bytes.len(),
            block_number
        ));
    }
    Ok(B256::from_slice(&bytes))
}

fn parse_address_field(tx: &Value, field: &str, block_number: u64) -> EyreResult<Address> {
    let value = tx
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre::eyre!("RPC tx in block {} missing {}", block_number, field))?;
    value
        .trim_start_matches("0x")
        .parse::<Address>()
        .map_err(|err| {
            eyre::eyre!(
                "invalid {} {} for block {}: {}",
                field,
                value,
                block_number,
                err
            )
        })
}

fn parse_u64_hex_field(tx: &Value, field: &str, block_number: u64) -> EyreResult<u64> {
    let value = tx
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre::eyre!("RPC tx in block {} missing {}", block_number, field))?;
    u64::from_str_radix(value.trim_start_matches("0x"), 16).map_err(|err| {
        eyre::eyre!(
            "invalid {} {} for block {}: {}",
            field,
            value,
            block_number,
            err
        )
    })
}

pub fn sequence_key_from_request(
    request: &crate::simulator::simulation_manager::TxSimulationJob,
    processed: Option<&ProcessedTransaction>,
) -> Option<SequenceKey> {
    match &request.category {
        TransactionCategory::CreatorTransaction {
            creator,
            target_token,
            target_address,
            ..
        } => {
            let token = target_token
                .clone()
                .filter(|addr| !addr.is_empty() && addr != "none")
                .or_else(|| Some(target_address.clone()))
                .filter(|addr| !addr.is_empty() && addr != "none")?;
            Some(SequenceKey {
                creator: creator.clone(),
                token,
            })
        }
        TransactionCategory::ContractCreation { deployer, .. } => {
            let processed = processed?;
            let contract = processed.contract_address.or_else(|| {
                processed
                    .contract_creation_events
                    .first()
                    .map(|evt| evt.contract_address)
            })?;
            Some(SequenceKey {
                creator: deployer.clone(),
                token: to_checksum_address(&contract),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use alloy_primitives::{Address, B256, U256};
    use tx_processor::ProcessedTransaction;

    use super::{MinedSenderNonce, MinedTransactionKeys, PendingSequences, SequenceKey};

    #[tokio::test]
    async fn prune_mined_keys_matches_original_mempool_hash() {
        let pending = PendingSequences::new();
        let key = test_key();
        let source_hash = B256::repeat_byte(0x11);
        let processed = test_tx(B256::repeat_byte(0xaa), Address::repeat_byte(0x01), 7);

        pending
            .record_transaction(key.clone(), source_hash, processed, Instant::now())
            .await;

        let mut mined = MinedTransactionKeys::default();
        mined.hashes.insert(source_hash);
        pending.prune_mined_keys(&mined).await;

        assert!(pending.transactions_for_key(&key).await.is_empty());
    }

    #[tokio::test]
    async fn prune_mined_keys_matches_replaced_sender_nonce() {
        let pending = PendingSequences::new();
        let key = test_key();
        let sender = Address::repeat_byte(0x02);
        let processed = test_tx(B256::repeat_byte(0xaa), sender, 9);

        pending
            .record_transaction(
                key.clone(),
                B256::repeat_byte(0x11),
                processed,
                Instant::now(),
            )
            .await;

        let mut mined = MinedTransactionKeys::default();
        mined
            .sender_nonces
            .insert(MinedSenderNonce { sender, nonce: 9 });
        pending.prune_mined_keys(&mined).await;

        assert!(pending.transactions_for_key(&key).await.is_empty());
    }

    #[tokio::test]
    async fn replacement_keeps_one_pending_entry_per_sender_nonce() {
        let pending = PendingSequences::new();
        let key = test_key();
        let sender = Address::repeat_byte(0x03);
        let first = test_tx(B256::repeat_byte(0xaa), sender, 12);
        let second = test_tx(B256::repeat_byte(0xbb), sender, 12);

        pending
            .record_transaction(key.clone(), B256::repeat_byte(0x11), first, Instant::now())
            .await;
        pending
            .record_transaction(key.clone(), B256::repeat_byte(0x22), second, Instant::now())
            .await;

        let entries = pending.transactions_for_key(&key).await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hash, B256::repeat_byte(0xbb));
    }

    impl PendingSequences {
        async fn transactions_for_key(&self, key: &SequenceKey) -> Vec<ProcessedTransaction> {
            self.inner
                .lock()
                .await
                .get(key)
                .map(|entries| entries.iter().map(|entry| entry.tx.clone()).collect())
                .unwrap_or_default()
        }
    }

    fn test_key() -> SequenceKey {
        SequenceKey {
            creator: "0xcreator".to_string(),
            token: "0xtoken".to_string(),
        }
    }

    fn test_tx(hash: B256, sender: Address, nonce: u64) -> ProcessedTransaction {
        ProcessedTransaction::new(
            hash,
            0,
            0,
            0,
            sender,
            Some(Address::repeat_byte(0x10)),
            U256::ZERO,
            true,
            nonce,
            2,
            Vec::new(),
        )
    }
}
