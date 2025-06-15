use super::{
    TransactionFetcher, PythonBridge, ComparisonEngine,
    transaction_fetcher::{TransactionCandidate, FetchConfig},
    python_bridge::{PythonBridgeConfig, PythonResult},
    comparison_engine::{TransactionComparison, ComparisonConfig},
};
// TODO: Update to use DebugTraceCallStateDiffCalculator instead
// use crate::tx_simulator::{comprehensive_state_diff::ComprehensiveStateDiffCalculator, ComprehensiveStateChange};
use crate::mempool_fetcher::types::{TransactionView};

// Temporary placeholder until we update to use DebugTraceCallStateDiffCalculator
type ComprehensiveStateChange = std::collections::HashMap<String, serde_json::Value>;
use ethers::prelude::*;
use eyre::Result;
use std::sync::Arc;
use tracing::{info, warn, error};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Configuration for batch validation
#[derive(Debug, Clone)]
pub struct BatchValidationConfig {
    pub fetch_config: FetchConfig,
    pub python_config: PythonBridgeConfig,
    pub comparison_config: ComparisonConfig,
    pub max_concurrent_transactions: usize,
    pub save_results: bool,
    pub results_dir: String,
    pub max_transactions: usize,
    pub timeout_seconds: u64,
    pub logs_dir: String,
}

impl Default for BatchValidationConfig {
    fn default() -> Self {
        Self {
            fetch_config: FetchConfig::default(),
            python_config: PythonBridgeConfig::default(),
            comparison_config: ComparisonConfig::default(),
            max_concurrent_transactions: 5,
            save_results: true,
            results_dir: "validation_results".to_string(),
            max_transactions: 10,
            timeout_seconds: 300,
            logs_dir: "/home/nima/code/crypto/logs/mempool".to_string(),
        }
    }
}

/// Summary of batch validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchValidationSummary {
    pub total_transactions: usize,
    pub successful_comparisons: usize,
    pub failed_rust: usize,
    pub failed_python: usize,
    pub exact_matches: usize,
    pub within_tolerance: usize,
    pub significant_differences: usize,
    pub overall_success_rate: f64,
    pub average_rust_time_ms: f64,
    pub average_python_time_ms: f64,
    pub transaction_types: HashMap<String, usize>,
}

/// Complete batch validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchValidationResult {
    pub summary: BatchValidationSummary,
    pub transaction_comparisons: Vec<TransactionComparison>,
    pub validation_timestamp: String,
    pub config_summary: String,
}

/// Detailed transaction log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLogEntry {
    pub transaction_hash: String,
    pub block_number: u64,
    pub timestamp: String,
    pub rust_success: bool,
    pub python_success: bool,
    pub rust_changes: HashMap<String, ComprehensiveStateChange>,
    pub python_result: PythonResult,
    pub comparison: TransactionComparison,
    pub rust_processing_time_ms: u64,
    pub python_processing_time_ms: u64,
}

/// Batch validator for comprehensive testing
pub struct BatchValidator {
    config: BatchValidationConfig,
    fetcher: TransactionFetcher,
    python_bridge: PythonBridge,
    comparison_engine: ComparisonEngine,
    rust_calculator: ComprehensiveStateDiffCalculator,
    provider: Arc<Provider<Http>>,
}

impl BatchValidator {
    pub fn new(
        provider: Arc<Provider<Http>>,
        config: BatchValidationConfig,
    ) -> Self {
        let fetcher = TransactionFetcher::new(provider.clone(), config.fetch_config.clone());
        let python_bridge = PythonBridge::new(config.python_config.clone());
        let comparison_engine = ComparisonEngine::new(config.comparison_config.clone());
        let rust_calculator = ComprehensiveStateDiffCalculator::new(provider.clone());

        Self {
            config,
            fetcher,
            python_bridge,
            comparison_engine,
            rust_calculator,
            provider,
        }
    }

