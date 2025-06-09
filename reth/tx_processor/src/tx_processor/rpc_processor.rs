use std::time::Instant;
use eyre::Result;

use ethers_providers::{Provider, Http, Middleware};
use ethers_core::types::{H256, Trace, TraceType};

use super::types::{ProcessedTransaction, InternalTransfer, ProcessingMetrics};
use crate::conversions::{ethers_to_revm_address, ethers_to_revm_u256, h256_to_b256};

pub struct RpcProcessorConfig {
    pub rpc_url: String,
    pub enable_traces: bool,
}

impl Default for RpcProcessorConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            enable_traces: true,
        }
    }
}

/// RPC-based processor for comparison
pub struct RpcProcessor {
    provider: Provider<Http>,
    config: RpcProcessorConfig,
}

impl RpcProcessor {
    pub async fn new(config: RpcProcessorConfig) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)?;
        Ok(Self { provider, config })
    }

    /// Process transaction using RPC calls (for benchmarking comparison)
    pub async fn process_transaction(&self, tx_hash: H256) -> Result<ProcessedTransaction> {
        let total_start = Instant::now();
        
        // Get transaction
        let tx = self.provider.get_transaction(tx_hash).await?
            .ok_or_else(|| eyre::eyre!("Transaction not found"))?;
        
        // Get receipt
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| eyre::eyre!("Receipt not found"))?;
        
        // Get traces if enabled
        let mut internal_transfers = vec![];
        if self.config.enable_traces {
            match self.provider.trace_transaction(tx_hash).await {
                Ok(traces) => {
                    for trace in traces {
                        if let Some(transfer) = extract_internal_transfer(&trace) {
                            internal_transfers.push(transfer);
                        }
                    }
                }
                Err(_) => {
                    // Try debug_traceTransaction as fallback
                    // This would need custom deserialization
                }
            }
        }
        
        let total_time = total_start.elapsed().as_secs_f64() * 1000.0;
        
        Ok(ProcessedTransaction {
            hash: h256_to_b256(tx_hash),
            block_number: receipt.block_number.unwrap().as_u64(),
            from: ethers_to_revm_address(tx.from),
            to: tx.to.map(ethers_to_revm_address),
            value: ethers_to_revm_u256(tx.value),
            gas_used: receipt.gas_used.unwrap().as_u64(),
            success: receipt.status.unwrap().as_u64() == 1,
            internal_transfers,
            state_changes: Default::default(),
            logs: vec![],
            metrics: ProcessingMetrics {
                fetch_time_ms: total_time,
                simulation_time_ms: 0.0,
                total_time_ms: total_time,
            },
        })
    }
}

fn extract_internal_transfer(trace: &Trace) -> Option<InternalTransfer> {
    // Extract internal ETH transfers from traces
    // This is a simplified version
    None // TODO: Implement trace parsing
}