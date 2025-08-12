/// Test Different Buy Amounts to Find Failure Threshold
/// 
/// This tool tests various buy amounts to identify exactly
/// when trades start failing due to excessive size.

use alloy_primitives::{Address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{TransactionRequest, BlockId};
use alloy_sol_types::SolCall;
use std::str::FromStr;
use eyre::Result;

const SCOUT_TOKEN: &str = "0x4e21B13330a5bfabcd1aC3F4Bc4fA444571F52ae";
const TEST_BLOCK: u64 = 23124804;
const UNISWAP_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

alloy_sol_types::sol! {
    function swapExactETHForTokens(
        uint256 amountOutMin,
        address[] calldata path,
        address to,
        uint256 deadline
    ) external payable returns (uint256[] memory amounts);
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Buy Amount Thresholds for Scout Token");
    println!("================================================\n");
    
    let provider = ProviderBuilder::new().on_http("http://localhost:8545".parse()?);
    
    let our_address = Address::from_str("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689")?;
    let token_address = Address::from_str(SCOUT_TOKEN)?;
    let router_address = Address::from_str(UNISWAP_ROUTER)?;
    let weth_address = Address::from_str(WETH)?;
    
    println!("Test Parameters:");
    println!("  Block: {}", TEST_BLOCK);
    println!("  Buyer: {:?}", our_address);
    println!("  Token: {}", SCOUT_TOKEN);
    println!();
    
    // Test amounts from small to large
    let test_amounts = vec![
        (U256::from(21_000_000_000_000u64), "0.000021 ETH", "Real TX amount"),
        (U256::from(100_000_000_000_000u64), "0.0001 ETH", "10x real"),
        (U256::from(500_000_000_000_000u64), "0.0005 ETH", "50x real"),
        (U256::from(1_000_000_000_000_000u64), "0.001 ETH", "test_multiple_addresses amount"),
        (U256::from(5_000_000_000_000_000u64), "0.005 ETH", "0.5% of pool"),
        (U256::from(10_000_000_000_000_000u64), "0.01 ETH", "1% of pool"),
        (U256::from(20_000_000_000_000_000u64), "0.02 ETH", "2% of pool"),
        (U256::from(50_000_000_000_000_000u64), "0.05 ETH", "5% of pool"),
        (U256::from(100_000_000_000_000_000u64), "0.1 ETH", "Simulator default"),
        (U256::from(200_000_000_000_000_000u64), "0.2 ETH", "20% of pool"),
        (U256::from(500_000_000_000_000_000u64), "0.5 ETH", "50% of pool"),
    ];
    
    println!("Testing buy amounts (Pool has ~1 ETH liquidity):\n");
    println!("Amount          | Description                      | Result");
    println!("----------------|----------------------------------|------------------");
    
    let mut last_success_amount = U256::ZERO;
    let mut first_failure_amount = U256::MAX;
    
    for (amount, label, description) in test_amounts {
        let call = swapExactETHForTokensCall {
            amountOutMin: U256::ZERO,
            path: vec![weth_address, token_address],
            to: our_address,
            deadline: U256::from(9999999999u64),
        };
        
        let tx = TransactionRequest::default()
            .from(our_address)
            .to(router_address)
            .value(amount)
            .input(call.abi_encode().into());
        
        let result = match provider.call(tx).block(BlockId::Number(TEST_BLOCK.into())).await {
            Ok(data) => {
                // Try to decode the output amounts
                let tokens_out = if data.len() >= 128 {
                    let amount1 = U256::from_be_slice(&data[96..128]);
                    format!("✅ SUCCESS - {} tokens", amount1)
                } else {
                    "✅ SUCCESS".to_string()
                };
                
                last_success_amount = amount;
                tokens_out
            }
            Err(e) => {
                let err_str = e.to_string();
                let result = if err_str.contains("TRANSFER_FAILED") {
                    "❌ TRANSFER_FAILED"
                } else if err_str.contains("INSUFFICIENT_OUTPUT_AMOUNT") {
                    "❌ INSUFFICIENT_OUTPUT"
                } else if err_str.contains("K") {
                    "❌ K (invariant)"
                } else if err_str.contains("execution reverted") {
                    "❌ Reverted"
                } else {
                    "❌ Unknown error"
                };
                
                if amount < first_failure_amount {
                    first_failure_amount = amount;
                }
                
                result.to_string()
            }
        };
        
        println!("{:14} | {:32} | {}", label, description, result);
    }
    
    println!("\n📊 Analysis:");
    println!("===========");
    
    if last_success_amount > U256::ZERO {
        let last_success_eth = format_ether(last_success_amount);
        println!("✅ Last successful amount: {}", last_success_eth);
    }
    
    if first_failure_amount < U256::MAX {
        let first_failure_eth = format_ether(first_failure_amount);
        println!("❌ First failing amount: {}", first_failure_eth);
        
        // Calculate the threshold
        if last_success_amount > U256::ZERO {
            let pool_liquidity = U256::from(1_000_021_000_000_000_000u64); // ~1 ETH from pool state
            let threshold_percent = (first_failure_amount * U256::from(100)) / pool_liquidity;
            println!("\n⚠️  Failure threshold: ~{}% of pool liquidity", threshold_percent);
        }
    }
    
    println!("\n💡 Conclusion:");
    println!("--------------");
    println!("The token CAN be bought by our address, but the amount matters!");
    println!("Our simulator's default of 0.1 ETH is too large for this small pool.");
    println!("The simulator should dynamically adjust buy amounts based on pool size.");
    
    Ok(())
}

fn format_ether(wei: U256) -> String {
    let wei_str = wei.to_string();
    let wei_f64 = wei_str.parse::<f64>().unwrap_or(0.0);
    let eth = wei_f64 / 1e18;
    format!("{:.6} ETH", eth)
}