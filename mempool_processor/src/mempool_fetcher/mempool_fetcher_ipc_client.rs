use crate::liquidity_approval_call::{
    decode_liquidity_approval_call, is_liquidity_approval_selector,
};
use crate::position_approval_call::decode_position_approval_call;
use crate::token_tracking::TokenTrackingCache;
use crate::tx_router::protocol::{
    is_known_position_manager_candidate, is_protocol_liquidity_removal_candidate,
    is_v4_modify_liquidity_candidate, liquidity_removal_token_candidates,
};
use alloy_primitives::U256;
use chrono::Utc;
use eyre::{eyre, Result};
use hex;
use serde_json::{json, Value};
use std::os::unix::net::UnixStream as StdUnixStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UnixStream as TokioUnixStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, error, info, warn};

use super::MempoolTransaction;

pub struct MempoolFetcherIPCClient {
    socket_path: String,
    tx_sender: mpsc::Sender<MempoolTransaction>,
    tx_receiver: Arc<Mutex<mpsc::Receiver<MempoolTransaction>>>,
    critical_tx_sender: mpsc::Sender<MempoolTransaction>,
    critical_tx_receiver: Arc<Mutex<mpsc::Receiver<MempoolTransaction>>>,
    stats: Arc<RwLock<Stats>>,
    queue_size: Arc<AtomicUsize>,
    critical_queue_size: Arc<AtomicUsize>,
    observer: Option<Arc<dyn MempoolIngressObserver>>,
    signal_filter_cache: Option<Arc<TokenTrackingCache>>,
}

#[derive(Default, Clone)]
pub struct Stats {
    pub total: u64,
    pub total_dropped: u64,
    pub total_filtered_irrelevant: u64,
    pub critical_received: u64,
    pub critical_dropped: u64,
    pub sub_1ms: u64,
    pub sub_100us: u64,
    pub sub_10us: u64,
    pub queue_size: usize,
    pub critical_queue_size: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct IngressStatsSnapshot {
    pub queue_depth: usize,
    pub total_received: u64,
    pub total_dropped: u64,
}

pub trait MempoolIngressObserver: Send + Sync + 'static {
    fn tx_received(&self, hash: &str, first_seen_ms: u64, stats: IngressStatsSnapshot);

    fn tx_dropped_queue_full(&self, hash: &str, first_seen_ms: u64, stats: IngressStatsSnapshot);

    fn queue_depth_updated(&self, stats: IngressStatsSnapshot);
}

impl MempoolFetcherIPCClient {
    pub fn new(socket_path: Option<&str>) -> Result<Self> {
        Self::new_with_observer(socket_path, None)
    }

    pub fn new_with_observer(
        socket_path: Option<&str>,
        observer: Option<Arc<dyn MempoolIngressObserver>>,
    ) -> Result<Self> {
        Self::new_with_observer_and_signal_filter(socket_path, observer, None)
    }