    /// Run comprehensive batch validation
    pub async fn run_validation(&mut self) -> Result<BatchValidationResult> {
        info!("Starting batch validation...");

        // Test Python environment first
        self.python_bridge.test_environment().await?;

        // Fetch test transactions based on max_transactions setting
        let candidates = if self.config.max_transactions <= 20 {
            // For small tests, use diverse set
            self.fetcher.fetch_diverse_test_set().await?
        } else {
            // For large tests, use all available transactions up to limit
            let all_candidates = self.fetcher.fetch_test_transactions().await?;
            let limited_candidates = all_candidates.into_iter()
                .take(self.config.max_transactions)
                .collect::<Vec<_>>();
            info!("Limited to {} transactions for large-scale test", limited_candidates.len());
            limited_candidates
        };
        
        info!("Fetched {} transaction candidates for validation", candidates.len());

        // Process transactions
        let mut transaction_comparisons = Vec::new();
        let mut transaction_types = HashMap::new();

        for (i, candidate) in candidates.iter().enumerate() {
            info!("Processing transaction {}/{}: {}", 
                  i + 1, candidates.len(), candidate.hash);

            // Count transaction types
            *transaction_types.entry(candidate.transaction_type.clone()).or_insert(0) += 1;

            match self.process_single_transaction(candidate).await {
                Ok(comparison) => {
                    transaction_comparisons.push(comparison);
                }
                Err(e) => {
                    error!("Failed to process transaction {}: {}", candidate.hash, e);
                    // Create a failed comparison record
                    let failed_comparison = TransactionComparison {
                        transaction_hash: candidate.hash.clone(),
                        block_number: candidate.block_number,
                        rust_success: false,
                        python_success: false,
                        address_comparisons: Vec::new(),
                        summary: super::comparison_engine::ComparisonSummary {
                            total_addresses: 0,
                            exact_matches: 0,
                            within_tolerance: 0,
                            significant_differences: 0,
                            only_in_rust: 0,
                            only_in_python: 0,
                            overall_match: false,
                        },
                        rust_processing_time_ms: 0,
                        python_processing_time_ms: 0,
                    };
                    transaction_comparisons.push(failed_comparison);
                }
            }
        }

        // Generate summary
        let summary = self.generate_batch_summary(&transaction_comparisons, &transaction_types);

        let result = BatchValidationResult {
            summary,
            transaction_comparisons,
            validation_timestamp: chrono::Utc::now().to_rfc3339(),
            config_summary: format!("{:?}", self.config),
        };

        // Save results if configured
        if self.config.save_results {
            self.save_validation_results(&result).await?;
        }

        info!("Batch validation completed: {}/{} successful comparisons", 
              result.summary.successful_comparisons, result.summary.total_transactions);

        Ok(result)
    }

