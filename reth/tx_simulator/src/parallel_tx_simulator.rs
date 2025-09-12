/// Batch simulation support for processing multiple transactions efficiently
/// 
/// This module provides functionality to simulate multiple transactions in parallel
/// with controlled concurrency and timeout support.

use crate::{
    simulator::TxSimulator,
    types::ParallelTxSimulationResult,
    unsigned_tx_simulator::UnsignedTransaction,
};
use alloy_primitives::Address;
use std::collections::HashMap;
use eyre::Result;
use reth_primitives::TransactionSigned;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use futures::future::join_all;

/// Options for parallel transaction simulation
#[derive(Debug, Clone)]
pub struct ParallelTxSimulationOptions {
    /// Maximum number of concurrent simulations (default: 10)
    pub max_concurrent: usize,
    /// Timeout per transaction (default: 100ms)
    pub timeout_per_tx: Option<Duration>,
    /// Block number to simulate at (default: latest)
    pub block_number: Option<u64>,
}

impl Default for ParallelTxSimulationOptions {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            timeout_per_tx: Some(Duration::from_millis(100)),
            block_number: None,
        }
    }
}

impl TxSimulator {
    /// Simulate a batch of signed transactions in parallel
    /// 
    /// This method processes multiple transactions concurrently with controlled parallelism
    /// and optional timeout support. It's ideal for high-throughput scenarios like
    /// mempool monitoring.
    /// 
    /// NOTE: For signed transactions with nonce errors, consider using simulate_signed_transaction_with_nonce_check
    /// which provides helpful error messages about the expected nonce.
    /// 
    /// # Arguments
    /// * `transactions` - Vector of (tx_hash, transaction) pairs
    /// * `options` - Batch simulation options (concurrency, timeout, block)
    /// 
    /// # Performance
    /// - Processes transactions in parallel up to max_concurrent limit
    /// - Each transaction runs in its own blocking thread with timeout support
    /// - Results are collected and returned with statistics
    pub async fn simulate_signed_tx_list_parallel(
        &self,
        transactions: Vec<(String, TransactionSigned)>,
        options: ParallelTxSimulationOptions,
    ) -> Result<ParallelTxSimulationResult> {
        let start = Instant::now();
        let total = transactions.len();
        
        // Get block number for simulation
        let block_number = match options.block_number {
            Some(n) => n,
            None => self.get_latest_block()?,
        };
        
        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(options.max_concurrent));
        
        // Process transactions concurrently
        let futures = transactions.into_iter().map(|(hash, tx)| {
            let sem = semaphore.clone();
            let sim = self.clone();
            let timeout_duration = options.timeout_per_tx;
            
            async move {
                // Acquire permit for concurrency control
                let _permit = sem.acquire().await.unwrap();
                
                // Simulate with optional timeout
                let result = match timeout_duration {
                    Some(duration) => {
                        match tokio::time::timeout(
                            duration,
                            sim.simulate_signed_transaction_at_block(&tx, block_number)
                        ).await {
                            Ok(Ok(res)) => Ok(res),
                            Ok(Err(e)) => Err(e),
                            Err(_) => Err(eyre::eyre!("Simulation timed out after {:?}", duration)),
                        }
                    }
                    None => sim.simulate_signed_transaction_at_block(&tx, block_number).await,
                };
                
                (hash, result)
            }
        });
        
        // Collect all results
        let results = join_all(futures).await;
        
        // Count statistics
        let mut successful = 0;
        let mut failed = 0;
        let mut timed_out = 0;
        
        for (_, result) in &results {
            match result {
                Ok(_) => successful += 1,
                Err(e) if e.to_string().contains("timed out") => timed_out += 1,
                Err(_) => failed += 1,
            }
        }
        
        let duration = start.elapsed();
        let avg_time_per_tx = duration / total as u32;
        
        Ok(ParallelTxSimulationResult {
            total,
            successful,
            failed,
            timed_out,
            results,
            duration,
            avg_time_per_tx,
        })
    }
    
    
    
    
    /// Simulate a batch of unsigned transactions
    /// 
    /// Automatically adapts nonce if "nonce too low" errors are encountered.
    /// This is ideal for simulating mempool transactions where nonces might be outdated.
    /// 
    /// # Arguments
    /// * `requests` - Vector of (identifier, UnsignedTransaction) pairs
    /// * `options` - Batch simulation options (concurrency, timeout, block)
    /// 
    /// # Returns
    /// Results with nonce adaptation applied where needed
    pub async fn simulate_unsigned_tx_list_parallel(
        &self,
        requests: Vec<(String, UnsignedTransaction)>,
        options: ParallelTxSimulationOptions,
    ) -> Result<ParallelTxSimulationResult> {
        let start = Instant::now();
        let total = requests.len();
        
        // Get block number for simulation
        let block_number = match options.block_number {
            Some(n) => n,
            None => self.get_latest_block()?,
        };
        
        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(options.max_concurrent));
        
        // Process requests concurrently
        let futures = requests.into_iter().map(|(id, request)| {
            let sem = semaphore.clone();
            let sim = self.clone();
            let timeout_duration = options.timeout_per_tx;
            
            async move {
                // Acquire permit for concurrency control
                let _permit = sem.acquire().await.unwrap();
                
                // Simulate with optional timeout at the chosen block and nonce fixing
                let result = match timeout_duration {
                    Some(duration) => {
                        match tokio::time::timeout(
                            duration,
                            sim.simulate_unsigned_transaction_at_block(request, block_number)
                        ).await {
                            Ok(Ok(res)) => Ok(res),
                            Ok(Err(e)) => Err(e),
                            Err(_) => Err(eyre::eyre!("Simulation timed out after {:?}", duration)),
                        }
                    }
                    None => sim.simulate_unsigned_transaction_at_block(request, block_number).await,
                };
                
                (id, result)
            }
        });
        
        // Collect all results
        let results = join_all(futures).await;
        
        // Count statistics
        let mut successful = 0;
        let mut failed = 0;
        let mut timed_out = 0;
        
        for (_, result) in &results {
            match result {
                Ok(_) => successful += 1,
                Err(e) if e.to_string().contains("timed out") => timed_out += 1,
                Err(_) => failed += 1,
            }
        }
        
        let duration = start.elapsed();
        let avg_time_per_tx = duration / total as u32;
        
        Ok(ParallelTxSimulationResult {
            total,
            successful,
            failed,
            timed_out,
            results,
            duration,
            avg_time_per_tx,
        })
    }
}
