/// Batch Processor for Mempool Transactions
/// 
/// This module provides efficient batch simulation of mempool transactions
/// by converting NonBlockingTransaction to CallRequest and using parallel simulation.

use crate::mempool_fetcher::NonBlockingTransaction;
use reth_tx_simulator::{DirectTxSimulator, CallRequest, BatchSimulationOptions, BatchSimulationResult, AddressStateChange};
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tracing::{info, debug, error};

/// Batch processor for simulating multiple transactions
pub struct BatchProcessor {
    simulator: Arc<DirectTxSimulator>,
    default_options: BatchSimulationOptions,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(simulator: Arc<DirectTxSimulator>) -> Self {
        let default_options = BatchSimulationOptions {
            max_concurrent: 10,
            timeout_per_tx: Some(Duration::from_millis(50)), // 50ms per tx
            block_number: None, // Use latest
        };
        
        Self {
            simulator,
            default_options,
        }
    }
    
    /// Set custom batch options
    pub fn with_options(mut self, options: BatchSimulationOptions) -> Self {
        self.default_options = options;
        self
    }
    
    /// Convert NonBlockingTransaction to CallRequest using pre-parsed fields
    pub fn convert_to_call_request(tx: &NonBlockingTransaction) -> CallRequest {
        CallRequest {
            from: Some(Address::from_slice(&tx.from)),
            to: tx.to.as_ref().map(|addr| Address::from_slice(addr)),
            gas: tx.data.get("gas")
                .and_then(|v| v.as_str())
                .and_then(|s| s.strip_prefix("0x"))
                .and_then(|s| u64::from_str_radix(s, 16).ok()),
            gas_price: tx.gas_price.and_then(|gp| gp.try_into().ok()),
            max_fee_per_gas: tx.data.get("maxFeePerGas")
                .and_then(|v| v.as_str())
                .and_then(|s| s.strip_prefix("0x"))
                .and_then(|s| u128::from_str_radix(s, 16).ok()),
            max_priority_fee_per_gas: tx.data.get("maxPriorityFeePerGas")
                .and_then(|v| v.as_str())
                .and_then(|s| s.strip_prefix("0x"))
                .and_then(|s| u128::from_str_radix(s, 16).ok()),
            value: {
                // Convert ethers U256 to alloy U256
                let mut bytes = [0u8; 32];
                tx.value.to_big_endian(&mut bytes);
                Some(U256::from_be_bytes(bytes))
            },
            data: Some(Bytes::from(tx.input.clone())),
            nonce: tx.data.get("nonce")
                .and_then(|v| v.as_str())
                .and_then(|s| s.strip_prefix("0x"))
                .and_then(|s| u64::from_str_radix(s, 16).ok()),
        }
    }
    
    /// Simulate a batch of NonBlockingTransactions
    pub async fn simulate_batch(
        &self,
        transactions: Vec<NonBlockingTransaction>,
    ) -> Result<BatchSimulationResult> {
        let start = Instant::now();
        let total = transactions.len();
        
        info!("🚀 Starting batch simulation of {} transactions", total);
        
        // Convert to CallRequests with identifiers
        let requests: Vec<(String, CallRequest)> = transactions
            .into_iter()
            .map(|tx| {
                let hash = tx.hash.clone();
                let request = Self::convert_to_call_request(&tx);
                (hash, request)
            })
            .collect();
        
        // Use the unsigned batch simulation which handles nonce fixing
        let result = self.simulator
            .simulate_unsigned_batch(requests, self.default_options.clone())
            .await?;
        
        let elapsed = start.elapsed();
        info!("✅ Batch simulation complete in {:?}", elapsed);
        info!("   Success: {}, Failed: {}, Timed out: {}", 
              result.successful, result.failed, result.timed_out);
        
        Ok(result)
    }
    
    /// Simulate a filtered batch (only specific transactions)
    pub async fn simulate_filtered_batch(
        &self,
        transactions: Vec<NonBlockingTransaction>,
        filter: impl Fn(&NonBlockingTransaction) -> bool,
    ) -> Result<BatchSimulationResult> {
        // Filter transactions
        let filtered: Vec<NonBlockingTransaction> = transactions
            .into_iter()
            .filter(|tx| filter(tx))
            .collect();
        
        if filtered.is_empty() {
            return Ok(BatchSimulationResult {
                total: 0,
                successful: 0,
                failed: 0,
                timed_out: 0,
                results: Vec::new(),
                duration: Duration::from_secs(0),
                avg_time_per_tx: Duration::from_secs(0),
            });
        }
        
        debug!("Filtered {} transactions for simulation", filtered.len());
        self.simulate_batch(filtered).await
    }
    