    /// Process a single transaction through both Rust and Python
    async fn process_single_transaction(
        &mut self,
        candidate: &TransactionCandidate,
    ) -> Result<TransactionComparison> {
        let tx_hash = &candidate.hash;
        let block_number = candidate.block_number;

        // Process through Python first to get baseline
        let python_result = self.python_bridge
            .process_transaction(tx_hash, block_number)
            .await?;

        if !python_result.success {
            return Err(eyre::eyre!("Python processing failed: {}", 
                                  python_result.error_message.unwrap_or_default()));
        }

        // Process through Rust using our ComprehensiveStateDiffCalculator
        let rust_start = std::time::Instant::now();
        
        // Parse transaction hash
        let tx_hash_h256 = tx_hash.parse::<H256>()
            .map_err(|e| eyre::eyre!("Failed to parse transaction hash: {}", e))?;
        
        // Fetch the transaction to get TransactionView
        let tx_option = self.provider.get_transaction(tx_hash_h256).await
            .map_err(|e| eyre::eyre!("Failed to fetch transaction: {}", e))?;
        
        let rust_changes = match tx_option {
            Some(tx) => {
                // Convert ethers Transaction to our TransactionView
                let tx_view = TransactionView {
                    hash: tx.hash.as_bytes().to_vec(),
                    from: tx.from.as_bytes().to_vec(),
                    to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                    value: tx.value,
                    gas_price: tx.gas_price,
                    gas_limit: Some(tx.gas),
                    nonce: Some(tx.nonce),
                    input_data: Some(tx.input.to_vec()),
                };
                
                // Get transaction index from the block
                let tx_receipt = self.provider.get_transaction_receipt(tx_hash_h256).await
                    .map_err(|e| eyre::eyre!("Failed to fetch transaction receipt: {}", e))?;
                
                let txn_index = match tx_receipt {
                    Some(receipt) => receipt.transaction_index.as_u64(),
                    None => return Err(eyre::eyre!("Transaction receipt not found")),
                };
                
                match self.rust_calculator.calculate_state_changes(&tx_view, block_number, txn_index).await {
                    Ok(changes) => changes,
                    Err(e) => {
                        warn!("Rust state calculation failed for {}: {}", tx_hash, e);
                        HashMap::new()
                    }
                }
            },
            None => {
                warn!("Transaction {} not found", tx_hash);
                HashMap::new()
            }
        };
        
        let rust_time_ms = rust_start.elapsed().as_millis() as u64;

        // Compare results
        let comparison = self.comparison_engine.compare_transaction(
            &rust_changes,
            &python_result,
            rust_time_ms,
        );

        Ok(comparison)
    }

    /// Generate batch summary statistics
    fn generate_batch_summary(
        &self,
        comparisons: &[TransactionComparison],
        transaction_types: &HashMap<String, usize>,
    ) -> BatchValidationSummary {
        let total_transactions = comparisons.len();
        let mut successful_comparisons = 0;
        let mut failed_rust = 0;
        let mut failed_python = 0;
        let mut exact_matches = 0;
        let mut within_tolerance = 0;
        let mut significant_differences = 0;
        let mut total_rust_time = 0u64;
        let mut total_python_time = 0u64;

        for comparison in comparisons {
            if comparison.rust_success && comparison.python_success {
                successful_comparisons += 1;
                
                if comparison.summary.overall_match {
                    exact_matches += 1;
                } else if comparison.summary.significant_differences == 0 {
                    within_tolerance += 1;
                } else {
                    significant_differences += 1;
                }
            } else {
                if !comparison.rust_success {
                    failed_rust += 1;
                }
                if !comparison.python_success {
                    failed_python += 1;
                }
            }

            total_rust_time += comparison.rust_processing_time_ms;
            total_python_time += comparison.python_processing_time_ms;
        }

        let overall_success_rate = if total_transactions > 0 {
            successful_comparisons as f64 / total_transactions as f64
        } else {
            0.0
        };

        let average_rust_time_ms = if total_transactions > 0 {
            total_rust_time as f64 / total_transactions as f64
        } else {
            0.0
        };

        let average_python_time_ms = if total_transactions > 0 {
            total_python_time as f64 / total_transactions as f64
        } else {
            0.0
        };

        BatchValidationSummary {
            total_transactions,
            successful_comparisons,
            failed_rust,
            failed_python,
            exact_matches,
            within_tolerance,
            significant_differences,
            overall_success_rate,
            average_rust_time_ms,
            average_python_time_ms,
            transaction_types: transaction_types.clone(),
        }
    }

    /// Save validation results to disk
    async fn save_validation_results(&self, result: &BatchValidationResult) -> Result<()> {
        // Create results directory
        std::fs::create_dir_all(&self.config.results_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}/validation_result_{}.json", self.config.results_dir, timestamp);

        let json_data = serde_json::to_string_pretty(result)?;
        std::fs::write(&filename, json_data)?;

        info!("Validation results saved to: {}", filename);

        // Also save a summary report
        let report_filename = format!("{}/validation_report_{}.txt", self.config.results_dir, timestamp);
        let report = self.generate_batch_report(result);
        std::fs::write(&report_filename, report)?;

        info!("Validation report saved to: {}", report_filename);

        Ok(())
    }

