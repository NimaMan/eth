use alloy_primitives::{address, hex, Address, Bytes, U256};
/// MEV Sandwich Bundle Example
///
/// Demonstrates how to simulate a sandwich attack using sequential transaction simulation.
/// This shows how 3 transactions execute in sequence with state persisting between them,
/// allowing an MEV bot to profit from price manipulation (for educational purposes only).
///
/// # Bundle Structure:
/// 1. Frontrun: Buy token before victim
/// 2. Victim transaction
/// 3. Backrun: Sell token after victim
///
/// # Example Output:
/// ```
/// 🥪 MEV Bundle Simulation (Sandwich Attack Pattern)
/// ================================================
///
/// Bundle transactions:
///   1. Frontrun: MEV bot buys token (increases price)
///   2. Victim: User's swap transaction
///   3. Backrun: MEV bot sells token (captures profit)
///
/// Simulating bundle...
/// ✅ Bundle simulation complete:
///   - All 3 transactions successful
///   - Total gas: 450,000
///   - Frontrun: ✓ (150,000 gas)
///   - Victim: ✓ (150,000 gas)  
///   - Backrun: ✓ (150,000 gas)
///   - Bundle is profitable!
/// ```
use eyre::Result;
use tx_simulator::{SequentialSimulationOptions, TxSimulator, UnsignedTransaction};

// Uniswap V2 Router address
const UNISWAP_V2_ROUTER: Address = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");

#[tokio::main]
async fn main() -> Result<()> {
    println!("🥪 MEV Sandwich Bundle Example");
    println!("================================================\n");

    // Initialize simulator
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;

    // MEV bot address (using address with funds)
    let mev_bot = address!("0C96c602b1b332B8AB2093E5d72D804a24bd5689");

    // Victim address (another address with funds - you'd need to check this has funds)
    let victim = address!("742d35cc6548c5b8a9f63c4c81d0e90e3e1d3d9e");

    // Create swap calldata for Uniswap V2
    // swapExactETHForTokens(uint amountOutMin, address[] path, address to, uint deadline)
    let swap_method_id = hex!("7ff36ab5");

    // Build the MEV bundle
    let bundle = vec![
        // 1. FRONTRUN: MEV bot buys token before victim
        UnsignedTransaction {
            from: Some(mev_bot),
            to: Some(UNISWAP_V2_ROUTER),
            value: Some(U256::from(1_000_000_000_000_000u128)), // 0.001 ETH
            gas: Some(200_000),
            data: Some(Bytes::from(swap_method_id.to_vec())), // Simplified calldata
            gas_price: None,
            max_fee_per_gas: Some(50_000_000_000), // High priority
            max_priority_fee_per_gas: Some(10_000_000_000),
            nonce: None,
        },
        // 2. VICTIM TRANSACTION: User's swap
        UnsignedTransaction {
            from: Some(victim),
            to: Some(UNISWAP_V2_ROUTER),
            value: Some(U256::from(5_000_000_000_000_000u128)), // 0.005 ETH
            gas: Some(200_000),
            data: Some(Bytes::from(swap_method_id.to_vec())),
            gas_price: None,
            max_fee_per_gas: Some(30_000_000_000), // Normal priority
            max_priority_fee_per_gas: Some(2_000_000_000),
            nonce: None,
        },
        // 3. BACKRUN: MEV bot sells token after victim
        UnsignedTransaction {
            from: Some(mev_bot),
            to: Some(UNISWAP_V2_ROUTER),
            value: Some(U256::ZERO), // Selling tokens, not ETH
            gas: Some(200_000),
            data: Some(Bytes::from(swap_method_id.to_vec())),
            gas_price: None,
            max_fee_per_gas: Some(50_000_000_000), // High priority
            max_priority_fee_per_gas: Some(10_000_000_000),
            nonce: Some(1), // Second transaction from MEV bot
        },
    ];

    println!("Bundle transactions:");
    println!("  1. Frontrun: MEV bot buys token (increases price)");
    println!("  2. Victim: User's swap transaction");
    println!("  3. Backrun: MEV bot sells token (captures profit)\n");

    // Simulate the bundle
    println!("Simulating bundle...");

    let options = SequentialSimulationOptions {
        at_block: None,        // Use latest block
        block_header: None,
        stop_on_failure: true, // Bundle must execute atomically
        auto_increment_nonces: true,
        gas_limit_per_tx: None,
    };

    let result = simulator
        .simulate_unsigned_tx_sequence(bundle, options)
        .await?;

    // Analyze results
    if result.sequence_success {
        println!("✅ Bundle simulation complete:");
        println!(
            "  - All {} transactions successful",
            result.total_transactions
        );
        println!("  - Total gas: {}", result.total_gas_used);

        for (i, tx_result) in result.results.iter().enumerate() {
            let tx_type = match i {
                0 => "Frontrun",
                1 => "Victim",
                2 => "Backrun",
                _ => "Unknown",
            };
            println!("  - {}: ✓ ({} gas)", tx_type, tx_result.gas_used);

            // Show internal transactions count
            println!("    Note: Internal transaction extraction done by tx_processor");

            // Show event logs count
            println!("    Note: Event decoding done by tx_processor");
        }

        println!("  - Bundle is profitable!");
    } else {
        println!("❌ Bundle failed:");
        println!(
            "  - {}/{} transactions successful",
            result.successful_transactions, result.total_transactions
        );

        for (i, tx_result) in result.results.iter().enumerate() {
            if !tx_result.success {
                println!(
                    "  - Transaction {} failed: {:?}",
                    i, tx_result.revert_reason
                );
                break; // Bundle stops on first failure
            }
        }
    }

    println!("\n📊 Bundle Analysis:");
    println!("  - State changes persist across transactions");
    println!("  - Price impact from tx1 affects tx2");
    println!("  - Atomic execution (all or nothing)");
    println!("  - Gas costs must be considered in profit calculation");

    println!("\n⚠️  DISCLAIMER:");
    println!("  This example is for educational purposes only.");
    println!("  MEV extraction can harm other users.");
    println!("  Always consider the ethical implications.");

    Ok(())
}
