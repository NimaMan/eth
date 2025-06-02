use super::{BatchValidator, batch_validator::BatchValidationConfig};
use ethers::prelude::*;
use eyre::Result;
use std::sync::Arc;
use tracing::{info, warn};

/// Test runner for validation testing
pub struct TestRunner {
    provider: Arc<Provider<Http>>,
}

impl TestRunner {
    pub fn new(reth_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(reth_url)?;
        let provider = Arc::new(provider);
        
        Ok(Self { provider })
    }

    /// Run quick validation test (3 transactions)
    pub async fn run_quick_test(&self) -> Result<()> {
        println!("🚀 Running Quick Validation Test (3 transactions)...");
        
        let mut config = BatchValidationConfig::default();
        config.logs_dir = "/home/nima/code/crypto/logs/mempool".to_string();
        
        let mut validator = BatchValidator::new(self.provider.clone(), config);
        let result = validator.validate_batch(3).await?;
        
        println!("✅ Quick test completed!");
        println!("   Success rate: {:.1}%", 
                if result.summary.total_transactions > 0 {
                    (result.summary.successful_comparisons as f64 / result.summary.total_transactions as f64) * 100.0
                } else { 0.0 });
        println!("   Exact matches: {}", result.summary.exact_matches);
        println!("   Results logged to: {}", "/home/nima/code/crypto/logs/mempool");
        
        Ok(())
    }

    /// Run comprehensive validation test (10 transactions)
    pub async fn run_comprehensive_test(&self) -> Result<()> {
        println!("🔍 Running Comprehensive Validation Test (10 transactions)...");
        
        let mut config = BatchValidationConfig::default();
        config.logs_dir = "/home/nima/code/crypto/logs/mempool".to_string();
        
        let mut validator = BatchValidator::new(self.provider.clone(), config);
        let result = validator.validate_batch(10).await?;
        
        println!("✅ Comprehensive test completed!");
        println!("   Success rate: {:.1}%", 
                if result.summary.total_transactions > 0 {
                    (result.summary.successful_comparisons as f64 / result.summary.total_transactions as f64) * 100.0
                } else { 0.0 });
        println!("   Exact matches: {}", result.summary.exact_matches);
        println!("   Within tolerance: {}", result.summary.within_tolerance);
        println!("   Significant differences: {}", result.summary.significant_differences);
        println!("   Average Python time: {:.2}ms", result.summary.average_python_time_ms);
        println!("   Results logged to: {}", "/home/nima/code/crypto/logs/mempool");
        
        Ok(())
    }

    /// Run stress test (many transactions across multiple blocks)
    pub async fn run_stress_test(&self) -> Result<()> {
        info!("🚀 Starting stress validation test...");
        
        let config = BatchValidationConfig {
            fetch_config: super::transaction_fetcher::FetchConfig {
                blocks_to_scan: 50,  // Increased from 10 to 50
                max_transactions_per_block: 25,  // Increased from 10 to 25  
                min_gas_used: 50_000,
                require_logs: false, // Include more transaction types
                require_internal_txns: false,
            },
            max_transactions: 1000,  // Set explicit limit to 1000
            save_results: true,
            results_dir: "stress_test_results".to_string(),
            ..Default::default()
        };

        let mut validator = BatchValidator::new(self.provider.clone(), config);
        let result = validator.run_validation().await?;

        self.print_detailed_summary(&result.summary);
        
        // Report non-exact matches
        if result.summary.within_tolerance > 0 || result.summary.significant_differences > 0 {
            warn!("⚠️  Found {} non-exact matches (within tolerance: {}, significant differences: {})", 
                  result.summary.within_tolerance + result.summary.significant_differences,
                  result.summary.within_tolerance,
                  result.summary.significant_differences);
        } else {
            info!("✅ ALL {} transactions had EXACT matches!", result.summary.exact_matches);
        }
        
        if result.summary.overall_success_rate > 0.95 {
            info!("✅ Stress test PASSED - Success rate: {:.1}%", 
                  result.summary.overall_success_rate * 100.0);
        } else {
            warn!("⚠️  Stress test had issues - Success rate: {:.1}%", 
                  result.summary.overall_success_rate * 100.0);
        }

        Ok(())
    }

