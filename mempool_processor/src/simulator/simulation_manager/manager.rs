use super::block_pruner::spawn_block_pruner;
use super::logging::log_simulation_start;
use super::pending_nonce_dependencies::{PendingNonceDependencies, PendingNonceDependencyStats};
use super::pending_sequences::{PendingSequences, SequenceKey};
use super::request_queue::{ManagerStats, RequestQueue};
use super::types::{SimulationResult, TxSimulationJob};
use super::{LiquidityRemovalSimulator, MempoolSimulator};
use crate::signal_detector::{SignalManager, SignalManagerConfig};
use crate::token_tracking::TokenTrackingCache;
use crate::tx_router::TransactionCategory;
use eyre::Result as EyreResult;
/// Simulation Manager
///
/// Manages transaction simulations on a PER-POOL basis.
///
/// Key Architecture:
/// - Each token can have multiple pools (WETH/TOKEN, USDC/TOKEN, etc.)
/// - Each pool is simulated INDEPENDENTLY
/// - Each pool generates its own signal with pool-specific data
/// - Signal = f(token_address, pool_address)
///
/// Flow for CreatorTransaction:
/// 1. Extract token address from transaction
/// 2. Get ALL pools for the token from cache
/// 3. Filter to V2 pools (V3/V4 not yet supported)
/// 4. FOR EACH POOL:
///    - Run transaction simulation
///    - Run buy/sell simulation for THIS pool
///    - Create pool-specific SimulationResult
///    - Send to signal manager
///    - Generate unique signal for (token, pool) pair
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::sync::Mutex as TokioMutex;
use tokio::time;
use tracing::warn;
use tx_processor::ProcessedTransaction;

/// Manager for transaction simulations
#[derive(Clone)]
pub struct SimulationManager {
    pub(super) mempool_simulator: Arc<MempoolSimulator>,
    pub(super) liquidity_removal_simulator: Arc<LiquidityRemovalSimulator>,
    pub(super) request_queue: RequestQueue,

    // Signal detection
    pub(super) signal_manager: Arc<Mutex<SignalManager>>,
    pub(super) token_cache: Arc<TokenTrackingCache>,

    // Pending processed transactions keyed by creator/token.
    pub(super) pending_sequences: PendingSequences,
    // Raw same-sender nonce dependencies used only to replay pending tx prefixes.
    pub(super) pending_nonce_dependencies: PendingNonceDependencies,
}

impl SimulationManager {
    /// Create new simulation manager with unified simulator
    pub fn new(
        mempool_simulator: Arc<MempoolSimulator>,
        token_cache: Arc<TokenTrackingCache>,
        signal_config: SignalManagerConfig,
        publisher: Arc<TokioMutex<crate::signal_publisher::SignalPublisher>>,
        max_concurrent: usize,
    ) -> Self {
        let mut signal_manager = SignalManager::new(signal_config);
        signal_manager.set_token_cache(token_cache.clone());
        signal_manager.set_publisher(publisher);

        // Create liquidity removal simulator with the same underlying simulator
        let mut liquidity_removal_simulator =
            LiquidityRemovalSimulator::new(mempool_simulator.get_tx_simulator());
        liquidity_removal_simulator.set_token_cache(token_cache.clone());

        let pending_sequences = PendingSequences::new();
        let pending_nonce_dependencies = PendingNonceDependencies::new();

        let tx_simulator_for_task = mempool_simulator.get_tx_simulator();
        let _ = spawn_block_pruner(tx_simulator_for_task, pending_sequences.clone());

        Self {
            mempool_simulator,
            liquidity_removal_simulator: Arc::new(liquidity_removal_simulator),
            request_queue: RequestQueue::new(max_concurrent),
            signal_manager: Arc::new(Mutex::new(signal_manager)),
            token_cache,
            pending_sequences,
            pending_nonce_dependencies,
        }
    }

    /// Submit a simulation request
    pub async fn submit(&self, request: TxSimulationJob) -> Result<(), String> {
        self.request_queue.submit(request).await
    }

