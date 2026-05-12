use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use mempool_processor::{
    function_detector::CreatorFunctionType,
    simulator::{
        MempoolSimulator, SimulationManager, SimulationResult, SimulationType, TxSimulationJob,
    },
    tx_router::{TransactionCategory, TransactionRouter},
    unresolved_intents::{UnresolvedIntentKind, UnresolvedIntentStore},
};
use tokio::sync::mpsc;
use tracing::warn;

use super::metrics::ServiceMetrics;
use super::support::parse_tx_hash_or_zero;

pub(crate) async fn drain_simulation_results(
    receiver: &mut mpsc::Receiver<SimulationResult>,
    metrics: &ServiceMetrics,
    mempool_simulator: &MempoolSimulator,
    simulation_error_log_path: &Path,
    unresolved_intent_store: &UnresolvedIntentStore,
) -> usize {
    let mut drained = 0usize;
    while let Ok(result) = receiver.try_recv() {
        drained += 1;
        let tx_hash = format!("{:?}", result.request.tx_hash);
        let category = match &result.request.category {
            TransactionCategory::ContractCreation { .. } => "ContractCreation",
            TransactionCategory::CreatorTransaction { .. } => "CreatorTransaction",
            _ => "Other",
        };

        if let Some(ref error) = result.error {
            if is_unresolved_cache_error(error) {
                unresolved_intent_store
                    .record(
                        result.request.tx.clone(),
                        unresolved_kind_for_result(&result),
                        error.clone(),
                    )
                    .await;
                continue;
            }

            if is_replay_context_mismatch(error) {
                unresolved_intent_store
                    .resolve(&result.request.tx.hash)
                    .await;
                warn!(
                    "Replay context mismatch for {} classified outside simulation error path: {}",
                    result.request.tx.hash, error
                );
                continue;
            }

            if is_stale_pending_tx_error(error) {
                unresolved_intent_store
                    .resolve(&result.request.tx.hash)
                    .await;
                warn!(
                    "Stale pending tx for {} classified outside simulation error path: {}",
                    result.request.tx.hash, error
                );
                continue;
            }

            metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
            if !error.contains("No pools found for token") {
                write_simulation_error(
                    mempool_simulator,
                    simulation_error_log_path,
                    &tx_hash,
                    category,
                    error,
                )
                .await;
            }
            unresolved_intent_store
                .resolve(&result.request.tx.hash)
                .await;
        } else {
            unresolved_intent_store
                .resolve(&result.request.tx.hash)
                .await;
            metrics
                .simulations_completed
                .fetch_add(1, Ordering::Relaxed);
            if result.simulation_time_ms > 0.0 {
                let sim_duration = Duration::from_secs_f64(result.simulation_time_ms / 1000.0);
                metrics.add_simulation_time(sim_duration).await;
            }
        }
    }
    drained
}

pub(crate) async fn retry_unresolved_intents(
    unresolved_intent_store: &UnresolvedIntentStore,
    tx_router: &TransactionRouter,
    simulation_manager: &SimulationManager,
    metrics: &ServiceMetrics,
) {
    let intents = unresolved_intent_store.take_ready_for_retry().await;
    for intent in intents {
        let classification = tx_router.classify(&intent.tx).await;
        if let Some((_kind, reason)) = tx_router.unresolved_intent_for(&intent.tx, &classification)
        {
            unresolved_intent_store
                .mark_pending(&intent.tx.hash, reason)
                .await;
            continue;
        }

        match &classification.category {
            TransactionCategory::ContractCreation { .. }
            | TransactionCategory::CreatorTransaction { .. } => {}
            _ => {
                unresolved_intent_store
                    .mark_pending(
                        &intent.tx.hash,
                        "classification still lacks token/pool mapping",
                    )
                    .await;
                continue;
            }
        }

        if !classification.requires_simulation {
            retry_unresolved_no_simulation(
                unresolved_intent_store,
                simulation_manager,
                &intent.tx.hash,
                &intent.tx,
                &classification.category,
            )
            .await;
            continue;
        }

        let sim_request = TxSimulationJob {
            tx: intent.tx.clone(),
            category: classification.category.clone(),
            priority: classification.priority,
            simulation_type: match &classification.category {
                TransactionCategory::ContractCreation { .. }
                | TransactionCategory::CreatorTransaction { .. } => {
                    SimulationType::TransactionWithBuySell
                }
                _ => SimulationType::TransactionOnly,
            },
            tx_hash: parse_tx_hash_or_zero(&intent.tx.hash),
        };

        match simulation_manager.submit(sim_request).await {
            Ok(()) => {
                metrics
                    .simulations_submitted
                    .fetch_add(1, Ordering::Relaxed);
            }
            Err(err) => {
                metrics.simulation_errors.fetch_add(1, Ordering::Relaxed);
                unresolved_intent_store
                    .mark_pending(
                        &intent.tx.hash,
                        format!("simulation queue rejected unresolved intent: {}", err),
                    )
                    .await;
            }
        }
    }
}

