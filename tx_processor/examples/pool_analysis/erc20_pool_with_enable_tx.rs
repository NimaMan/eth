use alloy_primitives::{Address, B256, U256};
/// Example: Analyze token that requires enable trading transaction
///
/// This example shows how to analyze a token that needs a prior transaction
/// (like enable trading) before buy/sell can occur.
use eyre::Result;
use std::sync::Arc;
use tx_processor::trade_simulation::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Token Trading Viability with Enable Trading TX");
    println!("==============================================");

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    // Create simulator and tx processor
    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    // Example: Get the enable trading transaction first
    // In a real scenario, you'd fetch this from the blockchain
    let enable_tx_hash: B256 = "0x1234...".parse().unwrap_or(B256::ZERO);

    // You would fetch the actual transaction like this:
    // let prior_tx = tx_processor.process_transaction_by_hash(enable_tx_hash).await?;

    // For demo, we'll proceed without a prior tx
    let token_address: Address = "0x6982508145454Ce325dDbE47a25d4ec3d2311933".parse()?;
    let pool_address: Address = "0xA43fe16908251ee70EF74718545e4FE6C5cCEc9f".parse()?;
    let denom_address: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse()?; // WETH

    // Configure with prior transaction and block delay
    let mut config = PoolBuySellParameters::new(token_address, pool_address, PoolType::UniswapV2)
        .with_test_amount(U256::from(1_000_000_000_000_000_000u128)) // 1 ETH
        .with_denom_address(denom_address)
        .with_denom_decimals(18)
        .with_token_decimals(18)
        .with_block_delay(1); // Sell in next block

    // If you had the prior tx:
    // config = config.with_prior_tx(prior_tx);

    println!("Analyzing token that may require enable trading...");

    match check_can_buy_sell_pool(simulator, tx_processor, config).await {
        Ok(result) => {
            println!("\nResults:");
            println!("  Tradeable: {}", result.is_tradeable);

            if !result.is_tradeable {
                println!("  Failure: {:?}", result.failure_reason);
                println!("  Can Buy: {}", result.can_buy);
                println!("  Can Approve: {}", result.can_approve);
                println!("  Can Sell: {}", result.can_sell);
                println!("\nThis token likely requires an enable trading transaction first.");
                println!("Steps to analyze:");
                println!("1. Find the enable trading transaction hash");
                println!("2. Process it with tx_processor.process_transaction_by_hash()");
                println!("3. Pass it to config.with_prior_tx()");
                println!("4. Re-run the analysis");
            } else {
                println!("  Buy Tax: {:.2}%", result.buy_tax_percent);
                println!("  Sell Tax: {:.2}%", result.sell_tax_percent);
                println!("  Tokens Received: {}", result.tokens_received);
                println!("  WETH Recovered: {} wei", result.denom_received);
                println!("  Block Delay Used: 1 (sell in next block)");
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    Ok(())
}