    /// Generate human-readable batch report
    fn generate_batch_report(&self, result: &BatchValidationResult) -> String {
        let mut report = String::new();
        let s = &result.summary;

        report.push_str("=== BATCH VALIDATION REPORT ===\n");
        report.push_str(&format!("Timestamp: {}\n", result.validation_timestamp));
        report.push_str(&format!("Total Transactions: {}\n", s.total_transactions));
        report.push_str(&format!("Successful Comparisons: {}\n", s.successful_comparisons));
        report.push_str(&format!("Overall Success Rate: {:.2}%\n", s.overall_success_rate * 100.0));
        report.push_str(&format!("\n"));

        report.push_str("=== PROCESSING FAILURES ===\n");
        report.push_str(&format!("Failed Rust: {}\n", s.failed_rust));
        report.push_str(&format!("Failed Python: {}\n", s.failed_python));
        report.push_str(&format!("\n"));

        report.push_str("=== COMPARISON RESULTS ===\n");
        report.push_str(&format!("Exact Matches: {}\n", s.exact_matches));
        report.push_str(&format!("Within Tolerance: {}\n", s.within_tolerance));
        report.push_str(&format!("Significant Differences: {}\n", s.significant_differences));
        report.push_str(&format!("\n"));

        report.push_str("=== PERFORMANCE ===\n");
        report.push_str(&format!("Average Rust Time: {:.2}ms\n", s.average_rust_time_ms));
        report.push_str(&format!("Average Python Time: {:.2}ms\n", s.average_python_time_ms));
        report.push_str(&format!("Rust vs Python Speed: {:.2}x\n", 
                                s.average_python_time_ms / s.average_rust_time_ms.max(1.0)));
        report.push_str(&format!("\n"));

        report.push_str("=== TRANSACTION TYPES ===\n");
        for (tx_type, count) in &s.transaction_types {
            report.push_str(&format!("{}: {}\n", tx_type, count));
        }
        report.push_str(&format!("\n"));

        // Detailed transaction results
        report.push_str("=== DETAILED RESULTS ===\n");
        for comparison in &result.transaction_comparisons {
            if !comparison.summary.overall_match {
                report.push_str(&format!("Transaction: {}\n", comparison.transaction_hash));
                report.push_str(&format!("  Block: {}\n", comparison.block_number));
                report.push_str(&format!("  Rust Success: {}\n", comparison.rust_success));
                report.push_str(&format!("  Python Success: {}\n", comparison.python_success));
                report.push_str(&format!("  Overall Match: {}\n", comparison.summary.overall_match));
                report.push_str(&format!("  Significant Differences: {}\n", comparison.summary.significant_differences));
                report.push_str(&format!("\n"));
            }
        }

        report
    }

    /// Run quick validation on a few recent transactions
    pub async fn quick_validation(&self) -> Result<BatchValidationSummary> {
        info!("Running quick validation...");

        // Use a smaller config for quick testing
        let mut quick_config = self.config.clone();
        quick_config.fetch_config.blocks_to_scan = 2;
        quick_config.fetch_config.max_transactions_per_block = 3;

        let mut quick_validator = BatchValidator::new(
            self.provider.clone(),
            quick_config,
        );

        let result = quick_validator.run_validation().await?;
        Ok(result.summary)
    }

    /// Test Python environment setup
    pub async fn test_environment(&self) -> Result<()> {
        info!("Testing Python environment setup...");
        self.python_bridge.test_environment().await
    }

