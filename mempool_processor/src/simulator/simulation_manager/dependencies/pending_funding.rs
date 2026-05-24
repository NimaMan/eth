use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use alloy_primitives::{Address, U256};
use tokio::sync::Mutex;

use crate::mempool_fetcher::MempoolTransaction;

const PENDING_FUNDING_DEPENDENCY_TTL: Duration = Duration::from_secs(120);
const MAX_FUNDING_TXS_PER_RECIPIENT: usize = 32;
const MAX_REPLAY_FUNDING_TXS: usize = 8;

#[derive(Clone, Debug)]
pub(in crate::simulator::simulation_manager) struct PendingFundingDependencyLookup {
    pub transactions: Vec<MempoolTransaction>,
    pub total_value: U256,
    pub required_value: Option<U256>,
}

#[derive(Clone, Debug, Default)]
pub struct PendingFundingDependencyStats {
    pub recipients: usize,
    pub transactions: usize,
    pub recorded: u64,
    pub replaced: u64,
    pub lookups: u64,
    pub hits: u64,
    pub gaps: u64,
    pub expired: u64,
}

#[derive(Clone, Default)]
pub(in crate::simulator::simulation_manager) struct PendingFundingDependencies {
    inner: Arc<Mutex<HashMap<Address, VecDeque<PendingFundingDependency>>>>,
    counters: Arc<PendingFundingDependencyCounters>,
}

#[derive(Default)]
struct PendingFundingDependencyCounters {
    recorded: AtomicU64,
    replaced: AtomicU64,
    lookups: AtomicU64,
    hits: AtomicU64,
    gaps: AtomicU64,
    expired: AtomicU64,
}

#[derive(Clone)]
struct PendingFundingDependency {
    tx: MempoolTransaction,
    value: U256,
    first_seen: Instant,
}

impl PendingFundingDependencies {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn record(&self, tx: MempoolTransaction, now: Instant) {
        if tx.value == U256::ZERO {
            return;
        }
        let Some(recipient) = tx
            .to
            .as_ref()
            .filter(|to| to.len() == 20)
            .map(|to| Address::from_slice(to))
        else {
            return;
        };

        let mut guard = self.inner.lock().await;
        let recipient_txs = guard.entry(recipient).or_insert_with(VecDeque::new);
        prune_expired_recipient_entries(
            recipient_txs,
            now,
            &self.counters.expired,
            PENDING_FUNDING_DEPENDENCY_TTL,
        );

        let tx_hash = tx.hash.clone();
        if let Some(existing) = recipient_txs
            .iter_mut()
            .find(|entry| entry.tx.hash == tx_hash)
        {
            existing.tx = tx;
            existing.value = existing.tx.value;
            existing.first_seen = now;
            self.counters.replaced.fetch_add(1, Ordering::Relaxed);
            return;
        }

        recipient_txs.push_back(PendingFundingDependency {
            value: tx.value,
            tx,
            first_seen: now,
        });
        self.counters.recorded.fetch_add(1, Ordering::Relaxed);