    /// Schedule a follow-up buy/sell probe (fire-and-forget enqueue).
    pub fn schedule_buy_sell_follow_up(&self, request: TxSimulationJob) {
        let queue = self.request_queue.clone();
        let tx_hash = request.tx.hash.clone();
        tokio::spawn(async move {
            if let Err(err) = queue.submit(request).await {
                warn!(
                    "Failed to enqueue follow-up buy/sell job {}: {}",
                    tx_hash, err
                );
            }
        });
    }

    /// Handle LP approval detection without simulation
    pub async fn detect_lp_approval(
        &self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
        category: &crate::tx_router::TransactionCategory,
    ) -> bool {
        let mut signal_manager = self.signal_manager.lock().await;
        signal_manager.detect_lp_approval(tx, category).await
    }

    /// Process pending simulations
    pub async fn process_queue(&self) -> Vec<SimulationResult> {
        self.request_queue
            .process(|req| self.simulate_request(req))
            .await
    }

    pub async fn next_request(&self) -> Option<TxSimulationJob> {
        self.request_queue.pop_one().await
    }

    pub async fn simulate_with_timeout(
        &self,
        request: TxSimulationJob,
        timeout: Duration,
    ) -> SimulationResult {
        let start = Instant::now();
        let result = match time::timeout(timeout, self.simulate_request(request.clone())).await {
            Ok(result) => result,
            Err(_) => SimulationResult {
                request,
                pool_viability_result: None,
                error: Some(format!(
                    "simulation timed out after {}ms",
                    timeout.as_millis()
                )),
                simulation_time_ms: start.elapsed().as_secs_f64() * 1000.0,
                token_address: None,
                pool_address: None,
                pool_type: None,
                debug_info: None,
                liquidity_removal_result: None,
            },
        };
        self.request_queue.record_result(&result).await;
        result
    }

    pub async fn stats(&self) -> ManagerStats {
        self.request_queue.stats().await
    }

    pub async fn record_pending_nonce_dependency(
        &self,
        tx: &crate::mempool_fetcher::MempoolTransaction,
    ) {
        self.pending_nonce_dependencies
            .record(tx.clone(), Instant::now())
            .await;
    }

    pub async fn pending_nonce_dependency_stats(&self) -> PendingNonceDependencyStats {
        self.pending_nonce_dependencies.stats().await
    }

    /// Simulate a single request
    async fn simulate_request(&self, request: TxSimulationJob) -> SimulationResult {
        log_simulation_start(&request);

        let start = Instant::now();
        let now = Instant::now();
        self.prune_expired_sequences(now).await;

        let mut result = match &request.category {
            TransactionCategory::ContractCreation { .. } => {
                self.handle_contract_creation(&request, now).await
            }
            TransactionCategory::CreatorTransaction { .. } => {
                self.handle_creator_transaction(&request, now).await
            }
            _ => SimulationResult {
                request: request.clone(),
                pool_viability_result: None,
                error: Some("Transaction type not supported for simulation".to_string()),
                simulation_time_ms: 0.0,
                token_address: None,
                pool_address: None,
                pool_type: None,
                debug_info: None,
                liquidity_removal_result: None,
            },
        };

        result.simulation_time_ms = start.elapsed().as_secs_f64() * 1000.0;
        result
    }

    /// Simulate transaction followed by buy/sell sequence for ALL pools
    ///
    /// CRITICAL: This function processes EACH pool independently:
    /// - Each pool gets its own simulation
    /// - Each pool's results are collected separately
    /// - Returns a Vec with one result per pool
    /// - Failed pools don't affect successful ones
    pub(super) async fn build_processed_transaction(
        &self,
        request: &TxSimulationJob,
        retry_on_missing_header: bool,
    ) -> EyreResult<ProcessedTransaction> {
        self.build_processed_transaction_with_nonce_dependencies(request, retry_on_missing_header)
            .await
            .map(|processed| processed.transaction)
    }

    pub(super) async fn record_pending_transaction(
        &self,
        key: SequenceKey,
        source_hash: alloy_primitives::B256,
        processed: ProcessedTransaction,
        now: Instant,
    ) -> Vec<ProcessedTransaction> {
        self.pending_sequences
            .record_transaction(key, source_hash, processed, now)
            .await
    }

    async fn prune_expired_sequences(&self, now: Instant) {
        self.pending_sequences.prune_expired(now).await;
        self.pending_nonce_dependencies.prune_expired(now).await;
    }
}
