//! Complete Input/Output Documentation Example
//! 
//! This example demonstrates all three optimization modes with exact documentation
//! of inputs, outputs, and error conditions.
//!
//! ## Inputs
//! 
//! ### Command Line Arguments
//! - `tx_hash` (optional): Transaction hash in 0x-prefixed hex format
//!   - Example: `0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae`
//!   - Default: Uses the audited transaction above
//!
//! ### Configuration Options
//! ```rust
//! TransactionDataOptions {
//!     reth_datadir: Option<String>,  // Path to Reth database
//!     force_level: Option<DataLevel>, // Force specific optimization level
//!     include_logs: bool,            // Include event logs in output
//!     max_retries: u8,              // Database retry attempts
//! }
//! ```
//!
//! ## Outputs
//!
//! ### Basic Mode Output (BasicTxData)
//! ```json
//! {
//!   "hash": "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae",
//!   "from": "0xae2fc483527b8ef99eb5d9b44875f005ba1fae13",
//!   "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
//!   "value": "1000000000000000000",  // 1 ETH in wei
//!   "gas_limit": 300000,
//!   "gas_used": 120456,
//!   "gas_price": "25000000000",       // 25 Gwei
//!   "block_number": 18900000,
//!   "status": true,                   // Success
//!   "erc20_transfers": [
//!     {
//!       "token_address": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
//!       "from": "0xae2fc483527b8ef99eb5d9b44875f005ba1fae13",
//!       "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
//!       "amount": "1500000000"        // 1500 USDC (6 decimals)
//!     }
//!   ],
//!   "performance": {
//!     "retrieval_time_ms": 0.004,
//!     "data_source": "database",
//!     "optimization_applied": "Provider reused, database-only access"
//!   }
//! }
//! ```
//!
//! ### Smart Mode Additional Fields
//! ```json
//! {
//!   // All Basic fields plus:
//!   "transaction_type": "DexInteraction",
//!   "simulation_performed": true,
//!   "internal_transfers": [
//!     {
//!       "from": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
//!       "to": "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
//!       "value": "1000000000000000000"  // 1 ETH
//!     }
//!   ],
//!   "detected_operations": ["swap", "liquidity_provision"],
//!   "confidence_score": 0.95
//! }
//! ```
//!
//! ### Complete Mode Full Output
//! ```json
//! {
//!   // All Smart fields plus:
//!   "call_trace": {
//!     "type": "CALL",
//!     "from": "0xae2fc483527b8ef99eb5d9b44875f005ba1fae13",
//!     "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
//!     "value": "1000000000000000000",
//!     "gas": 300000,
//!     "gas_used": 120456,
//!     "status": "Success",
//!     "subcalls": [...]
//!   },
//!   "state_changes": {
//!     "0xae2fc483527b8ef99eb5d9b44875f005ba1fae13": {
//!       "balance_change": "-1000000000000000000",
//!       "nonce_change": 1
//!     }
//!   },
//!   "gas_breakdown": {
//!     "intrinsic": 21000,
//!     "execution": 99456,
//!     "refund": 0
//!   }
//! }
//! ```
//!
//! ## Error Conditions
//!
//! ### NotFound Error
//! ```json
//! {
//!   "error": "NotFound",
//!   "message": "Transaction 0x123... not found in database",
//!   "tx_hash": "0x123..."
//! }
//! ```
//!
//! ### DatabaseError
//! ```json
//! {
//!   "error": "DatabaseError", 
//!   "message": "Failed to open MDBX database: Permission denied",
//!   "path": "/home/nima/.local/share/reth/mainnet"
//! }
//! ```
//!
//! ### SimulationError
//! ```json
//! {
//!   "error": "SimulationError",
//!   "message": "Transaction execution reverted: ERC20: insufficient balance",
//!   "tx_hash": "0xabc...",
//!   "revert_reason": "ERC20: insufficient balance"
//! }
//! ```
//!
//! To run:
//!   cargo run --bin optimized_tx_io_docs [tx_hash]

