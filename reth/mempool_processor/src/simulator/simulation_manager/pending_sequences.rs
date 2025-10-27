use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::B256;
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
const PENDING_SEQUENCE_TTL: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SequenceKey {
    pub creator: String,
    pub token: String,
}

#[derive(Debug, Clone)]
struct PendingSequenceEntry {
    tx: ProcessedTransaction,
    first_seen: Instant,
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
        processed: ProcessedTransaction,
        now: Instant,
    ) -> Vec<ProcessedTransaction> {
        let mut guard = self.inner.lock().await;
        let entry = guard.entry(key).or_insert_with(VecDeque::new);
        Self::prune_sequence(entry, now);

        if let Some(pos) = entry.iter().position(|tx| tx.tx.hash == processed.hash) {
            entry.remove(pos);
        } else if let Some(pos) = entry.iter().position(|tx| tx.tx.nonce == processed.nonce) {
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
        let mined_hashes = match provider.get_block_transactions(block_number).await {
            Ok(block) => block
                .transactions
                .iter()
                .map(|tx| tx.tx_metadata.hash)
                .collect::<HashSet<_>>(),
            Err(err) => {
                let err_msg = err.to_string();
                if err_msg.contains("No header for block") {
                    let set = Self::fetch_block_hashes_via_rpc(block_number).await?;
                    if set.is_empty() {
                        debug!(
                            block_number,
                            "RPC fallback returned no transactions; block not yet persisted"
                        );
                        return Ok(());
                    }
                    set
                } else {
                    return Err(err);
                }
            }
        };

        if mined_hashes.is_empty() {
            return Ok(());
        }

        let mut guard = self.inner.lock().await;
        guard.retain(|_, deque| {
            deque.retain(|entry| !mined_hashes.contains(&entry.tx.hash));
            !deque.is_empty()
        });

        Ok(())
    }

    fn prune_sequence(deque: &mut VecDeque<PendingSequenceEntry>, now: Instant) {
        deque.retain(|entry| now.duration_since(entry.first_seen) <= PENDING_SEQUENCE_TTL);
    }

    async fn fetch_block_hashes_via_rpc(block_number: u64) -> EyreResult<HashSet<B256>> {
        lazy_static! {
            static ref RPC_CLIENT: Client = Client::new();
            static ref RPC_URL: String = std::env::var("ETH_RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8545".to_string());
        }

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBlockByNumber",
            "params": [format!("0x{:x}", block_number), false]
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
            return Ok(HashSet::new());
        };

        if result.is_null() {
            return Ok(HashSet::new());
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

        let mut hashes = HashSet::with_capacity(transactions.len());
        for tx in transactions {
            let hash_str = tx
                .as_str()
                .ok_or_else(|| eyre::eyre!("transaction hash was not a string"))?;
            let hash_trimmed = hash_str.trim_start_matches("0x");
            let bytes = hex::decode(hash_trimmed).map_err(|err| {
                eyre::eyre!(
                    "invalid transaction hash {} for block {}: {}",
                    hash_str,
                    block_number,
                    err
                )
            })?;
            let hash = B256::from_slice(&bytes);
            hashes.insert(hash);
        }

        Ok(hashes)
    }
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