    /// Run batch validation on recent transactions
    pub async fn validate_batch(&mut self, transaction_count: usize) -> Result<BatchValidationResult> {
        info!("Starting batch validation...");
        
        // Test environment first
        self.test_environment().await?;
        
        // Fetch test transactions
        let transactions = self.fetcher
            .fetch_test_transactions()
            .await?;
        
        // Take only the requested number of transactions
        let transactions: Vec<_> = transactions.into_iter()
            .take(transaction_count)
            .map(|candidate| (candidate.hash, candidate.block_number))
            .collect();
        
        info!("Fetched {} transaction candidates for validation", transactions.len());
        
        let mut results = Vec::new();
        let mut successful_comparisons = 0;
        let mut failed_rust = 0;
        let mut failed_python = 0;
        let mut total_rust_time = 0u64;
        let mut total_python_time = 0u64;
        
        // Ensure logs directory exists
        self.ensure_logs_directory()?;
        
        for (i, (tx_hash, block_number)) in transactions.iter().enumerate() {
            info!("Processing transaction {}/{}: {}", i + 1, transactions.len(), tx_hash);
            
            // Process through Python
            info!("Processing transaction {} through Python", tx_hash);
            let python_result = match self.python_bridge.process_transaction(tx_hash, *block_number).await {
                Ok(result) => {
                    if result.success {
                        total_python_time += result.processing_time_ms;
                        result
                    } else {
                        failed_python += 1;
                        error!("Failed to process transaction {}: Python processing failed: {}", 
                               tx_hash, result.error_message.as_deref().unwrap_or("Unknown error"));
                        continue;
                    }
                }
                Err(e) => {
                    failed_python += 1;
                    error!("Failed to process transaction {}: {}", tx_hash, e);
                    continue;
                }
            };
            
            // Process through Rust using our ComprehensiveStateDiffCalculator
            info!("Processing transaction {} through Rust", tx_hash);
            let rust_start = std::time::Instant::now();
            
            // Parse transaction hash
            let tx_hash_h256 = match tx_hash.parse::<H256>() {
                Ok(hash) => hash,
                Err(e) => {
                    failed_rust += 1;
                    error!("Failed to parse transaction hash {}: {}", tx_hash, e);
                    continue;
                }
            };
            
            // Fetch the transaction to get TransactionView
            let tx_option = self.provider.get_transaction(tx_hash_h256).await
                .map_err(|e| eyre::eyre!("Failed to fetch transaction: {}", e))?;
            
            let rust_changes = match tx_option {
                Some(tx) => {
                    // Convert ethers Transaction to our TransactionView
                    let tx_view = TransactionView {
                        hash: tx.hash.as_bytes().to_vec(),
                        from: tx.from.as_bytes().to_vec(),
                        to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                        value: tx.value,
                        gas_price: tx.gas_price,
                        gas_limit: Some(tx.gas),
                        nonce: Some(tx.nonce),
                        input_data: Some(tx.input.to_vec()),
                    };
                    
                    // Get transaction index from the block
                    let tx_receipt = self.provider.get_transaction_receipt(tx_hash_h256).await
                        .map_err(|e| eyre::eyre!("Failed to fetch transaction receipt: {}", e))?;
                    
                    let txn_index = match tx_receipt {
                        Some(receipt) => receipt.transaction_index.as_u64(),
                        None => return Err(eyre::eyre!("Transaction receipt not found")),
                    };
                    
                    match self.rust_calculator.calculate_state_changes(&tx_view, *block_number, txn_index).await {
                        Ok(changes) => {
                            info!("Rust successfully calculated {} state changes for {}", changes.len(), tx_hash);
                            changes
                        },
                        Err(e) => {
                            failed_rust += 1;
                            error!("Rust state calculation failed for {}: {}", tx_hash, e);
                            continue;
                        }
                    }
                },
                None => {
                    warn!("Transaction {} not found", tx_hash);
                    HashMap::new()
                }
            };
            
            let rust_time_ms = rust_start.elapsed().as_millis() as u64;
            total_rust_time += rust_time_ms;
            
            // Compare results
            info!("Comparing results for transaction {}", tx_hash);
            let comparison = self.comparison_engine.compare_transaction(
                &rust_changes,
                &python_result,
                rust_time_ms,
            );
            
            // Log detailed transaction results
            self.log_transaction_details(&tx_hash, *block_number, &rust_changes, &python_result, &comparison).await?;
            
            results.push(comparison);
            successful_comparisons += 1;
        }
        
        let batch_result = BatchValidationResult {
            summary: BatchValidationSummary {
                total_transactions: transactions.len(),
                successful_comparisons,
                failed_rust,
                failed_python,
                exact_matches: results.iter().map(|r| r.summary.exact_matches).sum(),
                within_tolerance: results.iter().map(|r| r.summary.within_tolerance).sum(),
                significant_differences: results.iter().map(|r| r.summary.significant_differences).sum(),
                overall_success_rate: if transactions.len() > 0 { 
                    successful_comparisons as f64 / transactions.len() as f64 
                } else { 0.0 },
                average_rust_time_ms: if successful_comparisons > 0 { 
                    total_rust_time as f64 / successful_comparisons as f64 
                } else { 0.0 },
                average_python_time_ms: if successful_comparisons > 0 { 
                    total_python_time as f64 / successful_comparisons as f64 
                } else { 0.0 },
                transaction_types: HashMap::new(),
            },
            transaction_comparisons: results,
            validation_timestamp: chrono::Utc::now().to_rfc3339(),
            config_summary: format!("{:?}", self.config),
        };
        
        // Save batch results
        self.save_batch_results(&batch_result).await?;
        
        info!("Batch validation completed: {}/{} successful comparisons", 
              successful_comparisons, transactions.len());
        
        Ok(batch_result)
    }

