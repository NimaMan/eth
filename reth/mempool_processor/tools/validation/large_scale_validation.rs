use ethers::providers::{Http, Provider};
use std::sync::Arc;
use mempool_processor::validation_testing::{BatchValidator, batch_validator::BatchValidationConfig};
use mempool_processor::validation_testing::transaction_fetcher::FetchConfig;
use mempool_processor::validation_testing::comparison_engine::ComparisonStatus;
use tracing::{info, warn};
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Large-Scale Validation (1000+ transactions)");
    info!("📊 Will log all non-exact matches for detailed analysis");
    
    // Connect to local Ethereum node
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545")?);
    
    // Get target count from command line or default to 1000
    let target_count: u32 = std::env::args().nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    info!("🎯 Target transaction count: {}", target_count);

    // Create configuration for large-scale testing (1000+ transactions)
    let config = BatchValidationConfig {
        fetch_config: FetchConfig {
            blocks_to_scan: 500,  // Scan many more blocks to get 1000+ transactions
            max_transactions_per_block: 50,  // Allow more transactions per block
            min_gas_used: 21_000,  // Lower threshold to get more transactions
            require_logs: false,   // Include simple transfers too
            require_internal_txns: false,
        },
        max_transactions: target_count as usize,  // Use command line target or 1000
        save_results: true,
        results_dir: "large_scale_validation_1000plus_results".to_string(),
        logs_dir: "/home/nima/code/crypto/logs/mempool".to_string(),
        ..Default::default()
    };
    
    // Create logs directory for non-exact matches
    let non_exact_dir = "/home/nima/code/crypto/logs/mempool/non_exact_matches_1000plus";
    fs::create_dir_all(non_exact_dir)?;
    
    // Create batch validator
    let mut validator = BatchValidator::new(provider, config);
    
    info!("🧪 Starting large-scale validation...");
    
    // Run the validation
    let result = validator.validate_batch(target_count as usize).await?;
    
    // Analyze results for non-exact matches
    let mut exact_matches = 0;
    let mut non_exact_matches = 0;
    
    info!("📊 Analyzing {} transaction results...", result.transaction_comparisons.len());
    
    for comparison in &result.transaction_comparisons {
        if comparison.rust_success && comparison.python_success {
            let is_exact_match = comparison.summary.exact_matches == comparison.summary.total_addresses && 
                               comparison.summary.only_in_rust == 0 && 
                               comparison.summary.only_in_python == 0;
            
            if is_exact_match {
                exact_matches += 1;
                info!("✅ Transaction {} - EXACT MATCH", comparison.transaction_hash);
            } else {
                non_exact_matches += 1;
                warn!("⚠️  Transaction {} - NON-EXACT MATCH", comparison.transaction_hash);
                
                // Log detailed information for non-exact matches
                let log_file = format!("{}/non_exact_{}.json", non_exact_dir, comparison.transaction_hash);
                let detailed_json = serde_json::to_string_pretty(&comparison)?;
                fs::write(&log_file, detailed_json)?;
                
                // Create a summary log
                let summary_file = format!("{}/non_exact_{}_summary.txt", non_exact_dir, comparison.transaction_hash);
                let mut summary = String::new();
                summary.push_str(&format!("NON-EXACT MATCH ANALYSIS\n"));
                summary.push_str(&format!("========================\n"));
                summary.push_str(&format!("Transaction: {}\n", comparison.transaction_hash));
                summary.push_str(&format!("Block: {}\n", comparison.block_number));
                summary.push_str(&format!("Total addresses: {}\n", comparison.summary.total_addresses));
                summary.push_str(&format!("Exact matches: {}\n", comparison.summary.exact_matches));
                summary.push_str(&format!("Within tolerance: {}\n", comparison.summary.within_tolerance));
                summary.push_str(&format!("Only in Rust: {}\n", comparison.summary.only_in_rust));
                summary.push_str(&format!("Only in Python: {}\n", comparison.summary.only_in_python));
                summary.push_str(&format!("Significant differences: {}\n", comparison.summary.significant_differences));
                
                // Log specific address issues
                summary.push_str("\nADDRESS DETAILS:\n");
                for addr_comp in &comparison.address_comparisons {
                    if !matches!(addr_comp.status, ComparisonStatus::ExactMatch) {
                        summary.push_str(&format!("  {} - {:?}\n", addr_comp.address, addr_comp.status));
                        summary.push_str(&format!("    Rust:   token={}, denom={}\n", 
                                                addr_comp.rust_token_net, addr_comp.rust_denom_net));
                        summary.push_str(&format!("    Python: token={}, denom={}\n", 
                                                addr_comp.python_token_net, addr_comp.python_denom_net));
                        summary.push_str(&format!("    Diffs:  token={}, denom={}\n", 
                                                addr_comp.token_diff, addr_comp.denom_diff));
                    }
                }
                
                fs::write(&summary_file, summary)?;
                info!("📝 Logged non-exact match details to: {}", log_file);
                info!("📋 Logged summary to: {}", summary_file);
            }
        }
    }
    
    // Final summary
    info!("🎉 LARGE-SCALE VALIDATION COMPLETE");
    info!("=====================================");
    info!("📊 Total transactions: {}", result.summary.total_transactions);
    info!("✅ Exact matches: {} ({:.1}%)", exact_matches, 
          (exact_matches as f64 / result.summary.total_transactions as f64) * 100.0);
    info!("⚠️  Non-exact matches: {} ({:.1}%)", non_exact_matches,
          (non_exact_matches as f64 / result.summary.total_transactions as f64) * 100.0);
    info!("❌ Failed transactions: {} ({:.1}%)", 
          result.summary.total_transactions - result.summary.successful_comparisons,
          ((result.summary.total_transactions - result.summary.successful_comparisons) as f64 / result.summary.total_transactions as f64) * 100.0);
    
    if non_exact_matches > 0 {
        warn!("📁 Non-exact match details logged to: {}", non_exact_dir);
    }
    
    info!("🎯 Overall success rate: {:.1}%", result.summary.overall_success_rate * 100.0);
    info!("⚡ Average Rust time: {:.1}ms", result.summary.average_rust_time_ms);
    info!("⚡ Average Python time: {:.1}ms", result.summary.average_python_time_ms);
    
    if result.summary.average_rust_time_ms > 0.0 {
        let speed_ratio = result.summary.average_python_time_ms / result.summary.average_rust_time_ms;
        info!("🚀 Rust speed advantage: {:.1}x faster", speed_ratio);
    }
    
    Ok(())
} 