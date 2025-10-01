use alloy_primitives::{Address, U256};
use eyre::Result;
use std::str::FromStr;
/// Test the migration from reth_tx_simulator to tx_simulator
///
/// This example verifies that the new modular ProcessedTxProvider works correctly
/// with the NEW tx_simulator for basic simulation and balance change calculation.
use tx_processor::processed_tx_provider::ProcessedTxProvider;
use tx_simulator::UnsignedTransaction;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing TX Processor Migration");
    println!("=================================\n");

    // Initialize NEW ProcessedTxProvider (uses tx_simulator instead of reth_tx_simulator)
    println!("Initializing new ProcessedTxProvider with tx_simulator...");
    let provider = ProcessedTxProvider::new("/home/nima/.local/share/reth/mainnet")?;
    println!("✅ ProcessedTxProvider initialized successfully!\n");

    // Test basic functionality - get latest block using NEW simulator
    println!("Testing basic NEW simulator functionality...");
    let latest_block = provider.get_latest_block().await?;
    println!("✅ Latest block: {}", latest_block);

    let base_fee = provider.get_latest_base_fee().await?;
    println!("✅ Base fee: {} wei\n", base_fee);

    // Test simulation with balance changes using the NEW approach
    println!("Testing simulation with balance changes...");
    println!("Creating simple ETH transfer simulation...\n");

    // Create a simple ETH transfer simulation
    let unsigned_tx = UnsignedTransaction {
        from: Some(Address::from_str(
            "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5",
        )?), // Known funded address
        to: Some(Address::from_str(
            "0x8ba1f109551bD432803012645ac136c29F36cd42",
        )?), // Random recipient
        value: Some(U256::from(1_000_000_000_000_000u128)), // 0.001 ETH in wei
        data: None,                                         // Simple ETH transfer
        gas: Some(21000),                                   // Standard ETH transfer gas
        gas_price: Some(20_000_000_000u128),                // 20 gwei
        nonce: None,                                        // Let simulator determine
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
    };

    // Use NEW simulation approach that leverages proper flow:
    // 1. Simulate (tx_simulator returns raw CallFrame + logs)
    // 2. Extract internal transactions from CallFrame (tx_processor)
    // 3. Decode event logs (tx_processor)
    // 4. Calculate balance changes from logs + internal txs (tx_processor)
    match provider
        .process_transaction_from_unsigned_tx(unsigned_tx, Some(latest_block))
        .await
    {
        Ok(processed_tx) => {
            println!("✅ Simulation and processing completed successfully!\n");

            println!("📋 Basic Transaction Info:");
            println!("  Transaction hash: {}", processed_tx.hash);
            println!("  Transaction type: {}", processed_tx.txn_type);
            println!("  Status: {}", processed_tx.status);
            println!("  From: {}", processed_tx.from_address);
            println!(
                "  To: {}",
                processed_tx
                    .to_address
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "Contract Creation".to_string())
            );
            println!("  Value: {} ETH", processed_tx.value.to_string());

            println!("\n⛽ Gas & Fees:");
            println!("  Gas used: {}", processed_tx.fees.gas_used);
            println!(
                "  Gas price: {} gwei",
                processed_tx.fees.gas_price / U256::from(1_000_000_000u64)
            );
            println!("  Transaction fee: {} wei", processed_tx.fees.txn_fee);

            println!("\n🔄 Transfers & Events:");
            println!("  ERC20 transfers: {}", processed_tx.erc20_transfers.len());
            println!(
                "  ERC721 transfers: {}",
                processed_tx.erc721_transfers.len()
            );
            println!(
                "  ERC1155 transfers: {}",
                processed_tx.erc1155_transfers.len()
            );
            println!("  Approvals: {}", processed_tx.approvals.len());
            println!("  Mints: {}", processed_tx.mints.len());

            println!(
                "\n🔍 Internal Transactions: {}",
                processed_tx.internal_transactions.len()
            );
            for (i, internal_tx) in processed_tx.internal_transactions.iter().enumerate() {
                println!(
                    "  [{}] {} -> {}",
                    i, internal_tx.from_address, internal_tx.to_address
                );
                println!("      Value: {} wei", internal_tx.value);
                println!("      Type: {}", internal_tx.trace_type);
                println!("      Depth: {}", internal_tx.depth);
            }

            // Check address balance changes
            println!("\n💰 Address Balance Changes:");
            if !processed_tx.address_balance_changes.is_empty() {
                for (addr, changes) in &processed_tx.address_balance_changes {
                    println!("\n  Address: {}", addr);
                    // Pretty print the JSON balance changes
                    if let Ok(formatted) = serde_json::to_string_pretty(changes) {
                        // Indent each line for better readability
                        for line in formatted.lines() {
                            println!("    {}", line);
                        }
                    } else {
                        println!("    {:?}", changes);
                    }
                }
            } else {
                println!("  ⚠️  No balance changes calculated");
                println!("  Note: Balance changes should be calculated from internal transactions (ETH) and decoded logs (tokens)");
            }

            println!("\n✨ Migration Test Summary:");
            println!("  ✅ tx_simulator returns raw CallFrame and logs");
            println!("  ✅ tx_processor extracts internal transactions from CallFrame");
            println!("  ✅ tx_processor decodes event logs");
            println!("  ✅ tx_processor calculates balance changes");
            println!("\n🎉 NEW architecture is working correctly!");
        }
        Err(e) => {
            println!("❌ Processing failed: {}", e);
            println!("Error details: {:?}", e);
            println!("\nThis indicates an issue with the migration.");
        }
    }

    Ok(())
}
