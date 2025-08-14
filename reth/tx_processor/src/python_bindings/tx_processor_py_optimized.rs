/// Optimized Python interface for TxProcessor with true parallel batch processing
/// 
/// This version fixes the mutex contention issue by creating separate processor
/// instances for each worker thread, enabling true parallelism.

use pyo3::prelude::*;
use alloy_primitives::{B256, Address};
use std::str::FromStr;
use std::sync::Arc;
use rayon::prelude::*;

use crate::TxProcessor;
use super::processed_transaction::PyProcessedTransaction;

/// Optimized Python wrapper for TxProcessor with true parallel processing
#[pyclass(name = "TxProcessorOptimized")]
pub struct PyTxProcessorOptimized {
    reth_datadir: String,  // Store path instead of processor
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyTxProcessorOptimized {
    /// Create new TxProcessor instance
    /// 
    /// No arguments needed - uses hardcoded Reth data directory
    #[new]
    fn new() -> PyResult<Self> {
        // Hardcoded reth_datadir
        let reth_datadir = "/home/nima/.local/share/reth/mainnet".to_string();
        
        // Create tokio runtime for async operations
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;
        
        // Verify we can create a processor (but don't keep it)
        TxProcessor::new(&reth_datadir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to verify TxProcessor creation: {}", e)
            ))?;
        
        Ok(Self {
            reth_datadir,
            runtime: Arc::new(runtime),
        })
    }
    
    /// Process a single transaction by hash (same as before)
    fn process_transaction(&self, _py: Python, tx_hash: String) -> PyResult<PyProcessedTransaction> {
        // Create a fresh processor for this request
        let processor = TxProcessor::new(&self.reth_datadir)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create TxProcessor: {}", e)
            ))?;
        
        // Parse transaction hash
        let tx_hash = tx_hash.trim_start_matches("0x");
        let hash = B256::from_str(tx_hash)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid transaction hash: {}", e)
            ))?;
        
        // Process transaction
        let result = self.runtime.block_on(async move {
            processor.process_transaction_by_hash(hash).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
            format!("Failed to process transaction: {}", e)
        ))?;
        
        Ok(PyProcessedTransaction::from_processed_transaction(result))
    }
    
    /// Optimized batch processing with TRUE parallelism
    /// 
    /// This version creates separate TxProcessor instances per thread to avoid
    /// mutex contention, enabling true parallel processing.
    fn process_transactions_batch_optimized(&self, py: Python, tx_hashes: Vec<String>) -> PyResult<Vec<PyProcessedTransaction>> {
        // Parse transaction hashes first (fail fast on invalid input)
        let hashes: Result<Vec<B256>, _> = tx_hashes
            .iter()
            .map(|tx_hash| {
                let tx_hash = tx_hash.trim_start_matches("0x");
                B256::from_str(tx_hash)
            })
            .collect();
        
        let hashes = hashes.map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid transaction hash: {}", e)
        ))?;
        
        let reth_datadir = self.reth_datadir.clone();
        
        // Release GIL for parallel processing
        py.allow_threads(|| {
            // Use all available cores for maximum performance
            let num_threads = num_cpus::get().min(32); // Cap at 32 to be reasonable
            
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .ok();
            
            // Process transactions in parallel with SEPARATE processors
            let results: Vec<_> = hashes
                .par_iter()
                .map(|&hash| {
                    // Each thread creates its OWN processor - no sharing, no locks!
                    let processor = match TxProcessor::new(&reth_datadir) {
                        Ok(p) => p,
                        Err(e) => return Err(e),
                    };
                    
                    // Create a dedicated runtime for this thread
                    let runtime = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => return Err(eyre::eyre!("Failed to create runtime: {}", e)),
                    };
                    
                    // Process the transaction
                    runtime.block_on(async move {
                        processor.process_transaction_by_hash(hash).await
                    })
                })
                .collect();
            
            // Convert successful results
            let mut processed = Vec::with_capacity(results.len());
            for (i, result) in results.into_iter().enumerate() {
                match result {
                    Ok(ptx) => processed.push(PyProcessedTransaction::from_processed_transaction(ptx)),
                    Err(e) => {
                        eprintln!("Error processing transaction at index {}: {}", i, e);
                        // Continue processing other transactions
                    }
                }
            }
            
            Ok(processed)
        })
    }
    
    /// Ultra-fast batch processing using chunked parallel processing
    /// 
    /// This version processes transactions in chunks, with each worker handling
    /// multiple transactions to amortize the cost of processor creation.
    fn process_transactions_batch_chunked(&self, py: Python, tx_hashes: Vec<String>, chunk_size: Option<usize>) -> PyResult<Vec<PyProcessedTransaction>> {
        // Parse transaction hashes
        let hashes: Result<Vec<B256>, _> = tx_hashes
            .iter()
            .map(|tx_hash| {
                let tx_hash = tx_hash.trim_start_matches("0x");
                B256::from_str(tx_hash)
            })
            .collect();
        
        let hashes = hashes.map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
            format!("Invalid transaction hash: {}", e)
        ))?;
        
        let reth_datadir = self.reth_datadir.clone();
        let chunk_size = chunk_size.unwrap_or(10); // Process 10 txs per worker by default
        
        // Release GIL for parallel processing
        py.allow_threads(|| {
            // Split work into chunks
            let chunks: Vec<_> = hashes.chunks(chunk_size).collect();
            
            // Process chunks in parallel
            let results: Vec<Vec<_>> = chunks
                .par_iter()
                .map(|chunk| {
                    // Create one processor per chunk
                    let processor = match TxProcessor::new(&reth_datadir) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("Failed to create processor: {}", e);
                            return vec![];
                        }
                    };
                    
                    let runtime = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            eprintln!("Failed to create runtime: {}", e);
                            return vec![];
                        }
                    };
                    
                    // Process all transactions in this chunk with the same processor
                    let mut chunk_results = Vec::new();
                    for &hash in chunk.iter() {
                        match runtime.block_on(async {
                            processor.process_transaction_by_hash(hash).await
                        }) {
                            Ok(ptx) => {
                                chunk_results.push(PyProcessedTransaction::from_processed_transaction(ptx));
                            }
                            Err(e) => {
                                eprintln!("Error processing transaction: {}", e);
                            }
                        }
                    }
                    chunk_results
                })
                .collect();
            
            // Flatten results
            Ok(results.into_iter().flatten().collect())
        })
    }
    
    /// Get processor statistics
    fn get_stats(&self, py: Python) -> PyResult<Py<pyo3::types::PyDict>> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("version", "0.1.0-optimized")?;
        dict.set_item("backend", "Rust tx_processor (optimized)")?;
        dict.set_item("performance", "True parallel processing")?;
        dict.set_item("cpu_count", num_cpus::get())?;
        dict.set_item("max_workers", num_cpus::get().min(32))?;
        Ok(dict.into())
    }
    
    fn __repr__(&self) -> String {
        format!("TxProcessorOptimized(backend='Rust', version='0.1.0-optimized', max_workers={})", 
                num_cpus::get().min(32))
    }
}