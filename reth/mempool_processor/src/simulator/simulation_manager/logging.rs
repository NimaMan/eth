use crate::{mempool_fetcher::MempoolTransaction, tx_router::TransactionCategory};
use hex;
use tracing::info;

use super::types::TxSimulationJob;

pub fn format_contract_creation_context(request: &TxSimulationJob) -> String {
    let tx_hash = request.tx.hash.as_str();
    let from_addr = format_from_field(&request.tx);
    let creator = match &request.category {
        TransactionCategory::ContractCreation { deployer, .. } => deployer.as_str(),
        _ => from_addr.as_str(),
    };
    format!(
        "tx_hash={}, from={}, creator={}",
        tx_hash, from_addr, creator
    )
}

pub fn log_simulation_start(request: &TxSimulationJob) {
    info!(
        "=== SIMULATION MANAGER: Starting simulation for TX {} ===",
        request.tx.hash
    );
    info!("  Category: {:?}", request.category);
    info!("  Priority: {:?}", request.priority);
    info!("  Simulation Type: {:?}", request.simulation_type);
}

pub fn log_signal_dispatch_start(tx_hash: &str, pool_idx: Option<usize>) {
    match pool_idx {
        Some(idx) => info!(
            "📤 Sending pool-specific result to signal manager for TX {} (pool {})",
            tx_hash, idx
        ),
        None => info!(
            "📤 Sending simulation result to signal manager for TX {}",
            tx_hash
        ),
    }
}

pub fn log_signal_dispatch_result(signal_count: usize, pool_idx: Option<usize>) {
    match pool_idx {
        Some(idx) => info!(
            "  Signal manager returned {} signals for pool {}",
            signal_count, idx
        ),
        None => info!("  Signal manager returned {} signals", signal_count),
    }
}

fn format_from_field(tx: &MempoolTransaction) -> String {
    if tx.from.len() == 20 {
        format!("0x{}", hex::encode(&tx.from))
    } else {
        tx.data
            .get("from")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    }
}