    /// Ensure logs directory exists
    fn ensure_logs_directory(&self) -> Result<()> {
        let logs_path = Path::new(&self.config.logs_dir);
        if !logs_path.exists() {
            fs::create_dir_all(logs_path)?;
            info!("Created logs directory: {}", self.config.logs_dir);
        }
        Ok(())
    }

    /// Log detailed transaction results
    async fn log_transaction_details(
        &self,
        tx_hash: &str,
        block_number: u64,
        rust_changes: &HashMap<String, ComprehensiveStateChange>,
        python_result: &PythonResult,
        comparison: &TransactionComparison,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let safe_hash = tx_hash.replace("0x", "");
        
        // Create detailed log entry
        let log_entry = TransactionLogEntry {
            transaction_hash: tx_hash.to_string(),
            block_number,
            timestamp: chrono::Utc::now().to_rfc3339(),
            rust_success: comparison.rust_success,
            python_success: comparison.python_success,
            rust_changes: rust_changes.clone(),
            python_result: python_result.clone(),
            comparison: comparison.clone(),
            rust_processing_time_ms: comparison.rust_processing_time_ms,
            python_processing_time_ms: comparison.python_processing_time_ms,
        };
        
        // Save detailed JSON log
        // let json_filename = format!("{}/transaction_{}_{}.json", 
        //                            self.config.logs_dir, safe_hash, timestamp);
        // let json_content = serde_json::to_string_pretty(&log_entry)?;
        // fs::write(&json_filename, json_content)?;
        
        // Create human-readable summary
        // let summary_filename = format!("{}/transaction_{}_{}.txt", 
        //                               self.config.logs_dir, safe_hash, timestamp);
        // let summary_content = self.create_transaction_summary(&log_entry);
        // fs::write(&summary_filename, summary_content)?;
        
        // info!("Logged transaction details to: {} and {}", json_filename, summary_filename);
        
        Ok(())
    }