async fn retry_unresolved_no_simulation(
    unresolved_intent_store: &UnresolvedIntentStore,
    simulation_manager: &SimulationManager,
    tx_hash: &str,
    tx: &mempool_processor::mempool_fetcher::MempoolTransaction,
    category: &TransactionCategory,
) {
    if let TransactionCategory::CreatorTransaction {
        function_type: CreatorFunctionType::LiquidityPoolApproval,
        ..
    } = category
    {
        let published = simulation_manager.detect_lp_approval(tx, category).await;
        if published {
            unresolved_intent_store.resolve(tx_hash).await;
        } else {
            unresolved_intent_store
                .mark_pending(
                    tx_hash,
                    "LP approval enrichment still lacks token/pool mapping",
                )
                .await;
        }
    } else {
        unresolved_intent_store.resolve(tx_hash).await;
    }
}

async fn write_simulation_error(
    mempool_simulator: &MempoolSimulator,
    simulation_error_log_path: &Path,
    tx_hash: &str,
    category: &str,
    error: &str,
) {
    let block_str = match mempool_simulator.latest_simulation_block().await {
        Ok(b) => b.to_string(),
        Err(_) => "unknown".to_string(),
    };
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(simulation_error_log_path)
    {
        let timestamp = chrono::Local::now();
        writeln!(
            file,
            "[{}] SIMULATION_ERROR | block={} | tx={} | category={} | error={}",
            timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            block_str,
            tx_hash,
            category,
            error
        )
        .ok();
    }
}

fn is_unresolved_cache_error(error: &str) -> bool {
    error.contains("unresolved_cache_context")
        || error.contains("No tracked token found for liquidity removal")
        || error.contains("No token address found for creator")
        || error.contains("No pools found for token")
        || error.contains("Token cache reported no pools")
}

fn is_replay_context_mismatch(error: &str) -> bool {
    error.contains("Setup transaction replay failed") && error.contains("mined receipt succeeded")
}

fn is_stale_pending_tx_error(error: &str) -> bool {
    error.contains("transaction validation error: nonce")
        && error.contains("too low")
        && error.contains("expected")
}

fn unresolved_kind_for_result(result: &SimulationResult) -> UnresolvedIntentKind {
    if is_v4_modify_liquidity_selector(&result.request.tx.input) {
        return UnresolvedIntentKind::V4ModifyLiquidity;
    }

    match &result.request.category {
        TransactionCategory::CreatorTransaction { function_type, .. } => match function_type {
            CreatorFunctionType::LiquidityPoolApproval => UnresolvedIntentKind::LpApproval,
            CreatorFunctionType::LiquidityRemoval => UnresolvedIntentKind::LiquidityRemoval,
            _ => UnresolvedIntentKind::CreatorControl,
        },
        _ => UnresolvedIntentKind::CreatorControl,
    }
}

fn is_v4_modify_liquidity_selector(input: &[u8]) -> bool {
    let Some(selector) = input.get(0..4) else {
        return false;
    };
    matches!(
        selector,
        [0xdd, 0x46, 0x50, 0x8f] | [0xa3, 0x55, 0xde, 0x88] | [0x0d, 0x4f, 0x31, 0x9d]
    )
}
