use std::fs::OpenOptions;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use mempool_processor::{
    function_detector::{CreatorFunctionType, FunctionDetector},
    mempool_fetcher::{MempoolFetcherIPCClient, MempoolTransaction},
    simulator::{MempoolSimulator, SimulationManager, SimulationType, TxSimulationJob},
    tx_router::{RouteOrigin, TransactionCategory, TransactionRouter},
    unresolved_intents::{UnresolvedIntentKind, UnresolvedIntentStore},
};
use tokio::{signal, time};
use tracing::{info, warn};

use super::{
    mempool_transaction_hash::parse_mempool_transaction_hash_or_zero,
    service_metrics::ServiceMetrics,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DetectorLane {
    Normal,
    Critical,
}

#[derive(Clone, Copy)]
enum DetectorStage {
    FunctionDetectBatch,
    MetricsAddDetectionLatency,
    RecordPendingNonceDependency,
    RecordPendingFundingDependency,
    TxRouterClassify,
    UnresolvedIntentRecord,
    DetectLpApproval,
    UnresolvedIntentResolveLp,
    UnresolvedIntentRecordLp,
    SimulationSubmit,
}

impl DetectorLane {
    fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Critical => "critical",
        }
    }

    fn stage(self, stage: DetectorStage) -> &'static str {
        match (self, stage) {
            (Self::Normal, DetectorStage::FunctionDetectBatch) => "function_detect_batch",
            (Self::Critical, DetectorStage::FunctionDetectBatch) => {
                "critical_function_detect_batch"
            }
            (Self::Normal, DetectorStage::MetricsAddDetectionLatency) => {
                "metrics_add_detection_latency"
            }
            (Self::Critical, DetectorStage::MetricsAddDetectionLatency) => {
                "critical_metrics_add_detection_latency"
            }
            (Self::Normal, DetectorStage::RecordPendingNonceDependency) => {
                "record_pending_nonce_dependency"
            }
            (Self::Critical, DetectorStage::RecordPendingNonceDependency) => {
                "critical_record_pending_nonce_dependency"
            }
            (Self::Normal, DetectorStage::RecordPendingFundingDependency) => {
                "record_pending_funding_dependency"
            }
            (Self::Critical, DetectorStage::RecordPendingFundingDependency) => {
                "critical_record_pending_funding_dependency"
            }
            (Self::Normal, DetectorStage::TxRouterClassify) => "tx_router_classify",
            (Self::Critical, DetectorStage::TxRouterClassify) => "critical_tx_router_classify",
            (Self::Normal, DetectorStage::UnresolvedIntentRecord) => "unresolved_intent_record",
            (Self::Critical, DetectorStage::UnresolvedIntentRecord) => {
                "critical_unresolved_intent_record"
            }
            (Self::Normal, DetectorStage::DetectLpApproval) => "detect_lp_approval",
            (Self::Critical, DetectorStage::DetectLpApproval) => "critical_detect_lp_approval",
            (Self::Normal, DetectorStage::UnresolvedIntentResolveLp) => {
                "unresolved_intent_resolve_lp"
            }
            (Self::Critical, DetectorStage::UnresolvedIntentResolveLp) => {
                "critical_unresolved_intent_resolve_lp"
            }
            (Self::Normal, DetectorStage::UnresolvedIntentRecordLp) => {
                "unresolved_intent_record_lp"
            }
            (Self::Critical, DetectorStage::UnresolvedIntentRecordLp) => {
                "critical_unresolved_intent_record_lp"
            }
            (Self::Normal, DetectorStage::SimulationSubmit) => "simulation_submit",
            (Self::Critical, DetectorStage::SimulationSubmit) => "critical_simulation_submit",
        }
    }
}