    /// Create human-readable transaction summary
    fn create_transaction_summary(&self, log_entry: &TransactionLogEntry) -> String {
        let mut summary = String::new();
        
        summary.push_str(&format!("=== TRANSACTION VALIDATION SUMMARY ===\n"));
        summary.push_str(&format!("Transaction Hash: {}\n", log_entry.transaction_hash));
        summary.push_str(&format!("Block Number: {}\n", log_entry.block_number));
        summary.push_str(&format!("Timestamp: {}\n", log_entry.timestamp));
        summary.push_str(&format!("Rust Success: {}\n", log_entry.rust_success));
        summary.push_str(&format!("Python Success: {}\n", log_entry.python_success));
        summary.push_str(&format!("Rust Processing Time: {}ms\n", log_entry.rust_processing_time_ms));
        summary.push_str(&format!("Python Processing Time: {}ms\n", log_entry.python_processing_time_ms));
        summary.push_str(&format!("\n"));
        
        // Python Results
        summary.push_str(&format!("=== PYTHON STATE CHANGES ({} addresses) ===\n", 
                                 log_entry.python_result.state_changes.len()));
        for change in &log_entry.python_result.state_changes {
            summary.push_str(&format!("Address: {}\n", change.address));
            summary.push_str(&format!("  Token Net: {}\n", change.token_net));
            summary.push_str(&format!("  Denom Net: {:.9}\n", change.denom_net));
            summary.push_str(&format!("\n"));
        }
        
        // Rust Results (placeholder for now)
        summary.push_str(&format!("=== RUST STATE CHANGES ({} addresses) ===\n", 
                                 log_entry.rust_changes.len()));
        if log_entry.rust_changes.is_empty() {
            summary.push_str("(Placeholder - Rust implementation not yet connected)\n");
        } else {
            for (address, change) in &log_entry.rust_changes {
                summary.push_str(&format!("Address: {}\n", address));
                summary.push_str(&format!("  Token Net: {}\n", change.token_net));
                summary.push_str(&format!("  Denom Net: {:.9}\n", change.denom_net));
                summary.push_str(&format!("\n"));
            }
        }
        
        // Comparison Results
        summary.push_str(&format!("=== COMPARISON RESULTS ===\n"));
        let comp = &log_entry.comparison.summary;
        summary.push_str(&format!("Total Addresses: {}\n", comp.total_addresses));
        summary.push_str(&format!("Exact Matches: {}\n", comp.exact_matches));
        summary.push_str(&format!("Within Tolerance: {}\n", comp.within_tolerance));
        summary.push_str(&format!("Significant Differences: {}\n", comp.significant_differences));
        summary.push_str(&format!("Only in Rust: {}\n", comp.only_in_rust));
        summary.push_str(&format!("Only in Python: {}\n", comp.only_in_python));
        summary.push_str(&format!("Overall Match: {}\n", comp.overall_match));
        summary.push_str(&format!("\n"));
        
        // Address-by-address comparison
        if !log_entry.comparison.address_comparisons.is_empty() {
            summary.push_str(&format!("=== ADDRESS COMPARISONS ===\n"));
            for addr_comp in &log_entry.comparison.address_comparisons {
                summary.push_str(&format!("Address: {}\n", addr_comp.address));
                summary.push_str(&format!("  Status: {:?}\n", addr_comp.status));
                summary.push_str(&format!("  Rust:   token_net={}, denom_net={:.9}\n", 
                                        addr_comp.rust_token_net, addr_comp.rust_denom_net));
                summary.push_str(&format!("  Python: token_net={}, denom_net={:.9}\n", 
                                        addr_comp.python_token_net, addr_comp.python_denom_net));
                summary.push_str(&format!("  Diff:   token_diff={}, denom_diff={:.9}\n", 
                                        addr_comp.token_diff, addr_comp.denom_diff));
                summary.push_str(&format!("\n"));
            }
        }
        
        summary
    }

