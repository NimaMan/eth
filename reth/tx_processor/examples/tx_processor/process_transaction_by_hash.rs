use alloy_primitives::B256;
use eyre::Result;
use std::str::FromStr;
/// Process a transaction by hash and display all decoded information
///
/// This example shows how to fetch and process a transaction from the Reth database
/// using only its hash. It demonstrates the full processing pipeline including:
/// - Event decoding (ERC20 transfers, swaps, etc.)
/// - Internal transaction extraction
/// - Transaction classification
/// - Address balance changes calculation
use tx_processor::processed_tx_provider::ProcessedTxProvider;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Process Transaction by Hash Example");
    println!("=====================================\n");

    // Initialize the NEW ProcessedTxProvider that uses modular tx_simulator
    println!("Initializing ProcessedTxProvider...");
    let provider = ProcessedTxProvider::new("/home/nima/.local/share/reth/mainnet")?;
    println!("✅ Provider initialized successfully!\n");

    // Test transaction - let's use a recent simple ETH transfer
    // You can replace this with any transaction hash from your synced blocks
    let tx_hash =
        B256::from_str("0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e")?;

    println!("Fetching transaction: {}", tx_hash);
    println!("Testing user-provided transaction hash\n");

    match provider.process_transaction_by_hash(tx_hash).await {
        Ok(tx) => {
            println!("✅ Transaction Fetched and Processed Successfully!");
            println!("================================================");

            // Basic info
            println!("\n📋 Basic Information:");
            println!("  Hash: {}", tx.hash);
            println!("  Block: {}", tx.block_number);
            println!("  Timestamp: {}", tx.block_timestamp);
            println!("  From: {}", tx.from_address);
            println!(
                "  To: {:?}",
                tx.to_address
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "Contract Creation".to_string())
            );
            println!("  Value: {} wei", tx.value);
            println!("  Status: {}", tx.status);
            println!("  Type: {}", tx.tx_type);
            println!("  Input data length: {} bytes", tx.input.len());

            // Gas and Fees
            println!("\n⛽ Gas & Fees:");
            println!("  Gas Used: {}", tx.fees.gas_used);
            println!("  Gas Price: {} wei", tx.fees.gas_price);
            println!("  Transaction Fee: {} wei", tx.fees.tx_fee);

            // ERC20 Transfers
            if !tx.erc20_transfers.is_empty() {
                println!("\n📊 ERC20 Transfers: {} found", tx.erc20_transfers.len());
                for (i, transfer) in tx.erc20_transfers.iter().enumerate() {
                    println!("\n  Transfer #{}:", i + 1);
                    println!("    Token: {}", transfer.token_address);
                    println!("    From: {}", transfer.from_address);
                    println!("    To: {}", transfer.to_address);
                    println!("    Amount: {}", transfer.amount);
                    println!("    Log Index: {}", transfer.log_index);
                }
            }

            // Internal Transactions
            if !tx.internal_transactions.is_empty() {
                println!(
                    "\n🔍 Internal Transactions: {} found",
                    tx.internal_transactions.len()
                );
                for (i, internal) in tx.internal_transactions.iter().enumerate() {
                    println!("\n  Internal Tx #{}:", i + 1);
                    println!("    From: {}", internal.from_address);
                    let to_display = internal
                        .to_address
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|| "None".to_string());
                    println!("    To: {}", to_display);
                    println!("    Value: {} wei", internal.value);
                    println!("    Gas: {}", internal.gas);
                    println!("    Depth: {}", internal.depth);
                    println!("    Type: {}", internal.trace_type);
                    if let Some(call_type) = &internal.call_type {
                        println!("    Call Type: {}", call_type);
                    }
                }
            }

            // Uniswap Swaps
            if !tx.uniswap_v2_swaps.is_empty() {
                println!("\n🔄 Uniswap V2 Swaps: {} found", tx.uniswap_v2_swaps.len());
                for (i, swap) in tx.uniswap_v2_swaps.iter().enumerate() {
                    println!("\n  Swap #{}:", i + 1);
                    println!("    Pool: {}", swap.pair_address);
                    println!("    Amount0 In: {}", swap.amount0_in);
                    println!("    Amount1 In: {}", swap.amount1_in);
                    println!("    Amount0 Out: {}", swap.amount0_out);
                    println!("    Amount1 Out: {}", swap.amount1_out);
                    println!("    To: {}", swap.to);
                }
            }

            if !tx.uniswap_v3_swaps.is_empty() {
                println!("\n🔄 Uniswap V3 Swaps: {} found", tx.uniswap_v3_swaps.len());
                for (i, swap) in tx.uniswap_v3_swaps.iter().enumerate() {
                    println!("\n  Swap #{}:", i + 1);
                    println!("    Pool: {}", swap.pool_address);
                    println!("    Amount0: {}", swap.amount0);
                    println!("    Amount1: {}", swap.amount1);
                    println!("    Sqrt Price X96: {}", swap.sqrt_price_x96);
                    println!("    Liquidity: {}", swap.liquidity);
                    println!("    Tick: {}", swap.tick);
                }
            }

            // Other events
            if !tx.erc20_approval_events.is_empty() {
                println!(
                    "\n✅ ERC20 Approvals: {} found",
                    tx.erc20_approval_events.len()
                );
                for (i, approval) in tx.erc20_approval_events.iter().enumerate() {
                    println!(
                        "  Approval #{}: {} approves {} to spend {}",
                        i + 1,
                        approval.owner,
                        approval.spender,
                        approval.amount
                    );
                }
            }

            // Address Balance Changes - NEW IMPLEMENTATION!
            if !tx.address_balance_changes.is_empty() {
                println!(
                    "\n💰 Address Balance Changes: {} addresses affected",
                    tx.address_balance_changes.len()
                );
                for (address, changes) in tx.address_balance_changes.iter() {
                    println!("\n  Address: {}", address);

                    // Display currency net changes
                    if !changes.currency_net.is_empty() {
                        println!("    Currency Net Changes:");
                        for (symbol, amount) in &changes.currency_net {
                            println!("      {}: {} wei", symbol, amount);
                        }
                    }

                    if !changes.token_net.is_empty() {
                        println!("    Token Net Changes:");
                        for (token_addr, amount) in &changes.token_net {
                            println!("      {}: {}", token_addr, amount);
                        }
                    }
                }
            } else {
                println!("\n💰 No address balance changes detected");
            }

            // Summary
            println!("\n📊 Summary:");
            println!("  ✅ ERC20 Transfers: {}", tx.erc20_transfers.len());
            println!(
                "  ✅ Internal Transactions: {}",
                tx.internal_transactions.len()
            );
            println!(
                "  ✅ Address Balance Changes: {}",
                tx.address_balance_changes.len()
            );
            println!("  ✅ Unique Addresses: {}", tx.unique_addresses.len());
            println!("  ✅ Transaction Type: {}", tx.tx_type);
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            println!("\nPossible reasons:");
            println!("1. Transaction not in local Reth database");
            println!("2. Reth node hasn't synced to this block yet");
            println!("3. Invalid transaction hash");
            println!("\nDebug info: {:#?}", e);
        }
    }

    Ok(())
}