    /// Test Python environment setup
    pub async fn test_python_environment(&self) -> Result<()> {
        info!("🔧 Testing Python environment...");
        
        let python_bridge = super::PythonBridge::new(Default::default());
        python_bridge.test_environment().await?;
        
        info!("✅ Python environment test passed!");
        Ok(())
    }

    /// Run all validation tests in sequence
    pub async fn run_all_tests(&self) -> Result<()> {
        info!("🚀 Running complete validation test suite...");
        
        // Test Python environment first
        self.test_python_environment().await?;
        
        // Run tests in order of complexity
        println!("\n{}", "=".repeat(50));
        println!("1/4: Quick Test");
        println!("{}", "=".repeat(50));
        self.run_quick_test().await?;
        
        println!("\n{}", "=".repeat(50));
        println!("2/4: Comprehensive Test");
        println!("{}", "=".repeat(50));
        self.run_comprehensive_test().await?;
        
        println!("\n{}", "=".repeat(50));
        println!("3/4: Token Transfer Test");
        println!("{}", "=".repeat(50));
        self.run_transaction_type_test("Token Transfer").await?;
        
        println!("\n{}", "=".repeat(50));
        println!("4/4: Stress Test");
        println!("{}", "=".repeat(50));
        self.run_stress_test().await?;
        
        info!("🎉 Complete validation test suite finished!");
        Ok(())
    }

    /// Test specific transaction types
    pub async fn run_transaction_type_test(&self, tx_type: &str) -> Result<()> {
        info!("🚀 Starting {} transaction validation test...", tx_type);
        
        // This would need enhancement to filter by specific transaction types
        // For now, run a general test and report on the specific type
        let config = BatchValidationConfig {
            fetch_config: super::transaction_fetcher::FetchConfig {
                blocks_to_scan: 8,
                max_transactions_per_block: 8,
                min_gas_used: 100_000,
                require_logs: true,
                require_internal_txns: false,
            },
            save_results: true,
            results_dir: format!("{}_test_results", tx_type.to_lowercase().replace(" ", "_")),
            ..Default::default()
        };

        let mut validator = BatchValidator::new(self.provider.clone(), config);
        let result = validator.run_validation().await?;

        // Filter results for the specific transaction type
        let type_specific_count = result.summary.transaction_types.get(tx_type).unwrap_or(&0);
        
        info!("Found {} transactions of type '{}'", type_specific_count, tx_type);
        self.print_detailed_summary(&result.summary);

        Ok(())
    }

    /// Print basic summary
    fn print_summary(&self, summary: &super::batch_validator::BatchValidationSummary) {
        println!("\n=== VALIDATION SUMMARY ===");
        println!("Total Transactions: {}", summary.total_transactions);
        println!("Successful Comparisons: {}", summary.successful_comparisons);
        println!("Success Rate: {:.1}%", summary.overall_success_rate * 100.0);
        println!("Exact Matches: {}", summary.exact_matches);
        println!("Significant Differences: {}", summary.significant_differences);
        println!("Average Rust Time: {:.1}ms", summary.average_rust_time_ms);
        println!("Average Python Time: {:.1}ms", summary.average_python_time_ms);
        
        if summary.average_rust_time_ms > 0.0 {
            let speed_ratio = summary.average_python_time_ms / summary.average_rust_time_ms;
            println!("Rust Speed Advantage: {:.1}x faster", speed_ratio);
        }
    }

    /// Print detailed summary with transaction types
    fn print_detailed_summary(&self, summary: &super::batch_validator::BatchValidationSummary) {
        self.print_summary(summary);
        
        println!("\n=== TRANSACTION TYPES ===");
        for (tx_type, count) in &summary.transaction_types {
            println!("{}: {}", tx_type, count);
        }
        
        println!("\n=== DETAILED BREAKDOWN ===");
        println!("Failed Rust: {}", summary.failed_rust);
        println!("Failed Python: {}", summary.failed_python);
        println!("Within Tolerance: {}", summary.within_tolerance);
    }
} 