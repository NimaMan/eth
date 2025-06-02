/*
 * Comprehensive State Diff Validation Test
 * 
 * ALGORITHMIC DESCRIPTION:
 * This test validates our Rust comprehensive state diff calculator against
 * the Python ProcessedTxStateDiffCalculator by:
 * 1. Processing the same transactions with both implementations
 * 2. Comparing the calculated state changes for accuracy
 * 3. Validating that we correctly combine ETH and token transfers
 * 4. Ensuring compatibility with the existing Python pipeline
 * 
 * The test serves the main objective by providing confidence that our
 * Rust implementation produces identical results to the proven Python
 * implementation, enabling seamless integration and validation.
 */

use clap::Parser;
use ethers::prelude::*;
use ethers::types::{H256, U64};
use eyre::Result;
use mempool_processor::tx_simulator::{ComprehensiveStateDiffCalculator, ComprehensiveStateChange};
use mempool_processor::mempool_processor::types::TransactionView;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error, debug};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(name = "test_comprehensive_state_diff")]
#[command(about = "Test comprehensive state diff calculation against Python implementation")]
struct Args {
    /// Ethereum RPC URL
    #[arg(long, default_value = "http://localhost:8545")]
    eth_rpc_url: String,
    
    /// Transaction hash to test
    #[arg()]
    tx_hash: String,
    
    /// Expected Python state changes (JSON format)
    #[arg(long)]
    expected_python_result: Option<String>,
    
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Expected state change from Python (matching your format)
#[derive(Debug, Clone, serde::Deserialize)]
struct PythonStateChange {
    token_net: f64,
    denom_net: f64,
    #[serde(default)]
    movements: Option<PythonMovements>, // Make movements optional for simplified tests
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PythonMovements {
    token: PythonAddressMovements,
    denom: PythonAddressMovements,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct PythonAddressMovements {
    #[serde(rename = "in")]
    incoming: HashMap<String, f64>, // Using string keys for Python compatibility
    #[serde(rename = "out")]
    outgoing: HashMap<String, f64>,
}

/// Validation result for a single address
#[derive(Debug)]
struct AddressValidationResult {
    address: String,
    rust_result: Option<ComprehensiveStateChange>,
    python_result: Option<PythonStateChange>,
    token_net_match: bool,
    denom_net_match: bool,
    token_net_diff: f64,
    denom_net_diff: f64,
    overall_match: bool,
}

/// Overall test result
#[derive(Debug)]
struct ValidationTestResult {
    tx_hash: String,
    total_addresses: usize,
    matching_addresses: usize,
    rust_only_addresses: Vec<String>,
    python_only_addresses: Vec<String>,
    address_results: Vec<AddressValidationResult>,
    overall_accuracy: f64,
    success: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    if args.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }
    
    info!("🧪 Starting comprehensive state diff validation test");
    info!("📍 Transaction: {}", args.tx_hash);
    
    // Setup provider
    let provider = Arc::new(Provider::<Http>::try_from(&args.eth_rpc_url)?);
    info!("🔗 Connected to Ethereum node: {}", args.eth_rpc_url);
    
    // Fetch transaction details
    let tx_hash = args.tx_hash.parse::<H256>()?;
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| eyre::eyre!("Transaction not found"))?;
    
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| eyre::eyre!("Transaction receipt not found"))?;
    
    info!("📦 Transaction found in block: {}", receipt.block_number.unwrap_or_default());
    
    // Convert to our TransactionView format
    let tx_view = TransactionView {
        hash: tx_hash.as_bytes().to_vec(),
        from: tx.from.as_bytes().to_vec(),
        to: tx.to.map(|addr| addr.as_bytes().to_vec()),
        value: tx.value,
        gas_price: tx.gas_price,
        gas_limit: Some(tx.gas),
        nonce: Some(tx.nonce),
        input_data: Some(tx.input.to_vec()),
    };
    
    // Run our Rust implementation
    info!("🦀 Running Rust comprehensive state diff calculation...");
    let mut calculator = ComprehensiveStateDiffCalculator::new(provider.clone());
    
