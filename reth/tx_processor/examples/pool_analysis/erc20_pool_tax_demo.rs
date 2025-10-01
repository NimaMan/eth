use alloy_primitives::Address;
/// Test the pool viability analyzer with RFI (Reflect Finance) token
///
/// This example tests the erc20_token_buy_approve_sell_tx_simulator module
/// by analyzing RFI which has a confirmed 1% transaction fee on all transfers.
/// This verifies our tax calculation logic works correctly with fee-on-transfer tokens.
use eyre::Result;
use std::sync::Arc;
use tx_processor::simulator::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment - skip dotenv/env_logger if not available
    // dotenv::dotenv().ok();
    // env_logger::init();

    println!("Testing Tax Calculation with RFI Token");
    println!("======================================");
    println!("RFI (Reflect Finance) has a confirmed 1% fee on all transactions");
    println!("This verifies our tax calculation logic works with actual fee-on-transfer tokens.");

    // Get reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    // Create simulator and tx processor
    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    // RFI (Reflect Finance) token - has 1% fee on all transactions
    let token_address = Address::from([
        0xa1, 0xaf, 0xff, 0xe3, 0xf4, 0xd6, 0x11, 0xd2, 0x52, 0x01, 0x0e, 0x3e, 0xaf, 0x6f, 0x4d,
        0x77, 0x08, 0x8b, 0x0c, 0xd7,
    ]); // RFI token: 0xa1afffe3f4d611d252010e3eaf6f4d77088b0cd7

    let pool_address = Address::from([
        0xb9, 0xca, 0x9f, 0x21, 0x36, 0x67, 0xff, 0xd2, 0x21, 0xf0, 0x78, 0xec, 0xf3, 0xa7, 0x2d,
        0xaf, 0xe0, 0x4d, 0x45, 0xab,
    ]); // RFI/WETH Uniswap V2 pool: 0xb9ca9f213667ffd221f078ecf3a72dafe04d45ab

    // Create configuration with block delay and specific block number
    let config = PoolBuySellParameters::new(token_address, pool_address, PoolType::UniswapV2)
        .with_test_amount(alloy_primitives::U256::from(1_000_000_000_000_000_000u128)) // 1 ETH for better testing
        .with_block_delay(1) // Prefer next block when available; capped to latest
        .with_token_decimals(9); // RFI has 9 decimals

    println!();
    println!("Configuration:");
    println!("  Token: RFI (Reflect Finance) - {:?}", token_address);
    println!("  Pool: RFI/WETH Uniswap V2 - {:?}", pool_address);
    println!("  Type: {:?}", config.pool_type);
    println!("  Test Amount: {} wei (1 ETH)", config.test_amount);
    println!("  Token Decimals: 9");
    println!("  Expected Tax: 1% on all transactions (buy and sell)");
    println!();

    // Run the analysis
    println!("Running pool viability analysis...");
    let result = match check_can_buy_sell_pool(simulator, tx_processor, config).await {
        Ok(r) => r,
        Err(e) => {
            println!("Error: {}", e);
            return Ok(());
        }
    };

    // Display results
    println!("\nAnalysis Results:");
    println!("=================");
    println!("Pool Type: {:?}", result.pool_type);
    println!("Is Tradeable: {}", result.is_tradeable);
    println!("Block Number: {}", result.block_number);
    println!();

    if result.is_tradeable {
        println!("Tax Analysis (Expected: ~1% each for RFI):");

        // Display buy tax
        if result.buy_tax_percent < 0.0 {
            println!("  Buy Tax: N/A (transaction failed)");
        } else {
            println!(
                "  Buy Tax: {:.2}%{}",
                result.buy_tax_percent,
                if (result.buy_tax_percent - 1.0).abs() < 0.5 {
                    " ✅ Close to expected 1%"
                } else {
                    " ⚠️ Differs from expected 1%"
                }
            );
        }

        // Display sell tax
        if result.sell_tax_percent < 0.0 {
            println!("  Sell Tax: N/A (transaction failed)");
        } else {
            println!(
                "  Sell Tax: {:.2}%{}",
                result.sell_tax_percent,
                if (result.sell_tax_percent - 1.0).abs() < 0.5 {
                    " ✅ Close to expected 1%"
                } else {
                    " ⚠️ Differs from expected 1%"
                }
            );
        }
        println!();

        println!("Trade Details:");
        println!("  ETH Spent: {} wei", result.eth_spent);
        println!("  Tokens Received: {}", result.tokens_received);
        println!("  ETH Received: {} wei", result.eth_received);

        let net_loss = result.eth_spent.saturating_sub(result.eth_received);
        let loss_percent = if result.eth_spent > alloy_primitives::U256::ZERO {
            (net_loss.to_string().parse::<f64>().unwrap_or(0.0)
                / result.eth_spent.to_string().parse::<f64>().unwrap_or(1.0))
                * 100.0
        } else {
            0.0
        };

        println!("  Net Loss: {} wei ({:.2}%)", net_loss, loss_percent);
    } else {
        println!("Trading failed!");
        if let Some(reason) = &result.failure_reason {
            println!("Failure reason: {}", reason);
        }
    }

    println!("\nTransaction Details:");
    println!(
        "  Buy TX: {} ({})",
        result.buy_transaction.hash,
        if result.buy_transaction.status == "1" {
            "Success"
        } else {
            "Failed"
        }
    );
    println!(
        "  Approve TX: {} ({})",
        result.approve_transaction.hash,
        if result.approve_transaction.status == "1" {
            "Success"
        } else {
            "Failed"
        }
    );
    println!(
        "  Sell TX: {} ({})",
        result.sell_transaction.hash,
        if result.sell_transaction.status == "1" {
            "Success"
        } else {
            "Failed"
        }
    );

    if let Some(prior_tx) = &result.prior_transaction {
        println!(
            "  Prior TX: {} ({})",
            prior_tx.hash,
            if prior_tx.status == "1" {
                "Success"
            } else {
                "Failed"
            }
        );
    }

    Ok(())
}
