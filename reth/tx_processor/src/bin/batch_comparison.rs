//! Batch Transaction Comparison Tool
//! 
//! Compare multiple transactions between Rust and Python implementations

// Note: This binary uses internal modules directly
// In production, this would use the revm_tx_simulator_lib crate
use std::env;
use std::time::Instant;

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";
const DEFAULT_PYTHON_URL: &str = "http://127.0.0.1:18000";

// Test transaction hashes (known mainnet transactions)
const TEST_TRANSACTIONS: &[&str] = &[
    "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060", // Simple ETH transfer
    "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b", // ERC20 transfer
    "0xf7bd63f3d0c3b0b1b2a6bb8b0b1b0b1b0b1b0b1b0b1b0b1b0b1b0b1b0b1b0b1", // DeFi transaction (placeholder)
    "0xa1b2c3d4e5f6789012345678901234567890123456789012345678901234567", // Complex transaction (placeholder)
    "0xb2c3d4e5f6789012345678901234567890123456789012345678901234567890", // Uniswap swap (placeholder)
    "0xc3d4e5f6789012345678901234567890123456789012345678901234567890123", // Token approval (placeholder)
    "0xd4e5f6789012345678901234567890123456789012345678901234567890123456", // NFT transfer (placeholder)
    "0xe5f6789012345678901234567890123456789012345678901234567890123456789", // Compound interaction (placeholder)
    "0xf6789012345678901234567890123456789012345678901234567890123456789012", // Aave transaction (placeholder)
    "0x0123456789012345678901234567890123456789012345678901234567890123456", // Multi-token swap (placeholder)
];

#[derive(Debug)]
struct BatchSummary {
    total_transactions: usize,
    successful_matches: usize,
    failed_matches: usize,
    success_rate: f64,
    total_rust_time: f64,
    total_python_time: f64,
    avg_rust_time: f64,
    avg_python_time: f64,
    speedup_factor: f64,
}

impl BatchSummary {
    fn from_results(results: &[ValidationResult]) -> Self {
        let total = results.len();
        let successful = results.iter().filter(|r| r.matches).count();
        let failed = total - successful;
        let success_rate = if total > 0 { (successful as f64 / total as f64) * 100.0 } else { 0.0 };
        
        let total_rust_time: f64 = results.iter().map(|r| r.rust_processing_time_ms).sum();
        let total_python_time: f64 = results.iter().map(|r| r.python_processing_time_ms).sum();
        
        let avg_rust_time = if total > 0 { total_rust_time / total as f64 } else { 0.0 };
        let avg_python_time = if total > 0 { total_python_time / total as f64 } else { 0.0 };
        
        let speedup_factor = if avg_rust_time > 0.0 { avg_python_time / avg_rust_time } else { 0.0 };
        
        BatchSummary {
            total_transactions: total,
            successful_matches: successful,
            failed_matches: failed,
            success_rate,
            total_rust_time,
            total_python_time,
            avg_rust_time,
            avg_python_time,
            speedup_factor,
        }
    }
    
    fn print(&self) {
        println!("📊 Batch Comparison Summary");
        println!("════════════════════════════");
        println!("📈 Match Results:");
        println!("   Total transactions: {}", self.total_transactions);
        println!("   ✅ Successful matches: {}", self.successful_matches);
        println!("   ❌ Failed matches: {}", self.failed_matches);
        println!("   📊 Success rate: {:.1}%", self.success_rate);
        println!();
        
        println!("⚡ Performance Results:");
        println!("   🦀 Total Rust time: {:.1}ms", self.total_rust_time);
        println!("   🐍 Total Python time: {:.1}ms", self.total_python_time);
        println!("   🦀 Average Rust time: {:.1}ms", self.avg_rust_time);
        println!("   🐍 Average Python time: {:.1}ms", self.avg_python_time);
        println!("   🚀 Speedup factor: {:.1}x", self.speedup_factor);
        println!();
        
        let efficiency = if self.speedup_factor >= 2.0 { "Excellent" } else if self.speedup_factor >= 1.5 { "Good" } else { "Needs improvement" };
        println!("💯 Overall Performance: {}", efficiency);
    }
}

fn print_detailed_results(results: &[ValidationResult]) {
    println!("📋 Detailed Results");
    println!("═══════════════════");
    
    for (i, result) in results.iter().enumerate() {
        let status = if result.matches { "✅ MATCH" } else { "❌ MISMATCH" };
        let tx_short = &result.tx_hash[..10];
        
        println!("{:2}. {} {} (Rust: {:.1}ms, Python: {:.1}ms)", 
            i + 1, status, tx_short, result.rust_processing_time_ms, result.python_processing_time_ms);
        
        if !result.matches && !result.differences.is_empty() {
            println!("    Differences:");
            for diff in &result.differences {
                println!("      • {}: {}", diff.field, diff.description);
            }
        }
    }
    println!();
}