    let block_number = receipt.block_number.unwrap_or_default().as_u64();
    let txn_index = receipt.transaction_index.as_u64();
    
    let rust_result = calculator.calculate_state_changes(&tx_view, block_number, txn_index).await?;
    
    info!("✅ Rust calculation complete. Found {} addresses with state changes", rust_result.len());
    
    // Parse expected Python result if provided
    let python_result = if let Some(expected_json) = args.expected_python_result {
        let parsed: HashMap<String, PythonStateChange> = serde_json::from_str(&expected_json)?;
        Some(parsed)
    } else {
        info!("⚠️  No Python result provided for comparison");
        None
    };
    
    // Validate results
    let validation_result = validate_results(&args.tx_hash, rust_result, python_result);
    
    // Print detailed results
    print_validation_results(&validation_result);
    
    // Generate Python validation script
    generate_python_validation_script(&args.tx_hash, &validation_result)?;
    
    if validation_result.success {
        info!("🎉 Validation PASSED! Rust implementation matches Python results");
        Ok(())
    } else {
        error!("❌ Validation FAILED! Differences found between Rust and Python implementations");
        std::process::exit(1);
    }
}

/// Validate Rust results against Python results
fn validate_results(
    tx_hash: &str,
    rust_result: HashMap<String, ComprehensiveStateChange>,
    python_result: Option<HashMap<String, PythonStateChange>>,
) -> ValidationTestResult {
    let mut address_results = Vec::new();
    let mut matching_addresses = 0;
    let mut rust_only_addresses = Vec::new();
    let mut python_only_addresses = Vec::new();
    
    // Get all unique addresses
    let mut all_addresses = std::collections::HashSet::new();
    all_addresses.extend(rust_result.keys().cloned());
    if let Some(ref python) = python_result {
        all_addresses.extend(python.keys().cloned());
    }
    
    let total_addresses = all_addresses.len();
    let denom_tolerance = 0.0005; // Match Python denom_state_change_threshold
    let token_tolerance = 0.1;    // Match Python token_state_change_threshold
    
    for address in &all_addresses {
        let rust_change = rust_result.get(address);
        let python_change = python_result.as_ref().and_then(|p| p.get(address));
        
        let (token_net_match, denom_net_match, token_net_diff, denom_net_diff, overall_match) = 
            if let (Some(rust), Some(python)) = (rust_change, python_change) {
                let token_diff = (rust.token_net - python.token_net).abs();
                let denom_diff = (rust.denom_net - python.denom_net).abs();
                let token_match = token_diff <= token_tolerance;
                let denom_match = denom_diff <= denom_tolerance;
                let overall = token_match && denom_match;
                
                if overall {
                    matching_addresses += 1;
                }
                
                (token_match, denom_match, token_diff, denom_diff, overall)
            } else {
                if rust_change.is_some() && python_change.is_none() {
                    rust_only_addresses.push(address.clone());
                } else if rust_change.is_none() && python_change.is_some() {
                    python_only_addresses.push(address.clone());
                }
                (false, false, 0.0, 0.0, false)
            };
        
        address_results.push(AddressValidationResult {
            address: address.clone(),
            rust_result: rust_change.cloned(),
            python_result: python_change.cloned(),
            token_net_match,
            denom_net_match,
            token_net_diff,
            denom_net_diff,
            overall_match,
        });
    }
    
    let overall_accuracy = if total_addresses > 0 {
        matching_addresses as f64 / total_addresses as f64 * 100.0
    } else {
        100.0
    };
    
    let success = overall_accuracy >= 95.0 && rust_only_addresses.is_empty() && python_only_addresses.is_empty();
    
    ValidationTestResult {
        tx_hash: tx_hash.to_string(),
        total_addresses,
        matching_addresses,
        rust_only_addresses,
        python_only_addresses,
        address_results,
        overall_accuracy,
        success,
    }
}

