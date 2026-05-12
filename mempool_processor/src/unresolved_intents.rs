use crate::mempool_fetcher::MempoolTransaction;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnresolvedIntentKind {
    LpApproval,
    LiquidityRemoval,
    CreatorControl,
    V4ModifyLiquidity,
}

impl UnresolvedIntentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LpApproval => "lp_approval",
            Self::LiquidityRemoval => "liquidity_removal",
            Self::CreatorControl => "creator_control",
            Self::V4ModifyLiquidity => "v4_modify_liquidity",
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnresolvedIntent {
    pub tx: MempoolTransaction,
    pub kind: UnresolvedIntentKind,
    pub first_seen: Instant,
    pub last_seen: Instant,
    pub last_attempt: Option<Instant>,
    pub attempts: u32,
    pub reason: String,
    pub in_flight: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UnresolvedIntentStats {
    pub pending: usize,
    pub in_flight: usize,
    pub recorded_total: u64,
    pub resolved_total: u64,
    pub expired_total: u64,
    pub dropped_total: u64,
    pub cache_wait_avg_ms: f64,
}

#[derive(Default)]
struct StoreInner {
    entries: HashMap<String, UnresolvedIntent>,
    recorded_total: u64,
    resolved_total: u64,
    expired_total: u64,
    dropped_total: u64,
    resolved_cache_wait_total: Duration,
}

#[derive(Clone)]
pub struct UnresolvedIntentStore {
    inner: Arc<Mutex<StoreInner>>,
    max_entries: usize,
    ttl: Duration,
    retry_interval: Duration,
    log_path: Arc<PathBuf>,
}

impl UnresolvedIntentStore {
    pub fn new<P: AsRef<Path>>(
        max_entries: usize,
        ttl: Duration,
        retry_interval: Duration,
        log_path: P,
    ) -> Self {
        let log_path = Arc::new(log_path.as_ref().to_path_buf());
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path.as_ref())
        {
            writeln!(file, "# Unresolved Mempool Intents").ok();
            writeln!(file, "# Internal cache-wait lane; not user-facing signals").ok();
            writeln!(file, "# ===============================================").ok();
        }

        Self {
            inner: Arc::new(Mutex::new(StoreInner::default())),
            max_entries: max_entries.max(1),
            ttl,
            retry_interval,
            log_path,
        }
    }

    pub async fn record(
        &self,
        tx: MempoolTransaction,
        kind: UnresolvedIntentKind,
        reason: impl Into<String>,
    ) -> bool {
        let now = Instant::now();
        let reason = reason.into();
        let tx_hash = tx.hash.clone();
        let mut dropped: Option<UnresolvedIntent> = None;
        let inserted = {
            let mut inner = self.inner.lock().await;
            if let Some(existing) = inner.entries.get_mut(&tx_hash) {
                existing.last_seen = now;
                existing.reason = reason.clone();
                existing.in_flight = false;
                false
            } else {
                if inner.entries.len() >= self.max_entries {
                    if let Some(oldest_hash) = inner
                        .entries
                        .iter()
                        .min_by_key(|(_, entry)| entry.first_seen)
                        .map(|(hash, _)| hash.clone())
                    {
                        dropped = inner.entries.remove(&oldest_hash);
                        inner.dropped_total += 1;
                    }
                }

                inner.entries.insert(
                    tx_hash.clone(),
                    UnresolvedIntent {
                        tx,
                        kind,
                        first_seen: now,
                        last_seen: now,
                        last_attempt: None,
                        attempts: 0,
                        reason: reason.clone(),
                        in_flight: false,
                    },
                );
                inner.recorded_total += 1;
                true
            }
        };

        if let Some(dropped) = dropped {
            self.log_line(
                "DROPPED",
                &dropped.tx.hash,
                dropped.kind,
                "bounded store evicted oldest unresolved intent",
                0,
            );
        }
        if inserted {
            self.log_line("RECORDED", &tx_hash, kind, &reason, 0);
        }
        inserted
    }

    pub async fn take_ready_for_retry(&self) -> Vec<UnresolvedIntent> {
        let now = Instant::now();
        let mut expired = Vec::new();
        let mut ready = Vec::new();
        {
            let mut inner = self.inner.lock().await;
            let expired_hashes: Vec<String> = inner
                .entries
                .iter()
                .filter(|(_, entry)| now.duration_since(entry.first_seen) > self.ttl)
                .map(|(hash, _)| hash.clone())
                .collect();
            for hash in expired_hashes {
                if let Some(entry) = inner.entries.remove(&hash) {
                    inner.expired_total += 1;
                    expired.push(entry);
                }
            }

            for entry in inner.entries.values_mut() {
                if entry.in_flight {
                    continue;
                }
                if let Some(last_attempt) = entry.last_attempt {
                    if now.duration_since(last_attempt) < self.retry_interval {
                        continue;
                    }
                }
                entry.last_attempt = Some(now);
                entry.attempts = entry.attempts.saturating_add(1);
                entry.in_flight = true;
                ready.push(entry.clone());
            }
        }

        for entry in expired {
            self.log_line(
                "EXPIRED",
                &entry.tx.hash,
                entry.kind,
                &entry.reason,
                entry.attempts,
            );
        }

        ready
    }

    pub async fn mark_pending(&self, tx_hash: &str, reason: impl Into<String>) {
        let reason = reason.into();
        let mut log_data = None;
        {
            let mut inner = self.inner.lock().await;
            if let Some(entry) = inner.entries.get_mut(tx_hash) {
                entry.reason = reason.clone();
                entry.in_flight = false;
                log_data = Some((entry.kind, entry.attempts));
            }
        }
        if let Some((kind, attempts)) = log_data {
            self.log_line("WAITING", tx_hash, kind, &reason, attempts);
        }
    }

    pub async fn resolve(&self, tx_hash: &str) -> bool {
        let resolved = {
            let mut inner = self.inner.lock().await;
            inner.entries.remove(tx_hash).map(|entry| {
                inner.resolved_total += 1;
                inner.resolved_cache_wait_total += Instant::now().duration_since(entry.first_seen);
                entry
            })
        };

        if let Some(entry) = resolved {
            self.log_line(
                "RESOLVED",
                tx_hash,
                entry.kind,
                &entry.reason,
                entry.attempts,
            );
            true
        } else {
            false
        }
    }

    pub async fn stats(&self) -> UnresolvedIntentStats {
        let inner = self.inner.lock().await;
        let in_flight = inner
            .entries
            .values()
            .filter(|entry| entry.in_flight)
            .count();
        let cache_wait_avg_ms = if inner.resolved_total == 0 {
            0.0
        } else {
            inner.resolved_cache_wait_total.as_secs_f64() * 1000.0 / inner.resolved_total as f64
        };
        UnresolvedIntentStats {
            pending: inner.entries.len(),
            in_flight,
            recorded_total: inner.recorded_total,
            resolved_total: inner.resolved_total,
            expired_total: inner.expired_total,
            dropped_total: inner.dropped_total,
            cache_wait_avg_ms,
        }
    }

    fn log_line(
        &self,
        action: &str,
        tx_hash: &str,
        kind: UnresolvedIntentKind,
        reason: &str,
        attempts: u32,
    ) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path.as_ref())
        {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            writeln!(
                file,
                "[{}] {} | kind={} tx={} attempts={} reason={}",
                timestamp,
                action,
                kind.as_str(),
                tx_hash,
                attempts,
                reason
            )
            .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;
    use serde_json::json;
    use std::time::Instant;

    #[tokio::test]
    async fn records_and_resolves_intent() {
        let path = std::env::temp_dir().join(format!(
            "unresolved_intents_test_{}.log",
            std::process::id()
        ));
        let store = UnresolvedIntentStore::new(8, Duration::from_secs(600), Duration::ZERO, &path);

        let tx = test_tx("0xtx");
        assert!(
            store
                .record(tx.clone(), UnresolvedIntentKind::LpApproval, "pool missing")
                .await
        );
        assert!(
            !store
                .record(tx, UnresolvedIntentKind::LpApproval, "pool still missing")
                .await
        );

        let ready = store.take_ready_for_retry().await;
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].kind, UnresolvedIntentKind::LpApproval);
        assert!(store.resolve("0xtx").await);

        let stats = store.stats().await;
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.recorded_total, 1);
        assert_eq!(stats.resolved_total, 1);
    }

    fn test_tx(hash: &str) -> MempoolTransaction {
        MempoolTransaction {
            hash: hash.to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: vec![0u8; 20],
            to: Some(vec![1u8; 20]),
            input: vec![0u8; 4],
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: Vec::new(),
            function_category: None,
        }
    }
}
