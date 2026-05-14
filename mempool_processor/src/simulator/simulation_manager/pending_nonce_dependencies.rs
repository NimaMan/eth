use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy_primitives::{Address, B256};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::mempool_fetcher::MempoolTransaction;

const PENDING_NONCE_DEPENDENCY_TTL: Duration = Duration::from_secs(120);
const MAX_NONCES_PER_SENDER: usize = 64;

#[derive(Clone, Debug)]
pub(super) struct PendingNonceDependencyLookup {
    pub transactions: Vec<MempoolTransaction>,
    pub missing_nonce: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct PendingNonceDependencyStats {
    pub senders: usize,
    pub transactions: usize,
    pub recorded: u64,
    pub replaced: u64,
    pub lookups: u64,
    pub hits: u64,
    pub gaps: u64,
    pub expired: u64,
}

#[derive(Clone, Default)]
pub(super) struct PendingNonceDependencies {
    inner: Arc<Mutex<HashMap<Address, BTreeMap<u64, PendingNonceDependency>>>>,
    counters: Arc<PendingNonceDependencyCounters>,
}

#[derive(Default)]
struct PendingNonceDependencyCounters {
    recorded: AtomicU64,
    replaced: AtomicU64,
    lookups: AtomicU64,
    hits: AtomicU64,
    gaps: AtomicU64,
    expired: AtomicU64,
}

#[derive(Clone)]
struct PendingNonceDependency {
    tx: MempoolTransaction,
    first_seen: Instant,
}

impl PendingNonceDependencies {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn record(&self, tx: MempoolTransaction, now: Instant) {
        let Some((sender, nonce)) = sender_nonce(&tx) else {
            return;
        };

        let mut guard = self.inner.lock().await;
        let sender_txs = guard.entry(sender).or_insert_with(BTreeMap::new);
        prune_expired_sender_entries(
            sender_txs,
            now,
            &self.counters.expired,
            PENDING_NONCE_DEPENDENCY_TTL,
        );

        if sender_txs
            .insert(
                nonce,
                PendingNonceDependency {
                    tx,
                    first_seen: now,
                },
            )
            .is_some()
        {
            self.counters.replaced.fetch_add(1, Ordering::Relaxed);
        } else {
            self.counters.recorded.fetch_add(1, Ordering::Relaxed);
        }

