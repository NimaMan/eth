/// Simple test to verify the erc20_token_buy_approve_sell_tx_simulator API works
/// 
/// Tests a single token (USDC) with the clean API

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, U256};
use tx_simulator::TxSimulator;
use tx_processor::tx_processor::TxProcessor;
use tx_processor::simulator::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Testing ERC20 Token Buy-Approve-Sell API");
    println!("============================================\n");
    
    // Initialize
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let simulator = Arc::new(TxSimulator::new(reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());
    
    // Test USDC
    let usdc_token: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse()?;
    let usdc_pool: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse()?; // USDC/WETH V2
    
    println!("Testing USDC pool...");
    println!("Token: {}", usdc_token);
    println!("Pool: {}", usdc_pool);
    
    // Configure with block delay
    let config = PoolBuySellParameters::new(
        usdc_token,
        usdc_pool,
        PoolType::UniswapV2,
    )
    .with_test_amount(U256::from(1_000_000_000_000_000_000u128)) // 1 ETH
    .with_token_decimals(6)  // USDC has 6 decimals
    .with_block_delay(1);     // Sell in next block
    
    println!("\nRunning simulation with:");
    println!("  Test amount: 1 ETH");
    println!("  Block delay: 1 (sell in next block)");
    println!("  Token decimals: 6\n");
    
    // Run analysis
    match check_can_buy_sell_pool(simulator, tx_processor, config).await {
        Ok(result) => {
            println!("✅ Analysis Complete!\n");
            
            println!("Trading Status:");
            println!("  Can Buy: {}", if result.can_buy { "✅" } else { "❌" });
            println!("  Can Approve: {}", if result.can_approve { "✅" } else { "❌" });
            println!("  Can Sell: {}", if result.can_sell { "✅" } else { "❌" });
            println!("  Is Tradeable: {}", if result.is_tradeable { "✅" } else { "❌" });
            
            if result.is_tradeable {
                println!("\n💰 Tax Analysis:");
                println!("  Buy Tax: {:.2}%", result.buy_tax_percent);
                println!("  Sell Tax: {:.2}%", result.sell_tax_percent);
                
                println!("\n📊 Trade Details:");
                println!("  ETH Spent: {} wei", result.eth_spent);
                println!("  USDC Received: {} (raw with 6 decimals)", result.tokens_received);
                
                // Convert to human readable
                let usdc_amount = result.tokens_received / U256::from(10u64.pow(6));
                println!("  USDC Received: {} USDC", usdc_amount);
                
                println!("  ETH Recovered: {} wei", result.eth_received);
                
                let loss = result.eth_spent.saturating_sub(result.eth_received);
                let loss_percent = if result.eth_spent > U256::ZERO {
                    (loss.to::<u128>() as f64 / result.eth_spent.to::<u128>() as f64) * 100.0
                } else {
                    0.0
                };
                
                println!("\n📈 Summary:");
                println!("  Net Loss: {} wei ({:.2}%)", loss, loss_percent);
                println!("  Block Used: {}", result.block_number);
                
                println!("\n🎉 API is working correctly! The module successfully:");
                println!("  - Simulated buy transaction");
                println!("  - Extracted exact token amounts");
                println!("  - Simulated approve transaction");
                println!("  - Simulated sell in next block");
                println!("  - Calculated taxes from state changes");
            } else {
                println!("\n❌ Trading Failed!");
                if let Some(reason) = &result.failure_reason {
                    println!("Reason: {}", reason);
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
    
    Ok(())
}
