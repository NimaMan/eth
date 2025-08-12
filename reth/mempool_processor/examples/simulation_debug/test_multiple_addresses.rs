/// Test Scout Token Trading with Multiple Addresses
/// 
/// This tool tests if different addresses can trade the token,
/// helping us understand the restriction pattern.

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
    
    function balanceOf(address owner) external view returns (uint256);
}

struct TestResult {
    address: Address,
    description: String,
    eth_balance: U256,
    can_buy: bool,
    error_message: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Scout Token with Multiple Addresses");
    println!("==============================================\n");
    
    let provider = ProviderBuilder::new().on_http("http://localhost:8545".parse()?);
    let token_address = Address::from_str(SCOUT_TOKEN)?;
    let router_address = Address::from_str(UNISWAP_ROUTER)?;
    let weth_address = Address::from_str(WETH)?;
    
    // Test addresses - some that likely have ETH at that block
    let test_addresses = vec![
        ("0x0C96c602b1b332B8AB2093E5d72D804a24bd5689", "Our hardcoded test address"),
        ("0xBe3569068562218C792cF25b98DBf1418AFf2455", "Real buyer from successful TX"),
        ("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", "Uniswap V2 Router"),
        ("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "WETH Contract"),
        ("0x0000000000000000000000000000000000000000", "Zero address"),
        ("0x1111111111111111111111111111111111111111", "Random address 1"),
        ("0xdEAD000000000000000042069420694206942069", "Dead address"),
        ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC Contract"),
        ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT Contract"),
        ("0x6B175474E89094C44Da98b954EedeAC495271d0F", "DAI Contract"),
    ];
    
    let mut results = Vec::new();
    
    for (addr_str, description) in test_addresses {
        let address = Address::from_str(addr_str)?;
        
        // Get ETH balance
        let eth_balance = provider.get_balance(address).block_id(BlockId::Number(TEST_BLOCK.into())).await?;
        
        // Only test if address has ETH
        let (can_buy, error_message) = if eth_balance > U256::ZERO {
            // Try a small buy
            let amount = U256::from(1_000_000_000_000_000u64); // 0.001 ETH
            let amount = amount.min(eth_balance / U256::from(2)); // Use at most half of balance
            
            let buy_call = swapExactETHForTokensCall {
                amountOutMin: U256::ZERO,
                path: vec![weth_address, token_address],
                to: address,
                deadline: U256::from(9999999999u64),
            };
            
            let tx = TransactionRequest::default()
                .from(address)
                .to(router_address)
                .value(amount)
                .input(buy_call.abi_encode().into());
            
            match provider.call(tx).block(BlockId::Number(TEST_BLOCK.into())).await {
                Ok(_) => (true, None),
                Err(e) => {
                    let error_str = e.to_string();
                    let message = if error_str.contains("TRANSFER_FAILED") {
                        Some("TRANSFER_FAILED".to_string())
                    } else if error_str.contains("INSUFFICIENT_OUTPUT_AMOUNT") {
                        Some("INSUFFICIENT_OUTPUT_AMOUNT".to_string())
                    } else if error_str.contains("execution reverted") {
                        Some("Execution reverted".to_string())
                    } else {
                        Some(error_str.chars().take(50).collect())
                    };
                    (false, message)
                }
            }
        } else {
            (false, Some("No ETH balance".to_string()))
        };
        
        results.push(TestResult {
            address,
            description: description.to_string(),
            eth_balance,
            can_buy,
            error_message,
        });
    }
    
    // Print results
    println!("📊 Test Results at Block {}:", TEST_BLOCK);
    println!("┌─────────────────────────────────────────────┬──────────────────────┬──────────────┬──────────┬─────────────────────┐");
    println!("│ Address                                     │ Description          │ ETH Balance  │ Can Buy? │ Error               │");
    println!("├─────────────────────────────────────────────┼──────────────────────┼──────────────┼──────────┼─────────────────────┤");
    
    for result in &results {
        let eth_str = format_ether(result.eth_balance);
        let error_str = result.error_message.as_ref().map(|s| s.as_str()).unwrap_or("-");
        println!("│ {:?} │ {:20} │ {:12} │ {:8} │ {:19} │",
            result.address,
            truncate(&result.description, 20),
            truncate(&eth_str, 12),
            if result.can_buy { "✅ Yes" } else { "❌ No" },
            truncate(error_str, 19)
        );
    }
    println!("└─────────────────────────────────────────────┴──────────────────────┴──────────────┴──────────┴─────────────────────┘");
    
    // Analysis
    println!("\n📈 Analysis:");
    let addresses_with_eth: Vec<_> = results.iter()
        .filter(|r| r.eth_balance > U256::ZERO)
        .collect();
    
    let successful_buyers: Vec<_> = addresses_with_eth.iter()
        .filter(|r| r.can_buy)
        .collect();
    
    let failed_buyers: Vec<_> = addresses_with_eth.iter()
        .filter(|r| !r.can_buy)
        .collect();
    
    println!("  Total addresses tested: {}", results.len());
    println!("  Addresses with ETH: {}", addresses_with_eth.len());
    println!("  Successful buyers: {}", successful_buyers.len());
    println!("  Failed buyers: {}", failed_buyers.len());
    
    if !successful_buyers.is_empty() {
        println!("\n  ✅ Addresses that CAN buy:");
        for r in &successful_buyers {
            println!("    - {} ({})", r.address, r.description);
        }
    }
    
    if !failed_buyers.is_empty() {
        println!("\n  ❌ Addresses that CANNOT buy (despite having ETH):");
        for r in &failed_buyers {
            println!("    - {} ({}) - {}", 
                r.address, 
                r.description,
                r.error_message.as_ref().unwrap_or(&"Unknown".to_string())
            );
        }
    }
    
    // Check for patterns
    println!("\n🔍 Pattern Detection:");
    
    // Check if all failures are TRANSFER_FAILED
    let transfer_failed_count = failed_buyers.iter()
        .filter(|r| r.error_message.as_ref().map(|s| s.contains("TRANSFER_FAILED")).unwrap_or(false))
        .count();
    
    if transfer_failed_count == failed_buyers.len() && !failed_buyers.is_empty() {
        println!("  ⚠️  All failures are TRANSFER_FAILED - indicates token-level restriction");
        println!("  The token contract is blocking transfers for these addresses");
    }
    
    // Check if only specific addresses can buy
    if successful_buyers.len() == 1 {
        println!("  ⚠️  Only ONE address can buy - likely owner or deployer only");
    } else if successful_buyers.len() > 0 && successful_buyers.len() < 3 {
        println!("  ⚠️  Very few addresses can buy - likely whitelisted addresses only");
    }
    
    Ok(())
}

fn format_ether(wei: U256) -> String {
    let wei_str = wei.to_string();
    let wei_f64 = wei_str.parse::<f64>().unwrap_or(0.0);
    let eth = wei_f64 / 1e18;
    format!("{:.6} ETH", eth)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len-3])
    }
}