pub(crate) fn spawn_critical_signal_consumer(
    ipc_client: MempoolFetcherIPCClient,
    tx_router: Arc<TransactionRouter>,
    simulation_manager: SimulationManager,
    metrics: Arc<ServiceMetrics>,
    unresolved_intent_store: UnresolvedIntentStore,
    detector_timing: DetectorLoopTiming,
    shutdown: Arc<AtomicBool>,
    batch_size: usize,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let function_detector = FunctionDetector::new();
        let mut consecutive_empty = 0u64;

        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            let new_txs = {
                let _stage = detector_timing.stage("critical_ipc_get_transactions_instant");
                ipc_client
                    .get_critical_transactions_instant(batch_size)
                    .await
            };

            let mut idle_sleep = None;
            if new_txs.is_empty() {
                consecutive_empty += 1;
                idle_sleep = Some(match consecutive_empty {
                    1..=10 => Duration::from_micros(100),
                    11..=100 => Duration::from_millis(1),
                    _ => Duration::from_millis(10),
                });
            } else {
                consecutive_empty = 0;
                process_signal_path_transactions(
                    new_txs,
                    &function_detector,
                    tx_router.as_ref(),
                    &simulation_manager,
                    metrics.as_ref(),
                    &unresolved_intent_store,
                    &detector_timing,
                    DetectorLane::Critical,
                )
                .await;
            }

            if let Some(sleep_time) = idle_sleep {
                time::sleep(sleep_time).await;
            }
            detector_timing.mark_loop_completed();
        }

        info!("Critical mempool signal consumer stopped");
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_signal_path_transactions(
    new_txs: Vec<MempoolTransaction>,
    function_detector: &FunctionDetector,
    tx_router: &TransactionRouter,
    simulation_manager: &SimulationManager,
    metrics: &ServiceMetrics,
    unresolved_intent_store: &UnresolvedIntentStore,
    detector_timing: &DetectorLoopTiming,
    lane: DetectorLane,
) {
    let transactions_with_functions = {
        let _stage = detector_timing.stage(lane.stage(DetectorStage::FunctionDetectBatch));
        function_detector.detect_batch(new_txs)
    };

    for tx in transactions_with_functions {
        metrics.total_processed.fetch_add(1, Ordering::Relaxed);
        let _ = run_detector_stage(
            detector_timing,
            lane,
            DetectorStage::MetricsAddDetectionLatency,
            Duration::from_millis(500),
            &tx.hash,
            metrics.add_detection_latency(Duration::from_nanos(tx.detection_ns)),
        )
        .await;
        let _ = run_detector_stage(
            detector_timing,
            lane,
            DetectorStage::RecordPendingNonceDependency,
            Duration::from_millis(500),
            &tx.hash,
            simulation_manager.record_pending_nonce_dependency(&tx),
        )
        .await;
        let _ = run_detector_stage(
            detector_timing,
            lane,
            DetectorStage::RecordPendingFundingDependency,
            Duration::from_millis(500),
            &tx.hash,
            simulation_manager.record_pending_funding_dependency(&tx),
        )
        .await;

        let Some(classification) = run_detector_stage(
            detector_timing,
            lane,
            DetectorStage::TxRouterClassify,
            Duration::from_secs(2),
            &tx.hash,
            tx_router.classify(&tx),
        )
        .await
        else {
            continue;
        };
        tx_router.observe_route(&tx, &classification, RouteOrigin::MempoolIngress);
        match &classification.category {
            TransactionCategory::ContractCreation { .. }
            | TransactionCategory::CreatorTransaction { .. } => {}
            _ => {
                if let Some((kind, reason)) = tx_router.unresolved_intent_for(&tx, &classification)
                {
                    let _ = run_detector_stage(
                        detector_timing,
                        lane,
                        DetectorStage::UnresolvedIntentRecord,
                        Duration::from_millis(500),
                        &tx.hash,
                        unresolved_intent_store.record(tx.clone(), kind, reason),
                    )
                    .await;
                } else if lane == DetectorLane::Critical {
                    warn!(
                        "critical lane tx classified as non-actionable hash={} category={:?}",
                        tx.hash, classification.category
                    );
                }
                continue;
            }
        }

        if !classification.requires_simulation {
            if let TransactionCategory::CreatorTransaction {
                function_type: CreatorFunctionType::LiquidityPoolApproval,
                ..
            } = &classification.category
            {
                let published = run_detector_stage(
                    detector_timing,
                    lane,
                    DetectorStage::DetectLpApproval,
                    Duration::from_secs(2),
                    &tx.hash,
                    simulation_manager.detect_lp_approval(&tx, &classification.category),
                )
                .await
                .unwrap_or(false);
                if published {
                    let _ = run_detector_stage(
                        detector_timing,
                        lane,
                        DetectorStage::UnresolvedIntentResolveLp,
                        Duration::from_millis(500),
                        &tx.hash,
                        unresolved_intent_store.resolve(&tx.hash),
                    )
                    .await;
                } else {
                    let _ = run_detector_stage(
                        detector_timing,
                        lane,
                        DetectorStage::UnresolvedIntentRecordLp,
                        Duration::from_millis(500),
                        &tx.hash,
                        unresolved_intent_store.record(
                            tx.clone(),
                            UnresolvedIntentKind::LpApproval,
                            "LP approval enrichment failed after routing",
                        ),
                    )
                    .await;
                }
            }
            continue;
        }

        let sim_request = TxSimulationJob {
            tx: tx.clone(),
            category: classification.category.clone(),
            priority: classification.priority,
            simulation_type: match &classification.category {
                TransactionCategory::ContractCreation { .. }
                | TransactionCategory::CreatorTransaction { .. } => {
                    SimulationType::TransactionWithBuySell
                }
                _ => SimulationType::TransactionOnly,
            },
            tx_hash: parse_mempool_transaction_hash_or_zero(&tx.hash),
        };

        let submit_result = run_detector_stage(
            detector_timing,
            lane,
            DetectorStage::SimulationSubmit,
            Duration::from_secs(1),
            &tx.hash,
            simulation_manager.submit(sim_request),
        )
        .await;
        match submit_result {
            Some(Ok(())) => {
                metrics
                    .simulations_submitted
                    .fetch_add(1, Ordering::Relaxed);
            }
            Some(Err(e)) => {
                metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
                warn!(
                    "{} simulation submission error for {}: {}",
                    lane.label(),
                    tx.hash,
                    e
                );
            }
            None => {
                metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

async fn run_detector_stage<T, F>(
    detector_timing: &DetectorLoopTiming,
    lane: DetectorLane,
    stage: DetectorStage,
    timeout: Duration,
    tx_hash: &str,
    future: F,
) -> Option<T>
where
    F: Future<Output = T>,
{
    let stage_name = lane.stage(stage);
    let _stage = detector_timing.stage(stage_name);
    match time::timeout(timeout, future).await {
        Ok(value) => Some(value),
        Err(_) => {
            warn!(
                "detector stage timed out lane={} stage={} tx={} timeout_ms={}",
                lane.label(),
                stage_name,
                tx_hash,
                timeout.as_millis()
            );
            None
        }
    }
}

#[derive(Clone)]
pub(crate) struct DetectorLoopTiming {
    inner: Arc<StdMutex<DetectorLoopTimingState>>,
}

struct DetectorLoopTimingState {
    current_stage: &'static str,
    stage_started: Instant,
    last_loop_completed: Instant,
    completed_loops: u64,
    max_stage: &'static str,
    max_stage_ms: u128,
    slow_stage_count: u64,
}

pub(crate) struct DetectorLoopTimingSnapshot {
    pub(crate) current_stage: &'static str,
    pub(crate) current_stage_ms: u128,
    pub(crate) last_loop_ms_ago: u128,
    pub(crate) completed_loops: u64,
    pub(crate) max_stage: &'static str,
    pub(crate) max_stage_ms: u128,
    pub(crate) slow_stage_count: u64,
}

pub(crate) struct DetectorStageGuard {
    timing: DetectorLoopTiming,
    stage: &'static str,
    started: Instant,
}

impl DetectorLoopTiming {
    pub(crate) fn new() -> Self {
        let now = Instant::now();
        Self {
            inner: Arc::new(StdMutex::new(DetectorLoopTimingState {
                current_stage: "idle",
                stage_started: now,
                last_loop_completed: now,
                completed_loops: 0,
                max_stage: "none",
                max_stage_ms: 0,
                slow_stage_count: 0,
            })),
        }
    }

    pub(crate) fn stage(&self, stage: &'static str) -> DetectorStageGuard {
        let now = Instant::now();
        {
            let mut state = self.inner.lock().expect("detector timing mutex poisoned");
            state.current_stage = stage;
            state.stage_started = now;
        }
        DetectorStageGuard {
            timing: self.clone(),
            stage,
            started: now,
        }
    }

    pub(crate) fn mark_loop_completed(&self) {
        let mut state = self.inner.lock().expect("detector timing mutex poisoned");
        state.completed_loops += 1;
        state.last_loop_completed = Instant::now();
    }

    pub(crate) fn snapshot(&self) -> DetectorLoopTimingSnapshot {
        let now = Instant::now();
        let state = self.inner.lock().expect("detector timing mutex poisoned");
        DetectorLoopTimingSnapshot {
            current_stage: state.current_stage,
            current_stage_ms: now.duration_since(state.stage_started).as_millis(),
            last_loop_ms_ago: now.duration_since(state.last_loop_completed).as_millis(),
            completed_loops: state.completed_loops,
            max_stage: state.max_stage,
            max_stage_ms: state.max_stage_ms,
            slow_stage_count: state.slow_stage_count,
        }
    }
}

impl Drop for DetectorStageGuard {
    fn drop(&mut self) {
        let elapsed_ms = self.started.elapsed().as_millis();
        let mut state = self
            .timing
            .inner
            .lock()
            .expect("detector timing mutex poisoned");
        if elapsed_ms > state.max_stage_ms {
            state.max_stage_ms = elapsed_ms;
            state.max_stage = self.stage;
        }
        if elapsed_ms >= 250 {
            state.slow_stage_count += 1;
            warn!(
                "slow detector stage stage={} elapsed_ms={}",
                self.stage, elapsed_ms
            );
        }
        if state.current_stage == self.stage {
            state.current_stage = "idle";
            state.stage_started = Instant::now();
        }
    }
}

pub(crate) fn spawn_detector_timing_watchdog(
    lane: &'static str,
    timing: DetectorLoopTiming,
    shutdown: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        while !shutdown.load(Ordering::Relaxed) {
            time::sleep(Duration::from_secs(5)).await;
            let snapshot = timing.snapshot();
            if snapshot.current_stage != "idle" && snapshot.current_stage_ms >= 5_000 {
                warn!(
                    "detector consumer stage appears stuck lane={} stage={} elapsed_ms={} loops={} last_loop_ms_ago={} max_stage={} max_stage_ms={} slow_stages={}",
                    lane,
                    snapshot.current_stage,
                    snapshot.current_stage_ms,
                    snapshot.completed_loops,
                    snapshot.last_loop_ms_ago,
                    snapshot.max_stage,
                    snapshot.max_stage_ms,
                    snapshot.slow_stage_count
                );
            }
        }
    });
}

pub(crate) fn setup_shutdown_handler() -> Arc<AtomicBool> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c().await.unwrap_or_else(|e| {
                eprintln!("Failed to install Ctrl+C handler: {}", e);
                std::process::exit(1);
            });
        };

        #[cfg(unix)]
        let terminate = async {
            match signal::unix::signal(signal::unix::SignalKind::terminate()) {
                Ok(mut stream) => stream.recv().await,
                Err(e) => {
                    eprintln!("Failed to install SIGTERM handler: {}", e);
                    std::future::pending().await
                }
            };
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C signal");
            }
            _ = terminate => {
                info!("Received SIGTERM signal");
            }
        }

        shutdown_clone.store(true, Ordering::Relaxed);
    });

    shutdown
}