use anyhow::Result;
use std::env;
use std::str::FromStr;
use serde_json;

use ethers_core::types::H256;
use revm_tx_simulator_lib::fetch_from_reth::optimized_tx_data::*;
use revm_tx_simulator_lib::fetch_from_reth::RethDatabaseProvider;

#[tokio::main]
async fn main() -> Result<()> {
    println!("📚 Complete Input/Output Documentation Example");
    println!("=============================================\n");

    // INPUT: Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let tx_hash_str = if args.len() > 1 {
        println!("🔹 Input: Transaction hash from command line");
        println!("  Value: {}", &args[1]);
        &args[1]
    } else {
        println!("🔹 Input: Using default transaction hash");
        let default = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
        println!("  Value: {}", default);
        default
    };

    // Validate and parse transaction hash
    let tx_hash = match H256::from_str(tx_hash_str) {
        Ok(hash) => {
            println!("  ✅ Valid transaction hash format\n");
            hash
        }
        Err(e) => {
            println!("  ❌ ERROR: Invalid transaction hash format");
            println!("  Expected: 0x-prefixed 64 character hex string");
            println!("  Got: {}", tx_hash_str);
            println!("  Error: {}", e);
            return Err(e.into());
        }
    };

    // INPUT: Configuration options
    println!("🔹 Configuration Options:");
    let basic_options = TransactionDataOptions::basic();
    println!("  Basic Mode Options:");
    println!("    - reth_datadir: {:?}", basic_options.reth_datadir);
    println!("    - force_level: {:?}", basic_options.force_level);
    println!("    - include_logs: {}", basic_options.include_logs);
    println!("    - max_retries: {}", basic_options.max_retries);

    // Create database provider for optimized access
    let datadir = "/home/nima/.local/share/reth/mainnet";
    println!("\n🔹 Database Provider Creation:");
    println!("  Path: {}", datadir);
    
    let provider = match RethDatabaseProvider::new(datadir) {
        Ok(p) => {
            println!("  ✅ Provider created successfully\n");
            p
        }
        Err(e) => {
            println!("  ❌ ERROR: Failed to create database provider");
            println!("  Error Type: DatabaseError");
            println!("  Message: {}", e);
            println!("  Possible causes:");
            println!("    - Reth node not installed");
            println!("    - Database path incorrect");
            println!("    - Permission denied");
            return Err(e.into());
        }
    };

    // EXAMPLE 1: Basic Mode
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("1️⃣  BASIC MODE (Database Only)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    match get_basic_transaction_data_with_provider(tx_hash, basic_options.clone(), Some(&provider)).await {
        Ok(basic_data) => {
            println!("✅ SUCCESS: Basic transaction data retrieved\n");
            
            // Show exact output structure
            println!("📤 Output Structure (BasicTxData):");
            println!("  Transaction Info:");
            println!("    hash: {:?}", basic_data.hash);
            println!("    from: {:?}", basic_data.from);
            println!("    to: {:?}", basic_data.to);
            println!("    value: {} wei", basic_data.value);
            
            println!("\n  Gas Info:");
            println!("    gas_limit: {}", basic_data.gas_limit);
            println!("    gas_used: {} ({:.1}% of limit)", 
                basic_data.gas_used, 
                (basic_data.gas_used as f64 / basic_data.gas_limit as f64) * 100.0);
            println!("    gas_price: {} wei", basic_data.gas_price);
            
            println!("\n  Block Context:");
            println!("    block_number: {}", basic_data.block_number);
            println!("    block_hash: {:?}", basic_data.block_hash);
            println!("    transaction_index: {}", basic_data.transaction_index);
            
            println!("\n  Execution:");
            println!("    status: {} ({})", basic_data.status, if basic_data.status { "success" } else { "failed" });
            println!("    nonce: {}", basic_data.nonce);
            println!("    input_data: {} bytes", basic_data.input_data.len());
            
            println!("\n  Events & Transfers:");
            println!("    log_count: {}", basic_data.log_count);
            println!("    erc20_transfers: {} transfers", basic_data.erc20_transfers.len());
            
            if !basic_data.erc20_transfers.is_empty() {
                println!("\n  ERC20 Transfer Details:");
                for (i, transfer) in basic_data.erc20_transfers.iter().enumerate() {
                    println!("    Transfer {}:", i + 1);
                    println!("      token: {:?}", transfer.token_address);
                    println!("      from: {:?}", transfer.from);
                    println!("      to: {:?}", transfer.to);
                    println!("      amount: {}", transfer.amount);
                }
            }
            
            println!("\n  Performance Metrics:");
            println!("    retrieval_time_ms: {:.3}", basic_data.performance.retrieval_time_ms);
            println!("    data_source: {}", basic_data.performance.data_source);
            println!("    optimization_applied: {}", basic_data.performance.optimization_applied);
            
            // Show as JSON for exact format
            println!("\n📋 JSON Representation:");
            let json = serde_json::to_string_pretty(&basic_data)?;
            println!("{}", json);
        }
        Err(e) => {
            println!("❌ ERROR in Basic Mode:");
            println!("  Error Type: {:?}", e);
            println!("  Message: {}", e);
            
            // Show error structure
            println!("\n📋 Error Response Structure:");
            println!("{{");
            println!("  \"error\": \"{:?}\",", e);
            println!("  \"message\": \"{}\",", e);
            println!("  \"tx_hash\": \"{}\"", tx_hash);
            println!("}}");
        }
    }

    // EXAMPLE 2: Smart Mode
    println!("\n\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("2️⃣  SMART MODE (Auto-Detection)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let smart_options = TransactionDataOptions::smart();
    
    match get_smart_transaction_data_with_provider(tx_hash, smart_options, Some(&provider)).await {
        Ok(smart_data) => {
            println!("✅ SUCCESS: Smart transaction data retrieved\n");
            
            println!("📤 Additional Smart Mode Fields:");
            println!("  transaction_type: {:?}", smart_data.transaction_type);
            println!("  simulation_performed: {}", smart_data.simulation_performed);
            
            if smart_data.simulation_performed {
                println!("\n  Internal Transfers: {} found", smart_data.internal_transfers.len());
                for (i, transfer) in smart_data.internal_transfers.iter().enumerate() {
                    println!("    Transfer {}:", i + 1);
                    println!("      from: {:?}", transfer.from);
                    println!("      to: {:?}", transfer.to);
                    println!("      value: {} wei", transfer.value);
                }
            }
        }
        Err(e) => {
            println!("❌ ERROR in Smart Mode:");
            println!("  Error: {}", e);
        }
    }

    // EXAMPLE 3: Complete Mode
    println!("\n\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("3️⃣  COMPLETE MODE (Full Analysis)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let complete_options = TransactionDataOptions::complete();
    
    match get_full_transaction_analysis_with_provider(tx_hash, complete_options, Some(&provider)).await {
        Ok(complete_data) => {
            println!("✅ SUCCESS: Complete transaction analysis\n");
            
            println!("📤 Additional Complete Mode Fields:");
            println!("  has_call_trace: {}", complete_data.call_trace.is_some());
            println!("  has_state_changes: {}", complete_data.state_changes.is_some());
            
            if let Some(ref state_changes) = complete_data.state_changes {
                println!("\n  State Changes: {} addresses affected", state_changes.len());
                for (addr, change) in state_changes.iter().take(3) {
                    println!("    Address: {:?}", addr);
                    println!("      balance_change: {:?}", change.balance_change);
                    println!("      nonce_change: {:?}", change.nonce_change);
                    println!("      storage_changes: {} slots", change.storage_changes.len());
                }
            }
        }
        Err(e) => {
            println!("❌ ERROR in Complete Mode:");
            println!("  Error: {}", e);
        }
    }

    println!("\n\n✅ Example completed. See exact input/output formats above.");
    
    Ok(())
}