    /// Save batch validation results
    async fn save_batch_results(&self, results: &BatchValidationResult) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        
        // Save JSON results
        let json_filename = format!("{}/validation_result_{}.json", 
                                   self.config.logs_dir, timestamp);
        let json_content = serde_json::to_string_pretty(results)?;
        fs::write(&json_filename, json_content)?;
        
        // Save human-readable report
        let report_filename = format!("{}/validation_report_{}.txt", 
                                     self.config.logs_dir, timestamp);
        let report_content = self.create_batch_report(results);
        fs::write(&report_filename, report_content)?;
        
        info!("Validation results saved to: {}", json_filename);
        info!("Validation report saved to: {}", report_filename);
        
        Ok(())
    }

    /// Create human-readable batch report
    fn create_batch_report(&self, results: &BatchValidationResult) -> String {
        let mut report = String::new();
        
        report.push_str(&format!("=== BATCH VALIDATION REPORT ===\n"));
        report.push_str(&format!("Timestamp: {}\n", results.validation_timestamp));
        report.push_str(&format!("Total Transactions: {}\n", results.summary.total_transactions));
        report.push_str(&format!("Successful Comparisons: {}\n", results.summary.successful_comparisons));
        report.push_str(&format!("Overall Success Rate: {:.2}%\n", 
                                if results.summary.total_transactions > 0 {
                                    (results.summary.successful_comparisons as f64 / results.summary.total_transactions as f64) * 100.0
                                } else { 0.0 }));
        report.push_str(&format!("\n"));
        
        report.push_str(&format!("=== PROCESSING FAILURES ===\n"));
        report.push_str(&format!("Failed Rust: {}\n", results.summary.failed_rust));
        report.push_str(&format!("Failed Python: {}\n", results.summary.failed_python));
        report.push_str(&format!("\n"));
        
        report.push_str(&format!("=== COMPARISON RESULTS ===\n"));
        report.push_str(&format!("Exact Matches: {}\n", results.summary.exact_matches));
        report.push_str(&format!("Within Tolerance: {}\n", results.summary.within_tolerance));
        report.push_str(&format!("Significant Differences: {}\n", results.summary.significant_differences));
        report.push_str(&format!("\n"));
        
        report.push_str(&format!("=== PERFORMANCE ===\n"));
        report.push_str(&format!("Average Rust Time: {:.2}ms\n", results.summary.average_rust_time_ms));
        report.push_str(&format!("Average Python Time: {:.2}ms\n", results.summary.average_python_time_ms));
        if results.summary.average_python_time_ms > 0.0 {
            report.push_str(&format!("Rust vs Python Speed: {:.2}x\n", 
                                   results.summary.average_python_time_ms / results.summary.average_rust_time_ms.max(0.01)));
        }
        report.push_str(&format!("\n"));
        
        // Transaction type breakdown
        let mut transaction_types = std::collections::HashMap::new();
        for result in &results.transaction_comparisons {
            // For now, we'll classify all as "Token Transfer" since we don't have detailed classification
            *transaction_types.entry("Token Transfer".to_string()).or_insert(0) += 1;
        }
        
        report.push_str(&format!("=== TRANSACTION TYPES ===\n"));
        for (tx_type, count) in transaction_types {
            report.push_str(&format!("{}: {}\n", tx_type, count));
        }
        report.push_str(&format!("\n"));
        
        report.push_str(&format!("=== DETAILED RESULTS ===\n"));
        for result in &results.transaction_comparisons {
            report.push_str(&format!("Transaction: {}\n", result.transaction_hash));
            report.push_str(&format!("  Block: {}\n", result.block_number));
            report.push_str(&format!("  Rust Success: {}\n", result.rust_success));
            report.push_str(&format!("  Python Success: {}\n", result.python_success));
            report.push_str(&format!("  Overall Match: {}\n", result.summary.overall_match));
            report.push_str(&format!("  Significant Differences: {}\n", result.summary.significant_differences));
            report.push_str(&format!("\n"));
        }
        
        report
    }
} 