        while sender_txs.len() > MAX_NONCES_PER_SENDER {
            sender_txs.pop_first();
            self.counters.expired.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn dependencies_for(
        &self,
        sender: Address,
        expected_nonce: u64,
        target_nonce: u64,
        now: Instant,
    ) -> PendingNonceDependencyLookup {
        self.counters.lookups.fetch_add(1, Ordering::Relaxed);

        if target_nonce <= expected_nonce {
            self.counters.hits.fetch_add(1, Ordering::Relaxed);
            return PendingNonceDependencyLookup {
                transactions: Vec::new(),
                missing_nonce: None,
            };
        }

        let mut guard = self.inner.lock().await;
        let Some(sender_txs) = guard.get_mut(&sender) else {
            self.counters.gaps.fetch_add(1, Ordering::Relaxed);
            return PendingNonceDependencyLookup {
                transactions: Vec::new(),
                missing_nonce: Some(expected_nonce),
            };
        };

        prune_expired_sender_entries(
            sender_txs,
            now,
            &self.counters.expired,
            PENDING_NONCE_DEPENDENCY_TTL,
        );

        let mut transactions = Vec::new();
        for nonce in expected_nonce..target_nonce {
            let Some(entry) = sender_txs.get(&nonce) else {
                self.counters.gaps.fetch_add(1, Ordering::Relaxed);
                return PendingNonceDependencyLookup {
                    transactions,
                    missing_nonce: Some(nonce),
                };
            };
            transactions.push(entry.tx.clone());
        }

        self.counters.hits.fetch_add(1, Ordering::Relaxed);
        PendingNonceDependencyLookup {
            transactions,
            missing_nonce: None,
        }
    }

    pub async fn prune_expired(&self, now: Instant) {
        let mut guard = self.inner.lock().await;
        guard.retain(|_, sender_txs| {
            prune_expired_sender_entries(
                sender_txs,
                now,
                &self.counters.expired,
                PENDING_NONCE_DEPENDENCY_TTL,
            );
            !sender_txs.is_empty()
        });
    }

    pub async fn stats(&self) -> PendingNonceDependencyStats {
        let guard = self.inner.lock().await;
        PendingNonceDependencyStats {
            senders: guard.len(),
            transactions: guard.values().map(BTreeMap::len).sum(),
            recorded: self.counters.recorded.load(Ordering::Relaxed),
            replaced: self.counters.replaced.load(Ordering::Relaxed),
            lookups: self.counters.lookups.load(Ordering::Relaxed),
            hits: self.counters.hits.load(Ordering::Relaxed),
            gaps: self.counters.gaps.load(Ordering::Relaxed),
            expired: self.counters.expired.load(Ordering::Relaxed),
        }
    }
}

fn prune_expired_sender_entries(
    sender_txs: &mut BTreeMap<u64, PendingNonceDependency>,
    now: Instant,
    expired_counter: &AtomicU64,
    ttl: Duration,
) {
    let before = sender_txs.len();
    sender_txs.retain(|_, entry| now.duration_since(entry.first_seen) <= ttl);
    let expired = before.saturating_sub(sender_txs.len()) as u64;
    if expired > 0 {
        expired_counter.fetch_add(expired, Ordering::Relaxed);
    }
}

pub(super) fn sender_nonce(tx: &MempoolTransaction) -> Option<(Address, u64)> {
    if tx.from.len() != 20 {
        return None;
    }

    let nonce = parse_u64_hex(tx.data.get("nonce")?)?;
    Some((Address::from_slice(&tx.from), nonce))
}

pub(super) fn source_hash(tx: &MempoolTransaction) -> Option<B256> {
    tx.hash.parse::<B256>().ok()
}

fn parse_u64_hex(value: &Value) -> Option<u64> {
    let text = value.as_str()?;
    u64::from_str_radix(text.trim_start_matches("0x"), 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function_detector::CreatorFunctionType;
    use alloy_primitives::U256;
    use serde_json::json;

    #[tokio::test]
    async fn returns_contiguous_sender_nonce_dependencies() {
        let store = PendingNonceDependencies::new();
        let now = Instant::now();
        let sender = Address::repeat_byte(0x11);

        store.record(test_tx(sender, 7, "0x01"), now).await;
        store.record(test_tx(sender, 8, "0x02"), now).await;

        let lookup = store.dependencies_for(sender, 7, 9, now).await;
        assert_eq!(lookup.missing_nonce, None);
        assert_eq!(lookup.transactions.len(), 2);
    }

    #[tokio::test]
    async fn reports_first_missing_sender_nonce() {
        let store = PendingNonceDependencies::new();
        let now = Instant::now();
        let sender = Address::repeat_byte(0x22);

        store.record(test_tx(sender, 9, "0x09"), now).await;

        let lookup = store.dependencies_for(sender, 8, 10, now).await;
        assert_eq!(lookup.missing_nonce, Some(8));
        assert!(lookup.transactions.is_empty());
    }

    #[tokio::test]
    async fn expires_old_sender_nonce_dependencies() {
        let store = PendingNonceDependencies::new();
        let first_seen = Instant::now();
        let sender = Address::repeat_byte(0x33);

        store.record(test_tx(sender, 1, "0x01"), first_seen).await;

        let later = first_seen + PENDING_NONCE_DEPENDENCY_TTL + Duration::from_millis(1);
        let lookup = store.dependencies_for(sender, 1, 2, later).await;
        assert_eq!(lookup.missing_nonce, Some(1));
    }

    #[tokio::test]
    async fn replacement_keeps_latest_tx_for_sender_nonce() {
        let store = PendingNonceDependencies::new();
        let now = Instant::now();
        let sender = Address::repeat_byte(0x44);

        store.record(test_tx(sender, 3, "0x01"), now).await;
        store.record(test_tx(sender, 3, "0x02"), now).await;

        let lookup = store.dependencies_for(sender, 3, 4, now).await;
        assert_eq!(lookup.transactions.len(), 1);
        assert_eq!(lookup.transactions[0].hash, "0x02");
    }

    fn test_tx(sender: Address, nonce: u64, hash: &str) -> MempoolTransaction {
        MempoolTransaction {
            hash: hash.to_string(),
            data: json!({
                "from": format!("{sender:#x}"),
                "nonce": format!("0x{nonce:x}"),
                "input": "0x",
                "value": "0x0",
                "gas": "0x5208",
                "gasPrice": "0x1"
            }),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: sender.as_slice().to_vec(),
            to: None,
            input: Vec::new(),
            value: U256::ZERO,
            gas_price: Some(U256::from(1)),
            functions: Vec::new(),
            function_category: Some(CreatorFunctionType::Other("test".to_string())),
        }
    }
}
