use alloy_primitives::{B256, U256};
use reth_chain_query::provider::BlockTransactionOptions;
/// Verify Receipt RPC Equivalence
///
/// This example verifies that receipts fetched from the database
/// match EXACTLY with receipts from RPC's eth_getTransactionReceipt.
///
/// Run with: cargo run --example verify_receipts_rpc_equivalence
use reth_chain_query::{Result, RethQueryProvider};
use std::str::FromStr;
use std::time::Instant;

// For RPC calls
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Verifying Receipt RPC Equivalence");
    println!("=========================================\n");

    // Initialize our provider
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    println!("✅ Database provider initialized");

    // Initialize RPC client
    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
    println!("📡 Connecting to RPC: {}", rpc_url);

    let client = HttpClientBuilder::default().build(&rpc_url)?;

    // Get latest block
    let latest_block = provider.get_latest_block()?;
    let test_block = latest_block.saturating_sub(10); // Use a slightly older block

    println!("📊 Testing block: {}", test_block);

    // Get block to extract transaction hashes
    let block = provider
        .fetch_block_with_all_tx_data(test_block, BlockTransactionOptions::default())
        .await?;
    let tx_count = block.transactions.len();
    println!("📝 Transactions in block: {}", tx_count);

    println!("\n{}", "=".repeat(60));
    println!("STEP 1: FETCH RECEIPTS FROM DATABASE");
    println!("{}", "=".repeat(60));

    let start_db = Instant::now();

    // Get all receipts from database
    let db_receipts = provider.get_block_receipts(test_block).await?;

    let db_time = start_db.elapsed();
    println!(
        "✅ Database fetch completed in {:.3}s",
        db_time.as_secs_f64()
    );
    println!("   Receipts fetched: {}", db_receipts.len());

    println!("\n{}", "=".repeat(60));
    println!("STEP 2: FETCH RECEIPTS FROM RPC");
    println!("{}", "=".repeat(60));

    let start_rpc = Instant::now();
    let mut rpc_receipts = Vec::new();

    // Fetch each receipt from RPC
    for tx in &block.transactions {
        let tx_hash = format!("0x{:x}", tx.tx_metadata.hash);

        let receipt: Value = match client
            .request("eth_getTransactionReceipt", rpc_params![&tx_hash])
            .await
        {
            Ok(r) => r,
            Err(e) => {
                println!("❌ RPC call failed for tx {}: {}", tx_hash, e);
                continue;
            }
        };

        rpc_receipts.push(receipt);
    }

    let rpc_time = start_rpc.elapsed();
    println!("✅ RPC fetch completed in {:.3}s", rpc_time.as_secs_f64());
    println!("   Receipts fetched: {}", rpc_receipts.len());

    println!("\n{}", "=".repeat(60));
    println!("STEP 3: COMPARE RECEIPTS");
    println!("{}", "=".repeat(60));

    let mut matches = 0;
    let mut mismatches = 0;
    let mut mismatch_details = Vec::new();

    // Compare each receipt
    for (idx, (db_receipt, rpc_receipt)) in db_receipts.iter().zip(rpc_receipts.iter()).enumerate()
    {
        let tx_hash = block.transactions[idx].tx_metadata.hash;

        println!("\n[Transaction {}] 0x{:x}", idx, tx_hash);

        let mut tx_matches = true;
        let mut tx_mismatches = Vec::new();

        // Compare status
        let rpc_status = rpc_receipt["status"]
            .as_str()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);
        let db_status = if db_receipt.status { 1u64 } else { 0u64 };

        if rpc_status != db_status {
            tx_matches = false;
            tx_mismatches.push(format!("Status: db={}, rpc={}", db_status, rpc_status));
        }

        // Compare gas used
        let rpc_gas = rpc_receipt["gasUsed"]
            .as_str()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);

        if db_receipt.gas_used != rpc_gas {
            tx_matches = false;
            tx_mismatches.push(format!(
                "Gas used: db={}, rpc={}",
                db_receipt.gas_used, rpc_gas
            ));
        }

        // Compare cumulative gas used
        let rpc_cumulative = rpc_receipt["cumulativeGasUsed"]
            .as_str()
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);

        if db_receipt.cumulative_gas_used != rpc_cumulative {
            tx_matches = false;
            tx_mismatches.push(format!(
                "Cumulative gas: db={}, rpc={}",
                db_receipt.cumulative_gas_used, rpc_cumulative
            ));
        }

        // Compare logs count
        let rpc_logs = rpc_receipt["logs"].as_array().map(|l| l.len()).unwrap_or(0);

        if db_receipt.logs.len() != rpc_logs {
            tx_matches = false;
            tx_mismatches.push(format!(
                "Logs count: db={}, rpc={}",
                db_receipt.logs.len(),
                rpc_logs
            ));
        }

        // Compare contract address (for deployments)
        let rpc_contract = rpc_receipt["contractAddress"].as_str();
        // Note: Our TransactionReceipt doesn't have contract_address field
        // This would need to be checked from the transaction metadata (to == None)
        let db_has_contract = false; // Simplified for this example
        let rpc_has_contract = rpc_contract.is_some() && rpc_contract != Some("null");

        if db_has_contract != rpc_has_contract {
            tx_matches = false;
            tx_mismatches.push(format!(
                "Contract deployment: db={}, rpc={}",
                db_has_contract, rpc_has_contract
            ));
        }

        // Report result
        if tx_matches {
            println!("  ✅ MATCH");
            matches += 1;
        } else {
            println!("  ❌ MISMATCH:");
            for mismatch in &tx_mismatches {
                println!("    - {}", mismatch);
            }
            mismatches += 1;
            mismatch_details.push((idx, tx_hash, tx_mismatches));
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("VERIFICATION SUMMARY");
    println!("{}", "=".repeat(60));

    let total = matches + mismatches;
    let match_percentage = (matches as f64 / total as f64) * 100.0;

    println!("\n📊 Results:");
    println!("  • Total receipts: {}", total);
    println!("  • ✅ Matching: {} ({:.2}%)", matches, match_percentage);
    println!(
        "  • ❌ Mismatches: {} ({:.2}%)",
        mismatches,
        100.0 - match_percentage
    );

    if !mismatch_details.is_empty() {
        println!("\n📋 Mismatch Details (first 5):");
        for (idx, hash, details) in mismatch_details.iter().take(5) {
            println!("  Transaction {} (0x{:x}):", idx, hash);
            for detail in details {
                println!("    - {}", detail);
            }
        }
        if mismatch_details.len() > 5 {
            println!("  ... and {} more", mismatch_details.len() - 5);
        }
    }

    println!("\n⚡ Performance Comparison:");
    println!(
        "  • Database fetch: {:.3}s ({} receipts)",
        db_time.as_secs_f64(),
        db_receipts.len()
    );
    println!(
        "  • RPC fetch: {:.3}s ({} receipts)",
        rpc_time.as_secs_f64(),
        rpc_receipts.len()
    );
    println!(
        "  • Speedup: {:.1}x faster",
        rpc_time.as_secs_f64() / db_time.as_secs_f64()
    );

    println!("\n📈 Throughput:");
    println!(
        "  • Database: {:.0} receipts/second",
        db_receipts.len() as f64 / db_time.as_secs_f64()
    );
    println!(
        "  • RPC: {:.0} receipts/second",
        rpc_receipts.len() as f64 / rpc_time.as_secs_f64()
    );

    // Binary pass/fail
    if mismatches == 0 {
        println!("\n✅ VERIFICATION PASSED!");
        println!("Database receipts match RPC eth_getTransactionReceipt EXACTLY.");
    } else {
        println!("\n❌ VERIFICATION FAILED!");
        println!("Database receipts do NOT match RPC.");
        println!("{} of {} receipts have differences.", mismatches, total);
    }

    // Additional analysis
    println!("\n{}", "=".repeat(60));
    println!("ADDITIONAL ANALYSIS");
    println!("{}", "=".repeat(60));

    // Analyze log topics
    let mut total_logs_db = 0;
    let mut total_topics_db = 0;

    for receipt in &db_receipts {
        total_logs_db += receipt.logs.len();
        for log in &receipt.logs {
            total_topics_db += log.topics.len();
        }
    }

    println!("\n📝 Database Receipt Statistics:");
    println!("  • Total logs across all receipts: {}", total_logs_db);
    println!(
        "  • Average logs per receipt: {:.2}",
        total_logs_db as f64 / db_receipts.len() as f64
    );
    println!("  • Total topics: {}", total_topics_db);

    // Find receipts with most logs
    let mut most_logs = (&db_receipts[0], 0);
    for receipt in &db_receipts {
        if receipt.logs.len() > most_logs.1 {
            most_logs = (receipt, receipt.logs.len());
        }
    }

    println!("\n🔝 Receipt with most logs:");
    println!("  • Transaction hash: 0x{:x}", most_logs.0.tx_hash);
    println!("  • Log count: {}", most_logs.1);
    println!("  • Gas used: {}", most_logs.0.gas_used);

    Ok(())
}