        while recipient_txs.len() > MAX_FUNDING_TXS_PER_RECIPIENT {
            recipient_txs.pop_front();
            self.counters.expired.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn funding_for(
        &self,
        recipient: Address,
        required_value: Option<U256>,
        now: Instant,
    ) -> PendingFundingDependencyLookup {
        self.counters.lookups.fetch_add(1, Ordering::Relaxed);

        let mut guard = self.inner.lock().await;
        let Some(recipient_txs) = guard.get_mut(&recipient) else {
            self.counters.gaps.fetch_add(1, Ordering::Relaxed);
            return PendingFundingDependencyLookup {
                transactions: Vec::new(),
                total_value: U256::ZERO,
                required_value,
            };
        };

        prune_expired_recipient_entries(
            recipient_txs,
            now,
            &self.counters.expired,
            PENDING_FUNDING_DEPENDENCY_TTL,
        );

        let mut selected = Vec::new();
        let mut total_value = U256::ZERO;
        for entry in recipient_txs.iter().take(MAX_REPLAY_FUNDING_TXS) {
            selected.push(entry.tx.clone());
            total_value = total_value.saturating_add(entry.value);
            if required_value
                .map(|required| total_value >= required)
                .unwrap_or(false)
            {
                break;
            }
        }

        if selected.is_empty()
            || required_value
                .map(|required| total_value < required)
                .unwrap_or(false)
        {
            self.counters.gaps.fetch_add(1, Ordering::Relaxed);
        } else {
            self.counters.hits.fetch_add(1, Ordering::Relaxed);
        }

        PendingFundingDependencyLookup {
            transactions: selected,
            total_value,
            required_value,
        }
    }

    pub async fn prune_expired(&self, now: Instant) {
        let mut guard = self.inner.lock().await;
        guard.retain(|_, recipient_txs| {
            prune_expired_recipient_entries(
                recipient_txs,
                now,
                &self.counters.expired,
                PENDING_FUNDING_DEPENDENCY_TTL,
            );
            !recipient_txs.is_empty()
        });
    }

    pub async fn stats(&self) -> PendingFundingDependencyStats {
        let guard = self.inner.lock().await;
        PendingFundingDependencyStats {
            recipients: guard.len(),
            transactions: guard.values().map(VecDeque::len).sum(),
            recorded: self.counters.recorded.load(Ordering::Relaxed),
            replaced: self.counters.replaced.load(Ordering::Relaxed),
            lookups: self.counters.lookups.load(Ordering::Relaxed),
            hits: self.counters.hits.load(Ordering::Relaxed),
            gaps: self.counters.gaps.load(Ordering::Relaxed),
            expired: self.counters.expired.load(Ordering::Relaxed),
        }
    }
}

fn prune_expired_recipient_entries(
    recipient_txs: &mut VecDeque<PendingFundingDependency>,
    now: Instant,
    expired_counter: &AtomicU64,
    ttl: Duration,
) {
    let before = recipient_txs.len();
    recipient_txs.retain(|entry| now.duration_since(entry.first_seen) <= ttl);
    let expired = before.saturating_sub(recipient_txs.len()) as u64;
    if expired > 0 {
        expired_counter.fetch_add(expired, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn returns_visible_inbound_funding_for_recipient() {
        let store = PendingFundingDependencies::new();
        let now = Instant::now();
        let recipient = Address::repeat_byte(0x22);

        store
            .record(test_tx(Address::repeat_byte(0x11), recipient, 10), now)
            .await;

        let lookup = store
            .funding_for(recipient, Some(U256::from(7u64)), now)
            .await;
        assert_eq!(lookup.transactions.len(), 1);
        assert_eq!(lookup.total_value, U256::from(10u64));
        assert_eq!(lookup.required_value, Some(U256::from(7u64)));

        let stats = store.stats().await;
        assert_eq!(stats.recorded, 1);
        assert_eq!(stats.hits, 1);
    }

    #[tokio::test]
    async fn reports_gap_when_visible_funding_is_insufficient() {
        let store = PendingFundingDependencies::new();
        let now = Instant::now();
        let recipient = Address::repeat_byte(0x44);

        store
            .record(test_tx(Address::repeat_byte(0x33), recipient, 3), now)
            .await;

        let lookup = store
            .funding_for(recipient, Some(U256::from(7u64)), now)
            .await;
        assert_eq!(lookup.transactions.len(), 1);
        assert_eq!(lookup.total_value, U256::from(3u64));

        let stats = store.stats().await;
        assert_eq!(stats.gaps, 1);
    }

    fn test_tx(from: Address, to: Address, value: u64) -> MempoolTransaction {
        MempoolTransaction {
            hash: format!("0x{:064x}", value),
            data: json!({
                "from": format!("{from:#x}"),
                "to": format!("{to:#x}"),
                "value": format!("0x{:x}", value),
                "nonce": "0x0"
            }),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: from.as_slice().to_vec(),
            to: Some(to.as_slice().to_vec()),
            input: Vec::new(),
            value: U256::from(value),
            gas_price: Some(U256::ZERO),
            functions: Vec::new(),
            function_category: None,
        }
    }
}