    pub fn new_with_observer_and_signal_filter(
        socket_path: Option<&str>,
        observer: Option<Arc<dyn MempoolIngressObserver>>,
        signal_filter_cache: Option<Arc<TokenTrackingCache>>,
    ) -> Result<Self> {
        let socket_path = socket_path
            .map(str::to_string)
            .unwrap_or_else(crate::config::reth_ipc_path_from_env);
        let (tx_sender, tx_receiver) = mpsc::channel(50000);
        let (critical_tx_sender, critical_tx_receiver) = mpsc::channel(10000);

        Ok(Self {
            socket_path,
            tx_sender,
            tx_receiver: Arc::new(Mutex::new(tx_receiver)),
            critical_tx_sender,
            critical_tx_receiver: Arc::new(Mutex::new(critical_tx_receiver)),
            stats: Arc::new(RwLock::new(Stats::default())),
            queue_size: Arc::new(AtomicUsize::new(0)),
            critical_queue_size: Arc::new(AtomicUsize::new(0)),
            observer,
            signal_filter_cache,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("IPC start: attempting connect to {}", self.socket_path);
        // Manual retry with std sockets to avoid any timer driver issues
        let deadline = Instant::now() + Duration::from_secs(5);
        let std_stream = loop {
            match StdUnixStream::connect(&self.socket_path) {
                Ok(stream) => break stream,
                Err(err) => {
                    if Instant::now() >= deadline {
                        return Err(eyre!("Failed to connect to IPC socket within 5s: {}", err));
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        };
        std_stream
            .set_nonblocking(true)
            .map_err(|e| eyre!("Failed to set IPC socket nonblocking: {}", e))?;
        let stream = TokioUnixStream::from_std(std_stream)
            .map_err(|e| eyre!("Failed to convert IPC socket to async: {}", e))?;
        info!("IPC start: connected to {}", self.socket_path);

        // Subscribe with correct parameters
        let subscribe = json!({
            "jsonrpc": "2.0",
            "method": "eth_subscribe",
            "params": ["newPendingTransactions", true],
            "id": 1
        });

        let socket = stream.into_std()?;
        socket.set_nonblocking(true)?;

        // Send subscription
        use std::io::Write;
        let mut socket = socket;
        socket.write_all(format!("{}\n", subscribe).as_bytes())?;
        socket.flush()?;
        info!("IPC start: subscription sent");

        // Quick check for subscription response
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut buf = vec![0u8; 4096];
        match socket.read(&mut buf) {
            Ok(n) => {
                let response = std::str::from_utf8(&buf[..n])?;
                if !response.contains("result") {
                    return Err(eyre!("Subscription failed: {}", response));
                }
                info!("Subscription active");
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                info!("IPC start: no subscription response yet (will continue)");
            }
            Err(e) => return Err(e.into()),
        }

        // Convert back to async
        let stream = TokioUnixStream::from_std(socket)?;

        // Start monitoring with ultra-fast detection
        let tx_sender = self.tx_sender.clone();
        let critical_tx_sender = self.critical_tx_sender.clone();
        let stats = self.stats.clone();
        let queue_size = self.queue_size.clone();
        let critical_queue_size = self.critical_queue_size.clone();
        let observer = self.observer.clone();
        let signal_filter_cache = self.signal_filter_cache.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::monitor_nonblocking(
                stream,
                tx_sender,
                critical_tx_sender,
                stats,
                queue_size,
                critical_queue_size,
                observer,
                signal_filter_cache,
            )
            .await
            {
                error!("Monitor error: {}", e);
            }
        });

        Ok(())
    }

    async fn monitor_nonblocking(
        stream: TokioUnixStream,
        tx_sender: mpsc::Sender<MempoolTransaction>,
        critical_tx_sender: mpsc::Sender<MempoolTransaction>,
        stats: Arc<RwLock<Stats>>,
        queue_size: Arc<AtomicUsize>,
        critical_queue_size: Arc<AtomicUsize>,
        observer: Option<Arc<dyn MempoolIngressObserver>>,
        signal_filter_cache: Option<Arc<TokenTrackingCache>>,
    ) -> Result<()> {
        let mut buffer = vec![0u8; 65536]; // 64KB
        let mut pending = Vec::with_capacity(1024 * 1024); // 1MB
        const MAX_PENDING_SIZE: usize = 10 * 1024 * 1024; // 10MB limit

        loop {
            // Try to read data with minimal blocking
            let detect_start = Instant::now();

            match stream.try_read(&mut buffer) {
                Ok(n) => {
                    if n == 0 {
                        break; // Connection closed
                    }

                    // ULTRA-FAST DETECTION!
                    let detection_ns = detect_start.elapsed().as_nanos() as u64;

                    // Append data with bounds checking
                    if pending.len() + n > MAX_PENDING_SIZE {
                        warn!("Pending buffer too large ({} bytes), clearing to prevent memory exhaustion", pending.len());
                        pending.clear();
                        continue;
                    }
                    pending.extend_from_slice(&buffer[..n]);

                    // Process complete JSON lines
                    while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
                        let line = pending.drain(..=pos).collect::<Vec<u8>>();

                        // Quick parse
                        if let Ok(json_str) = std::str::from_utf8(&line) {
                            if let Ok(notification) = serde_json::from_str::<Value>(json_str) {
                                if let Some(params) = notification.get("params") {
                                    if let Some(result) = params.get("result") {
                                        if result.is_object() {
                                            let hash = result
                                                .get("hash")
                                                .and_then(|h| h.as_str())
                                                .unwrap_or("unknown")
                                                .to_string();

                                            // Update stats
                                            {
                                                let mut stats = stats.write().await;
                                                stats.total += 1;
                                                if detection_ns < 1_000_000 {
                                                    stats.sub_1ms += 1;
                                                }
                                                if detection_ns < 100_000 {
                                                    stats.sub_100us += 1;
                                                }
                                                if detection_ns < 10_000 {
                                                    stats.sub_10us += 1;
                                                }
                                            }

                                            let first_seen_ms =
                                                Utc::now().timestamp_millis() as u64;
                                            let (total_received, total_dropped) = {
                                                let stats = stats.read().await;
                                                (stats.total, stats.total_dropped)
                                            };
                                            let ingress_stats = IngressStatsSnapshot {
                                                queue_depth: total_queue_depth(
                                                    &queue_size,
                                                    &critical_queue_size,
                                                ),
                                                total_received,
                                                total_dropped,
                                            };
                                            if let Some(observer) = observer.as_ref() {
                                                observer.tx_received(
                                                    &hash,
                                                    first_seen_ms,
                                                    ingress_stats,
                                                );
                                            }

                                            // Pre-parse transaction fields
                                            let from = result
                                                .get("from")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| {
                                                    hex::decode(s.trim_start_matches("0x")).ok()
                                                })
                                                .unwrap_or_default();

                                            let to = result
                                                .get("to")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| {
                                                    hex::decode(s.trim_start_matches("0x")).ok()
                                                });

                                            let input = result
                                                .get("input")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| {
                                                    hex::decode(s.trim_start_matches("0x")).ok()
                                                })
                                                .unwrap_or_default();

                                            let value = result
                                                .get("value")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| {
                                                    U256::from_str_radix(
                                                        s.trim_start_matches("0x"),
                                                        16,
                                                    )
                                                    .ok()
                                                })
                                                .unwrap_or_default();

                                            let gas_price = result
                                                .get("gasPrice")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| {
                                                    U256::from_str_radix(
                                                        s.trim_start_matches("0x"),
                                                        16,
                                                    )
                                                    .ok()
                                                });

                                            let detection_time = Instant::now();
                                            let tx = MempoolTransaction {
                                                hash: hash.clone(),
                                                data: result.clone(),
                                                detection_ns,
                                                detection_time,
                                                latency_ns: detection_ns, // Same value as detection_ns for compatibility
                                                from,
                                                to,
                                                input,
                                                value,
                                                gas_price,
                                                functions: Vec::new(), // Will be populated by function detector
                                                function_category: None, // Will be populated by function detector
                                            };

                                            let preserve_kind = critical_lane_candidate(
                                                &tx,
                                                signal_filter_cache.as_ref(),
                                            )
                                            .await;
                                            if preserve_kind.is_none()
                                                && !should_enqueue_for_signal_path(
                                                    &tx,
                                                    signal_filter_cache.as_ref(),
                                                )
                                                .await
                                            {
                                                let mut stats = stats.write().await;
                                                stats.total_filtered_irrelevant += 1;
                                                continue;
                                            }

                                            if let Some(preserve_kind) = preserve_kind {
                                                route_critical_tx(
                                                    tx,
                                                    preserve_kind,
                                                    CriticalRouteReason::FastLane,
                                                    &critical_tx_sender,
                                                    &stats,
                                                    &queue_size,
                                                    &critical_queue_size,
                                                    observer.as_ref(),
                                                    first_seen_ms,
                                                )
                                                .await;
                                                continue;
                                            }

                                            match tx_sender.try_send(tx) {
                                                Ok(_) => {
                                                    let depth = queue_size
                                                        .fetch_add(1, Ordering::Relaxed)
                                                        + 1;
                                                    let mut stats = stats.write().await;
                                                    stats.queue_size = depth
                                                        + critical_queue_size
                                                            .load(Ordering::Relaxed);
                                                    let snapshot = IngressStatsSnapshot {
                                                        queue_depth: stats.queue_size,
                                                        total_received: stats.total,
                                                        total_dropped: stats.total_dropped,
                                                    };
                                                    drop(stats);
                                                    if let Some(observer) = observer.as_ref() {
                                                        observer.queue_depth_updated(snapshot);
                                                    }
                                                }
                                                Err(mpsc::error::TrySendError::Full(tx)) => {
                                                    let preserve_kind = critical_lane_candidate(
                                                        &tx,
                                                        signal_filter_cache.as_ref(),
                                                    )
                                                    .await;
                                                    if let Some(preserve_kind) = preserve_kind {
                                                        route_critical_tx(
                                                            tx,
                                                            preserve_kind,
                                                            CriticalRouteReason::Backpressure,
                                                            &critical_tx_sender,
                                                            &stats,
                                                            &queue_size,
                                                            &critical_queue_size,
                                                            observer.as_ref(),
                                                            first_seen_ms,
                                                        )
                                                        .await;
                                                        continue;
                                                    }

                                                    let hash = tx.hash.clone();
                                                    let mut stats = stats.write().await;
                                                    stats.total_dropped += 1;
                                                    stats.queue_size = total_queue_depth(
                                                        &queue_size,
                                                        &critical_queue_size,
                                                    );
                                                    let snapshot = IngressStatsSnapshot {
                                                        queue_depth: stats.queue_size,
                                                        total_received: stats.total,
                                                        total_dropped: stats.total_dropped,
                                                    };
                                                    drop(stats);
                                                    if let Some(observer) = observer.as_ref() {
                                                        observer.tx_dropped_queue_full(
                                                            &hash,
                                                            first_seen_ms,
                                                            snapshot,
                                                        );
                                                    }
                                                    warn!(
                                                        "IPC ingress queue full, dropping transaction hash={} depth={} received={} dropped={} error=no available capacity",
                                                        hash,
                                                        snapshot.queue_depth,
                                                        snapshot.total_received,
                                                        snapshot.total_dropped
                                                    );
                                                }
                                                Err(mpsc::error::TrySendError::Closed(tx)) => {
                                                    let hash = tx.hash.clone();
                                                    let mut stats = stats.write().await;
                                                    stats.total_dropped += 1;
                                                    stats.queue_size = total_queue_depth(
                                                        &queue_size,
                                                        &critical_queue_size,
                                                    );
                                                    let snapshot = IngressStatsSnapshot {
                                                        queue_depth: stats.queue_size,
                                                        total_received: stats.total,
                                                        total_dropped: stats.total_dropped,
                                                    };
                                                    drop(stats);
                                                    if let Some(observer) = observer.as_ref() {
                                                        observer.tx_dropped_queue_full(
                                                            &hash,
                                                            first_seen_ms,
                                                            snapshot,
                                                        );
                                                    }
                                                    warn!(
                                                        "IPC ingress queue closed, dropping transaction hash={} depth={} received={} dropped={}",
                                                        hash,
                                                        snapshot.queue_depth,
                                                        snapshot.total_received,
                                                        snapshot.total_dropped
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Prevent unbounded growth
                    if pending.capacity() > 10_000_000 {
                        pending = Vec::with_capacity(1024 * 1024);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data ready, yield briefly
                    tokio::time::sleep(std::time::Duration::from_micros(10)).await;
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn get_transactions(&self, max: usize) -> Result<Vec<MempoolTransaction>> {
        let mut txs = Vec::with_capacity(max);

        {
            let mut receiver = self.critical_tx_receiver.lock().await;
            while txs.len() < max {
                match receiver.try_recv() {
                    Ok(tx) => {
                        txs.push(tx);
                        self.critical_queue_size.fetch_sub(1, Ordering::Relaxed);
                    }
                    Err(_) => break,
                }
            }
        }

        let mut receiver = self.tx_receiver.lock().await;

        // Get first transaction (with timeout)
        if txs.is_empty() {
            match tokio::time::timeout(std::time::Duration::from_millis(25), receiver.recv()).await
            {
                Ok(Some(tx)) => {
                    txs.push(tx);
                    self.queue_size.fetch_sub(1, Ordering::Relaxed);
                }
                Ok(None) => return Err(eyre!("Channel closed")),
                Err(_) => return Ok(txs), // Timeout
            }
        }

        // Get more without blocking
        while txs.len() < max {
            match receiver.try_recv() {
                Ok(tx) => {
                    txs.push(tx);
                    self.queue_size.fetch_sub(1, Ordering::Relaxed);
                }
                Err(_) => break,
            }
        }

        // Update stats with current queue size
        let normal_queue_size = self.queue_size.load(Ordering::Relaxed);
        let critical_queue_size = self.critical_queue_size.load(Ordering::Relaxed);
        let mut stats = self.stats.write().await;
        stats.queue_size = normal_queue_size + critical_queue_size;
        stats.critical_queue_size = critical_queue_size;

        Ok(txs)
    }

    /// Get transactions instantly without any waiting - for ultra-low latency
    pub async fn get_transactions_instant(&self, max: usize) -> Vec<MempoolTransaction> {
        let mut txs = Vec::with_capacity(max);

        {
            let mut receiver = self.critical_tx_receiver.lock().await;
            while txs.len() < max {
                match receiver.try_recv() {
                    Ok(tx) => {
                        txs.push(tx);
                        self.critical_queue_size.fetch_sub(1, Ordering::Relaxed);
                    }
                    Err(_) => break,
                }
            }
        }

        let mut receiver = self.tx_receiver.lock().await;

        // No waiting - just drain what's available immediately
        while txs.len() < max {
            match receiver.try_recv() {
                Ok(tx) => {
                    txs.push(tx);
                    self.queue_size.fetch_sub(1, Ordering::Relaxed);
                }
                Err(_) => break,
            }
        }

        // Update stats with current queue size
        let normal_queue_size = self.queue_size.load(Ordering::Relaxed);
        let critical_queue_size = self.critical_queue_size.load(Ordering::Relaxed);
        let mut stats = self.stats.write().await;
        stats.queue_size = normal_queue_size + critical_queue_size;
        stats.critical_queue_size = critical_queue_size;

        txs
    }

    pub async fn get_stats(&self) -> Stats {
        let normal_queue_size = self.queue_size.load(Ordering::Relaxed);
        let critical_queue_size = self.critical_queue_size.load(Ordering::Relaxed);
        let mut stats = self.stats.write().await;
        stats.queue_size = normal_queue_size + critical_queue_size;
        stats.critical_queue_size = critical_queue_size;
        stats.clone()
    }
}

async fn route_critical_tx(
    tx: MempoolTransaction,
    preserve_kind: PreserveCandidate,
    route_reason: CriticalRouteReason,
    critical_tx_sender: &mpsc::Sender<MempoolTransaction>,
    stats: &Arc<RwLock<Stats>>,
    queue_size: &Arc<AtomicUsize>,
    critical_queue_size: &Arc<AtomicUsize>,
    observer: Option<&Arc<dyn MempoolIngressObserver>>,
    first_seen_ms: u64,
) {
    let hash = tx.hash.clone();
    match critical_tx_sender.try_send(tx) {
        Ok(()) => {
            let critical_depth = critical_queue_size.fetch_add(1, Ordering::Relaxed) + 1;
            let mut stats = stats.write().await;
            stats.critical_received += 1;
            stats.critical_queue_size = critical_depth;
            stats.queue_size = queue_size.load(Ordering::Relaxed) + critical_depth;
            let snapshot = IngressStatsSnapshot {
                queue_depth: stats.queue_size,
                total_received: stats.total,
                total_dropped: stats.total_dropped,
            };
            drop(stats);
            if let Some(observer) = observer {
                observer.queue_depth_updated(snapshot);
            }
            match route_reason {
                CriticalRouteReason::FastLane => {
                    debug!(
                        "Routed {} candidate to critical mempool lane hash={} normal_depth={} critical_depth={} received={} dropped={}",
                        preserve_kind.label(),
                        hash,
                        queue_size.load(Ordering::Relaxed),
                        critical_depth,
                        snapshot.total_received,
                        snapshot.total_dropped
                    );
                }
                CriticalRouteReason::Backpressure => {
                    warn!(
                        "IPC ingress queue full, routed {} candidate to critical queue hash={} normal_depth={} critical_depth={} received={} dropped={}",
                        preserve_kind.label(),
                        hash,
                        queue_size.load(Ordering::Relaxed),
                        critical_depth,
                        snapshot.total_received,
                        snapshot.total_dropped
                    );
                }
            }
        }
        Err(e) => {
            let mut stats = stats.write().await;
            stats.total_dropped += 1;
            stats.critical_dropped += 1;
            stats.queue_size = total_queue_depth(queue_size, critical_queue_size);
            stats.critical_queue_size = critical_queue_size.load(Ordering::Relaxed);
            let snapshot = IngressStatsSnapshot {
                queue_depth: stats.queue_size,
                total_received: stats.total,
                total_dropped: stats.total_dropped,
            };
            drop(stats);
            if let Some(observer) = observer {
                observer.tx_dropped_queue_full(&hash, first_seen_ms, snapshot);
            }
            warn!(
                "CRITICAL mempool ingress queue full, dropping {} candidate hash={} depth={} received={} dropped={} error={}",
                preserve_kind.label(),
                hash,
                snapshot.queue_depth,
                snapshot.total_received,
                snapshot.total_dropped,
                e
            );
        }
    }
}

fn total_queue_depth(
    queue_size: &Arc<AtomicUsize>,
    critical_queue_size: &Arc<AtomicUsize>,
) -> usize {
    queue_size.load(Ordering::Relaxed) + critical_queue_size.load(Ordering::Relaxed)
}

#[derive(Clone, Copy)]
enum PreserveCandidate {
    LiquidityApproval,
    PositionApproval,
    LiquidityRemoval,
}

#[derive(Clone, Copy)]
enum CriticalRouteReason {
    FastLane,
    Backpressure,
}

impl PreserveCandidate {
    fn label(self) -> &'static str {
        match self {
            Self::LiquidityApproval => "liquidity-approval",
            Self::PositionApproval => "position-approval",
            Self::LiquidityRemoval => "liquidity-removal",
        }
    }
}

async fn critical_lane_candidate(
    tx: &MempoolTransaction,
    signal_filter_cache: Option<&Arc<TokenTrackingCache>>,
) -> Option<PreserveCandidate> {
    if let Some(approval) = decode_liquidity_approval_call(tx) {
        if approval.amount != U256::ZERO
            && is_tracked_liquidity_ownership_token(&approval.ownership_token, signal_filter_cache)
                .await
        {
            return Some(PreserveCandidate::LiquidityApproval);
        }
    }

    if is_tracked_position_approval(tx, signal_filter_cache).await {
        return Some(PreserveCandidate::PositionApproval);
    }

    if is_tracked_liquidity_removal(tx, signal_filter_cache).await {
        return Some(PreserveCandidate::LiquidityRemoval);
    }

    None
}

async fn should_enqueue_for_signal_path(
    tx: &MempoolTransaction,
    signal_filter_cache: Option<&Arc<TokenTrackingCache>>,
) -> bool {
    let Some(cache) = signal_filter_cache else {
        return true;
    };

    // Contract creation is sparse and may define a token/pool before the cache
    // can know the addresses, so keep it on the signal path.
    if tx.to.is_none() {
        return true;
    }

    if let Some(from) = address_string_from_bytes(&tx.from) {
        if cache.is_creator(&from).await {
            return true;
        }
    }

    if let Some(to) = tx.to.as_ref().and_then(|to| address_string_from_bytes(to)) {
        if cache.is_creator(&to).await || cache.is_pool(&to).await {
            return true;
        }

        if cache.is_token(&to).await {
            return !is_plain_token_noise(tx);
        }
    }

    if is_tracked_liquidity_removal(tx, signal_filter_cache).await {
        return true;
    }

    false
}

async fn is_tracked_liquidity_ownership_token(
    ownership_token: &alloy_primitives::Address,
    signal_filter_cache: Option<&Arc<TokenTrackingCache>>,
) -> bool {
    let Some(cache) = signal_filter_cache else {
        return true;
    };
    let ownership_token = address_string(ownership_token);
    cache.is_liquidity_ownership_token(&ownership_token).await
        || cache.is_pool(&ownership_token).await
}

async fn is_tracked_position_approval(
    tx: &MempoolTransaction,
    signal_filter_cache: Option<&Arc<TokenTrackingCache>>,
) -> bool {
    let Some(approval) = decode_position_approval_call(tx) else {
        return false;
    };

    let manager = approval.position_manager();
    let Some(cache) = signal_filter_cache else {
        return is_known_position_manager_candidate(&manager);
    };
    cache.is_position_manager(&address_string(&manager)).await
}

async fn is_tracked_liquidity_removal(
    tx: &MempoolTransaction,
    signal_filter_cache: Option<&Arc<TokenTrackingCache>>,
) -> bool {
    if !is_protocol_liquidity_removal_candidate(tx) {
        return false;
    }

    let Some(cache) = signal_filter_cache else {
        return true;
    };

    for candidate in liquidity_removal_token_candidates(&tx.input) {
        if cache.is_token(&candidate).await || cache.is_pool(&candidate).await {
            return true;
        }
    }

    if let Some(to) = tx.to.as_ref().and_then(|to| address_string_from_bytes(to)) {
        if cache.is_pool(&to).await {
            return true;
        }

        if is_v4_modify_liquidity_candidate(tx) && cache.is_position_manager(&to).await {
            return true;
        }

        if tx.input.get(0..4) == Some([0x0c, 0x49, 0xcc, 0xbe].as_slice())
            && cache.is_position_manager(&to).await
            && tx.input.len() >= 36
        {
            let token_id = U256::from_be_slice(&tx.input[4..36]);
            if cache
                .position_context_by_token_id(&to, token_id)
                .await
                .is_some()
            {
                return true;
            }
        }
    }

    false
}

fn address_string(address: &alloy_primitives::Address) -> String {
    format!("0x{}", hex::encode(address.as_slice()))
}

fn address_string_from_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.len() != 20 {
        return None;
    }
    Some(format!("0x{}", hex::encode(bytes)))
}

fn is_plain_token_noise(tx: &MempoolTransaction) -> bool {
    let Some(selector) = tx.input.get(0..4) else {
        return false;
    };
    matches!(
        selector,
        [0xa9, 0x05, 0x9c, 0xbb] | [0x23, 0xb8, 0x72, 0xdd]
    ) || is_liquidity_approval_selector(selector)
}

use std::io::Read;

impl std::fmt::Debug for MempoolFetcherIPCClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MempoolFetcherIPCClient")
            .field("socket_path", &self.socket_path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Address};
    use serde_json::json;
    use std::collections::HashMap;

    use crate::token_tracking::{
        types::PoolLifecycle, CacheConfig, Pool, PoolType, Token, TokenTrackingCache, TokenUpdate,
        TokenWithPools,
    };

    #[tokio::test]
    async fn preserves_nonzero_erc20_approval_without_signal_filter() {
        let tx = tx(
            Some(address!("1111111111111111111111111111111111111111")),
            erc20_approve_calldata(
                address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
                U256::from(1000u64),
            ),
        );

        assert!(matches!(
            critical_lane_candidate(&tx, None).await,
            Some(PreserveCandidate::LiquidityApproval)
        ));
    }

    #[tokio::test]
    async fn does_not_preserve_zero_erc20_approval_when_ingress_is_full() {
        let tx = tx(
            Some(address!("1111111111111111111111111111111111111111")),
            erc20_approve_calldata(
                address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
                U256::ZERO,
            ),
        );

        assert!(critical_lane_candidate(&tx, None).await.is_none());
    }

    #[tokio::test]
    async fn preserves_position_approval_without_signal_filter() {
        let tx = tx(
            Some(address!("C36442b4a4522E871399CD717aBDD847Ab11FE88")),
            set_approval_for_all_calldata(
                address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
                true,
            ),
        );

        assert!(matches!(
            critical_lane_candidate(&tx, None).await,
            Some(PreserveCandidate::PositionApproval)
        ));
    }

    #[tokio::test]
    async fn preserves_liquidity_removal_without_signal_filter() {
        let token = address!("1111111111111111111111111111111111111111");
        let tx = tx(
            Some(address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D")),
            remove_liquidity_eth_calldata(token),
        );

        assert!(matches!(
            critical_lane_candidate(&tx, None).await,
            Some(PreserveCandidate::LiquidityRemoval)
        ));
    }

    #[tokio::test]
    async fn filters_unrelated_regular_transfer_from_signal_queue() {
        let cache = Arc::new(test_cache().await);
        let mut tx = tx(
            Some(address!("1111111111111111111111111111111111111111")),
            erc20_transfer_calldata(address!("2222222222222222222222222222222222222222")),
        );
        tx.from = address!("dddddddddddddddddddddddddddddddddddddddd").to_vec();

        assert!(!should_enqueue_for_signal_path(&tx, Some(&cache)).await);
        assert!(critical_lane_candidate(&tx, Some(&cache)).await.is_none());
    }

    #[tokio::test]
    async fn filters_plain_transfer_to_tracked_token_from_unrelated_sender() {
        let cache = Arc::new(test_cache().await);
        let mut tx = tx(
            Some(address!("9999999999999999999999999999999999999999")),
            erc20_transfer_calldata(address!("2222222222222222222222222222222222222222")),
        );
        tx.from = address!("dddddddddddddddddddddddddddddddddddddddd").to_vec();

        assert!(!should_enqueue_for_signal_path(&tx, Some(&cache)).await);
    }

    #[tokio::test]
    async fn keeps_tracked_creator_tx_on_signal_queue() {
        let cache = Arc::new(test_cache().await);
        let mut tx = tx(
            Some(address!("1111111111111111111111111111111111111111")),
            Vec::new(),
        );
        tx.from = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec();

        assert!(should_enqueue_for_signal_path(&tx, Some(&cache)).await);
    }

    #[tokio::test]
    async fn routes_tracked_lp_approval_to_critical_queue() {
        let cache = Arc::new(test_cache().await);
        let tx = tx(
            Some(address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")),
            erc20_approve_calldata(
                address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
                U256::from(1000u64),
            ),
        );

        assert!(matches!(
            critical_lane_candidate(&tx, Some(&cache)).await,
            Some(PreserveCandidate::LiquidityApproval)
        ));
    }

    #[tokio::test]
    async fn does_not_route_untracked_lp_approval_to_signal_queue() {
        let cache = Arc::new(test_cache().await);
        let mut tx = tx(
            Some(address!("1111111111111111111111111111111111111111")),
            erc20_approve_calldata(
                address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
                U256::from(1000u64),
            ),
        );
        tx.from = address!("dddddddddddddddddddddddddddddddddddddddd").to_vec();

        assert!(critical_lane_candidate(&tx, Some(&cache)).await.is_none());
        assert!(!should_enqueue_for_signal_path(&tx, Some(&cache)).await);
    }

    #[tokio::test]
    async fn drains_critical_queue_before_normal_queue() {
        let client = MempoolFetcherIPCClient::new(Some("/tmp/not-used.sock")).unwrap();
        let mut normal = tx(
            Some(address!("1111111111111111111111111111111111111111")),
            Vec::new(),
        );
        normal.hash = "normal".to_string();
        let mut critical = tx(
            Some(address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D")),
            remove_liquidity_eth_calldata(address!("2222222222222222222222222222222222222222")),
        );
        critical.hash = "critical".to_string();

        client.tx_sender.try_send(normal).unwrap();
        client.queue_size.fetch_add(1, Ordering::Relaxed);
        client.critical_tx_sender.try_send(critical).unwrap();
        client.critical_queue_size.fetch_add(1, Ordering::Relaxed);

        let txs = client.get_transactions(2).await.unwrap();
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].hash, "critical");
        assert_eq!(txs[1].hash, "normal");

        let stats = client.get_stats().await;
        assert_eq!(stats.queue_size, 0);
        assert_eq!(stats.critical_queue_size, 0);
    }

    fn tx(to: Option<Address>, input: Vec<u8>) -> MempoolTransaction {
        MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec(),
            to: to.map(|address| address.to_vec()),
            input,
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: Vec::new(),
            function_category: None,
        }
    }

    fn erc20_approve_calldata(spender: Address, amount: U256) -> Vec<u8> {
        let mut input = vec![0x09, 0x5e, 0xa7, 0xb3];
        input.extend_from_slice(&pad_address(spender));
        input.extend_from_slice(&amount.to_be_bytes::<32>());
        input
    }

    fn erc20_transfer_calldata(to: Address) -> Vec<u8> {
        let mut input = vec![0xa9, 0x05, 0x9c, 0xbb];
        input.extend_from_slice(&pad_address(to));
        input.extend_from_slice(&U256::from(1u64).to_be_bytes::<32>());
        input
    }

    fn set_approval_for_all_calldata(operator: Address, approved: bool) -> Vec<u8> {
        let mut input = vec![0xa2, 0x2c, 0xb4, 0x65];
        input.extend_from_slice(&pad_address(operator));
        input.extend_from_slice(&U256::from(approved as u8).to_be_bytes::<32>());
        input
    }

    fn remove_liquidity_eth_calldata(token: Address) -> Vec<u8> {
        let mut input = vec![0x0c, 0x49, 0xcc, 0xbe];
        input.extend_from_slice(&pad_address(token));
        input
    }

    fn pad_address(address: Address) -> [u8; 32] {
        let mut padded = [0u8; 32];
        padded[12..].copy_from_slice(address.as_slice());
        padded
    }

    async fn test_cache() -> TokenTrackingCache {
        let cache = TokenTrackingCache::new(CacheConfig::default());
        let token_address = "0x9999999999999999999999999999999999999999".to_string();
        let creator_address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
        let pool_address = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();

        let token = Token {
            address: token_address.clone(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            decimals: 18,
            total_supply: Some("1000000".to_string()),
            creator_address,
            current_owner: "0xcccccccccccccccccccccccccccccccccccccccc".to_string(),
            tax_setter_addresses: Vec::new(),
            ownership_renounced: false,
            renouncement_block: None,
            buy_tax: Some(0.0),
            sell_tax: Some(0.0),
            last_tax_change_block: None,
            tax_history: Vec::new(),
            creation_block: 1,
            creation_tx: "0xcreate".to_string(),
            creation_timestamp: None,
            latest_activity_block: 1,
            is_scam: false,
            scam_label: None,
            total_liquidity: 0.0,
        };

        let pool = Pool {
            address: pool_address.clone(),
            token_address: token_address.clone(),
            pool_type: PoolType::UniswapV2,
            token_reserve: 1_000_000.0,
            eth_reserve: 10.0,
            denom_currency: "WETH".to_string(),
            denom_address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
            trading_enabled: true,
            trading_enabled_block: Some(1),
            trading_enabled_tx: None,
            fee_tier: None,
            pool_id: None,
            lp_token_address: Some(pool_address.clone()),
            position_manager_address: None,
            lp_total_supply: Some(1.0),
            liquidity_positions: Vec::new(),
            last_updated_block: 1,
            last_updated_time: 1.0,
            is_scam: false,
            scam_label: None,
            lp_tokens_approved_percentage: None,
            lifecycle: PoolLifecycle::Active,
            control_addresses: Vec::new(),
            can_buy: true,
            can_sell: true,
            received_at: Instant::now(),
        };

        let mut pools = HashMap::new();
        pools.insert(pool_address, pool);
        let mut data = HashMap::new();
        data.insert(token_address, TokenWithPools { token, pools });

        cache
            .batch_update(TokenUpdate {
                message_type: "test".to_string(),
                token_count: 1,
                block_number: 1,
                timestamp: 1.0,
                data,
            })
            .await;
        cache
    }
}