    /// Simulate transactions that match detected functions
    pub async fn simulate_with_function_filter(
        &self,
        transactions: Vec<NonBlockingTransaction>,
        detected_functions: &std::collections::HashMap<String, String>,
    ) -> Result<BatchSimulationResult> {
        // Filter only transactions with detected functions
        let filtered: Vec<NonBlockingTransaction> = transactions
            .into_iter()
            .filter(|tx| detected_functions.contains_key(&tx.hash))
            .collect();
        
        if filtered.is_empty() {
            info!("No transactions with interesting functions to simulate");
            return Ok(BatchSimulationResult {
                total: 0,
                successful: 0,
                failed: 0,
                timed_out: 0,
                results: Vec::new(),
                duration: Duration::from_secs(0),
                avg_time_per_tx: Duration::from_secs(0),
            });
        }
        
        info!("Simulating {} transactions with detected functions", filtered.len());
        for tx in &filtered {
            if let Some(function) = detected_functions.get(&tx.hash) {
                debug!("  {} -> {}", tx.hash, function);
            }
        }
        
        self.simulate_batch(filtered).await
    }
    
    /// Simulate transactions with state changes (with optional function filter)
    pub async fn simulate_batch_with_state_changes(
        &self,
        transactions: Vec<NonBlockingTransaction>,
    ) -> Result<Vec<(String, Result<HashMap<Address, AddressStateChange>>)>> {
        self.simulate_batch_with_state_changes_filtered(transactions, None).await
    }
    
    /// Simulate transactions with state changes, optionally filtered by detected functions
    pub async fn simulate_batch_with_state_changes_filtered(
        &self,
        transactions: Vec<NonBlockingTransaction>,
        detected_functions: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<Vec<(String, Result<HashMap<Address, AddressStateChange>>)>> {
        // Filter transactions if function detection provided
        let filtered_txs = if let Some(functions) = detected_functions {
            transactions.into_iter()
                .filter(|tx| functions.contains_key(&tx.hash))
                .collect::<Vec<_>>()
        } else {
            transactions
        };
        
        let total = filtered_txs.len();
        if total == 0 {
            info!("No transactions to simulate after filtering");
            return Ok(Vec::new());
        }
        
        info!("🚀 Starting batch simulation with state changes for {} transactions", total);
        
        // Convert to CallRequests
        let requests: Vec<(String, CallRequest)> = filtered_txs
            .into_iter()
            .map(|tx| {
                let hash = tx.hash.clone();
                let request = Self::convert_to_call_request(&tx);
                (hash, request)
            })
            .collect();
        
        // Get block number
        let block_number = match self.default_options.block_number {
            Some(n) => n,
            None => self.simulator.get_latest_block()?,
        };
        
        // Create semaphore for concurrency control
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.default_options.max_concurrent));
        
        // Process requests concurrently
        let futures = requests.into_iter().map(|(hash, request)| {
            let sem = semaphore.clone();
            let sim = self.simulator.clone();
            let timeout_duration = self.default_options.timeout_per_tx;
            
            async move {
                // Acquire permit for concurrency control
                let _permit = match sem.acquire().await {
                    Ok(permit) => permit,
                    Err(e) => {
                        error!("Failed to acquire semaphore permit: {}", e);
                        return (hash, Err(eyre::eyre!("Semaphore error: {}", e)));
                    }
                };
                
                // Simulate with optional timeout
                let result = match timeout_duration {
                    Some(duration) => {
                        match tokio::time::timeout(
                            duration,
                            sim.simulate_unsigned_transaction_with_call_trace_at_block(request, block_number)
                        ).await {
                            Ok(Ok(res)) => Ok(res),
                            Ok(Err(e)) => Err(e),
                            Err(_) => Err(eyre::eyre!("Simulation timed out after {:?}", duration)),
                        }
                    }
                    None => sim.simulate_unsigned_transaction_with_call_trace_at_block(request, block_number).await,
                };
                
                (hash, result)
            }
        });
        
        // Collect all results
        let results = futures::future::join_all(futures).await;
        
        // Log summary
        let successful = results.iter().filter(|(_, r)| r.is_ok()).count();
        let failed = results.len() - successful;
        info!("✅ State change simulation complete: {} successful, {} failed", successful, failed);
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_convert_to_call_request() {
        // Create a mock NonBlockingTransaction
        let mut data = serde_json::Map::new();
        data.insert("gas".to_string(), serde_json::Value::String("0x5208".to_string()));
        data.insert("nonce".to_string(), serde_json::Value::String("0x1".to_string()));
        
        let tx = NonBlockingTransaction {
            hash: "0xabc123".to_string(),
            data: serde_json::Value::Object(data),
            detection_ns: 1000,
            from: vec![0x11; 20],
            to: Some(vec![0x22; 20]),
            input: vec![0x12, 0x34, 0x56, 0x78],
            value: U256::from(1000u64),
            gas_price: Some(U256::from(20_000_000_000u64)),
        };
        
        let request = BatchProcessor::convert_to_call_request(&tx);
        
        assert_eq!(request.from, Some(Address::from_slice(&[0x11; 20])));
        assert_eq!(request.to, Some(Address::from_slice(&[0x22; 20])));
        assert_eq!(request.gas, Some(21000)); // 0x5208
        assert_eq!(request.nonce, Some(1));
        assert_eq!(request.value, Some(U256::from(1000u64)));
        assert_eq!(request.gas_price, Some(20_000_000_000));
    }
}