/// Print detailed validation results
fn print_validation_results(result: &ValidationTestResult) {
    info!("📊 VALIDATION RESULTS");
    info!("==================");
    info!("Transaction: {}", result.tx_hash);
    info!("Total addresses: {}", result.total_addresses);
    info!("Matching addresses: {}", result.matching_addresses);
    info!("Overall accuracy: {:.2}%", result.overall_accuracy);
    
    if !result.rust_only_addresses.is_empty() {
        warn!("🦀 Rust-only addresses ({}): {:?}", result.rust_only_addresses.len(), result.rust_only_addresses);
    }
    
    if !result.python_only_addresses.is_empty() {
        warn!("🐍 Python-only addresses ({}): {:?}", result.python_only_addresses.len(), result.python_only_addresses);
    }
    
    info!("📋 DETAILED ADDRESS COMPARISON");
    info!("==============================");
    
    for addr_result in &result.address_results {
        if !addr_result.overall_match {
            warn!("❌ Address: {}", addr_result.address);
            
            if let Some(ref rust) = addr_result.rust_result {
                info!("  🦀 Rust - Token: {:.6}, Denom: {:.6}", rust.token_net, rust.denom_net);
            } else {
                info!("  🦀 Rust - No result");
            }
            
            if let Some(ref python) = addr_result.python_result {
                info!("  🐍 Python - Token: {:.6}, Denom: {:.6}", python.token_net, python.denom_net);
            } else {
                info!("  🐍 Python - No result");
            }
            
            if addr_result.rust_result.is_some() && addr_result.python_result.is_some() {
                info!("  📊 Differences - Token: {:.6}, Denom: {:.6}", 
                      addr_result.token_net_diff, addr_result.denom_net_diff);
            }
        } else {
            debug!("✅ Address: {} - Perfect match", addr_result.address);
        }
    }
}

/// Generate Python validation script for further testing
fn generate_python_validation_script(tx_hash: &str, _result: &ValidationTestResult) -> Result<()> {
    let script_content = format!(r#"#!/usr/bin/env python3
"""
Python validation script for transaction {}
Generated automatically by Rust comprehensive state diff test
"""

import sys
import os
sys.path.append('/home/nima/code/crypto/py/eth_block_processor')

from web3 import Web3
from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
from eth_block_processor.txn.txn_processor import TransactionProcessor

def validate_transaction():
    # Setup
    w3 = Web3(Web3.HTTPProvider('http://localhost:8545'))
    txn_data_fetcher = TransactionDataFetcher(w3)
    txn_processor = TransactionProcessor(w3, calculate_state_changes=True)
    
    # Fetch transaction data
    tx_hash = '{}'
    print(f"🐍 Processing transaction: {{tx_hash}}")
    
    try:
        txn_data = txn_data_fetcher.get_transaction_data(tx_hash)
        if not txn_data:
            print(f"❌ Transaction {{tx_hash}} not found")
            return False
            
        # Process transaction
        processed_tx = txn_processor.process_transaction(
            txn_data['transaction'], 
            txn_data['receipt'], 
            txn_data['trace']
        )
        
        # Print state changes in format compatible with Rust test
        state_changes = processed_tx.state_changes
        print(f"✅ Found state changes for {{len(state_changes)}} addresses")
        
        print("📊 PYTHON STATE CHANGES:")
        print("=" * 50)
        for address, changes in state_changes.items():
            print(f"Address: {{address}}")
            print(f"  Token Net: {{changes['token_net']:.6f}}")
            print(f"  Denom Net: {{changes['denom_net']:.6f}}")
            print(f"  Movements: {{changes['movements']}}")
            print()
        
        # Generate JSON for Rust comparison
        import json
        json_output = json.dumps(state_changes, indent=2, default=str)
        print("🔄 JSON OUTPUT FOR RUST COMPARISON:")
        print("=" * 50)
        print(json_output)
        
        return True
        
    except Exception as e:
        print(f"❌ Error processing transaction: {{e}}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = validate_transaction()
    sys.exit(0 if success else 1)
"#, tx_hash, tx_hash);

    let script_path = format!("validate_tx_{}.py", &tx_hash[2..12]); // Use first 10 chars of hash
    std::fs::write(&script_path, script_content)?;
    
    info!("📝 Generated Python validation script: {}", script_path);
    info!("💡 Run with: python3 {}", script_path);
    
    Ok(())
} 