/*
 * Validation Test Runner Binary
 * 
 * ALGORITHMIC DESCRIPTION:
 * This binary provides a command-line interface to run comprehensive validation tests
 * that compare the Rust state change implementation against the Python baseline.
 * 
 * Test Types:
 * 1. Quick Test: 2-3 recent transactions for fast validation
 * 2. Comprehensive Test: 10-15 diverse transactions with detailed reporting
 * 3. Stress Test: Many transactions across multiple blocks
 * 4. Type-Specific Tests: Focus on specific transaction types (swaps, transfers, etc.)
 * 5. Environment Test: Validate Python setup and dependencies
 * 
 * This serves the main objective by providing automated validation that ensures
 * the Rust implementation produces identical results to the proven Python implementation.
 */

use mempool_processor::validation_testing::TestRunner;
use clap::{Parser, Subcommand};
use tracing::{info, error};
use eyre::Result;

#[derive(Parser)]
#[command(name = "validation-tests")]
#[command(about = "Run validation tests comparing Rust and Python state change calculations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, default_value = "http://localhost:8545")]
    reth_url: String,
    
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Run quick validation test (2-3 recent transactions)
    Quick,
    
    /// Run comprehensive validation test (10-15 diverse transactions)
    Comprehensive,
    
    /// Run stress test (many transactions across multiple blocks)
    Stress,
    
    /// Test specific transaction types
    TypeTest {
        #[arg(help = "Transaction type to test (e.g., 'Token Transfer', 'Uniswap V2 Swap')")]
        transaction_type: String,
    },
    
    /// Test Python environment setup
    EnvTest,
    
    /// Run all validation tests in sequence
    All,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize tracing
    let log_level = match cli.verbose {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    info!("🚀 Starting Rust vs Python State Change Validation");
    info!("Using Reth node: {}", cli.reth_url);

    // Create test runner
    let test_runner = TestRunner::new(&cli.reth_url)?;

    // Execute the requested command
    let result = match cli.command {
        Commands::Quick => {
            println!("🏃‍♂️ Running Quick Validation Test...");
            test_runner.run_quick_test().await
        }
        
        Commands::Comprehensive => {
            println!("🔍 Running Comprehensive Validation Test...");
            test_runner.run_comprehensive_test().await
        }
        
        Commands::Stress => {
            println!("💪 Running Stress Validation Test...");
            test_runner.run_stress_test().await
        }
        
        Commands::TypeTest { transaction_type } => {
            println!("🎯 Running {} Transaction Type Test...", transaction_type);
            test_runner.run_transaction_type_test(&transaction_type).await
        }
        
        Commands::EnvTest => {
            println!("🔧 Testing Python Environment...");
            test_runner.test_python_environment().await
        }
        
        Commands::All => {
            println!("🌟 Running Complete Validation Test Suite...");
            test_runner.run_all_tests().await
        }
    };

    match result {
        Ok(()) => {
            info!("✅ Validation tests completed successfully!");
            println!("\n🎉 All tests completed! Check the validation_results directory for detailed reports.");
        }
        Err(e) => {
            error!("❌ Validation tests failed: {}", e);
            eprintln!("\n💥 Tests failed with error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
} 