pub(crate) fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            let value = value.trim();
            value == "1"
                || value.eq_ignore_ascii_case("true")
                || value.eq_ignore_ascii_case("yes")
                || value.eq_ignore_ascii_case("on")
        })
        .unwrap_or(false)
}

fn append_line_to_file(path: &Path, line: &str) {
    if let Err(err) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| {
            use std::io::Write;
            writeln!(file, "{}", line)
        })
    {
        warn!(
            "Failed to write external update log at {}: {}",
            path.display(),
            err
        );
    }
}

pub(crate) fn schedule_latest_simulation_status_log(
    mempool_simulator: Arc<MempoolSimulator>,
    external_data_log_path: PathBuf,
    probe_in_flight: Arc<AtomicBool>,
) {
    if probe_in_flight.swap(true, Ordering::Relaxed) {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        append_line_to_file(
            &external_data_log_path,
            &format!(
                "[{}]  WARN 📡 Skipping latest simulation block probe; previous probe is still running",
                timestamp
            ),
        );
        return;
    }

    tokio::spawn(async move {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let status_task = tokio::task::spawn_blocking(move || {
            mempool_simulator.latest_simulation_status_blocking()
        });

        match time::timeout(Duration::from_secs(2), status_task).await {
            Ok(Ok(Ok(status))) => {
                let line = format!(
                    "[{}]  INFO 📡 Latest simulation block target: {} source={:?} reth_finished={} historical_context={} live_head={:?} tracked_state={:?}",
                    timestamp,
                    status.selected_block_number,
                    status.source,
                    status.latest_reth_finished_block_number,
                    status.latest_historical_context_block_number,
                    status.latest_live_block_number,
                    status.latest_tracked_state_block_number
                );
                append_line_to_file(&external_data_log_path, &line);
                probe_in_flight.store(false, Ordering::Relaxed);
            }
            Ok(Ok(Err(err))) => {
                let line = format!(
                    "[{}]  WARN 📡 Unable to determine latest simulation block: {}",
                    timestamp, err
                );
                append_line_to_file(&external_data_log_path, &line);
                probe_in_flight.store(false, Ordering::Relaxed);
            }
            Ok(Err(err)) => {
                let line = format!(
                    "[{}]  WARN 📡 Latest simulation block probe task failed: {}",
                    timestamp, err
                );
                append_line_to_file(&external_data_log_path, &line);
                probe_in_flight.store(false, Ordering::Relaxed);
            }
            Err(_) => {
                let line = format!(
                    "[{}]  WARN 📡 Latest simulation block probe timed out after 2s; disabling further probes for this process",
                    timestamp
                );
                append_line_to_file(&external_data_log_path, &line);
            }
        }
    });
}