async fn run_batch_comparison(tx_hashes: Vec<String>, detailed: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Batch Transaction Comparison");
    println!("Comparing {} transactions between Rust and Python implementations", tx_hashes.len());
    println!("📊 Rust RPC: {}", DEFAULT_RPC_URL);
    println!("🐍 Python Service: {}", DEFAULT_PYTHON_URL);
    println!();
    
    let start_time = Instant::now();
    
    match batch_compare_with_python(tx_hashes.clone(), DEFAULT_RPC_URL, Some(DEFAULT_PYTHON_URL)).await {
        Ok(results) => {
            let total_time = start_time.elapsed();
            
            // Print summary
            let summary = BatchSummary::from_results(&results);
            summary.print();
            
            println!("⏱️  Total wall time: {:.1}ms", total_time.as_secs_f64() * 1000.0);
            println!("🔄 Concurrency efficiency: {:.1}x", 
                (summary.total_rust_time + summary.total_python_time) / (total_time.as_secs_f64() * 1000.0));
            println!();
            
            // Print detailed results if requested
            if detailed {
                print_detailed_results(&results);
            }
            
            // Print failed transactions with details
            let failed_results: Vec<_> = results.iter().filter(|r| !r.matches).collect();
            if !failed_results.is_empty() {
                println!("🔍 Failed Comparisons Details:");
                println!("═══════════════════════════════");
                for result in failed_results {
                    println!("Transaction: {}", result.tx_hash);
                    for diff in &result.differences {
                        println!("  ❌ {}: {}", diff.field, diff.description);
                        println!("     Rust: {:?}", diff.rust_value);
                        println!("     Python: {:?}", diff.python_value);
                    }
                    println!();
                }
            }
        }
        Err(e) => {
            println!("❌ Batch comparison failed: {}", e);
            println!();
            println!("💡 Troubleshooting:");
            println!("   • Ensure Python service is running: python validation_service.py");
            println!("   • Ensure Reth node is running at {}", DEFAULT_RPC_URL);
            println!("   • Check that transactions exist and node is synced");
            return Err(e.into());
        }
    }
    
    Ok(())
}

fn print_usage() {
    println!("🔧 Batch Transaction Comparison Tool");
    println!();
    println!("Usage:");
    println!("  cargo run --bin batch_comparison [OPTIONS] [TX_HASHES...]");
    println!();
    println!("Options:");
    println!("  --test-10          Compare 10 predefined test transactions");
    println!("  --detailed         Show detailed results for each transaction");
    println!("  --help             Show this help message");
    println!();
    println!("Examples:");
    println!("  # Compare 10 test transactions");
    println!("  cargo run --bin batch_comparison -- --test-10");
    println!();
    println!("  # Compare specific transactions");
    println!("  cargo run --bin batch_comparison -- 0x5c504ed... 0x7b944d9...");
    println!();
    println!("  # Compare with detailed output");
    println!("  cargo run --bin batch_comparison -- --test-10 --detailed");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.contains(&"--help".to_string()) {
        print_usage();
        return Ok(());
    }
    
    let detailed = args.contains(&"--detailed".to_string());
    
    let tx_hashes = if args.contains(&"--test-10".to_string()) {
        // Use first 10 test transactions
        println!("🧪 Using 10 predefined test transactions");
        TEST_TRANSACTIONS[..10].iter().map(|s| s.to_string()).collect()
    } else {
        // Use provided transaction hashes
        let mut hashes = Vec::new();
        let mut skip_next = false;
        
        for arg in args.iter().skip(1) {
            if skip_next {
                skip_next = false;
                continue;
            }
            
            if arg.starts_with("--") {
                if arg == "--detailed" {
                    skip_next = false;
                } else {
                    skip_next = false;
                }
                continue;
            }
            
            if arg.starts_with("0x") && arg.len() >= 10 {
                hashes.push(arg.clone());
            }
        }
        
        if hashes.is_empty() {
            println!("❌ No valid transaction hashes provided");
            println!();
            print_usage();
            return Ok(());
        }
        
        hashes
    };
    
    if tx_hashes.is_empty() {
        println!("❌ No transaction hashes to compare");
        return Ok(());
    }
    
    run_batch_comparison(tx_hashes, detailed).await
}