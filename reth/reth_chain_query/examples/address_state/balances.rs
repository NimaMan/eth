/// Query ETH and token balances
/// 
/// This example shows how to:
/// 1. Query ETH balances
/// 2. Query ERC20 token balances
/// 3. Check multiple addresses
/// 4. Query at specific blocks
/// 
/// Run with: cargo run --example balances

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, U256, utils::{format_ether, format_units}};
use std::str::FromStr;

// Common token addresses on mainnet
const USDC: &str = "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const USDT: &str = "dAC17F958D2ee523a2206206994597C13D831ec7";
const WETH: &str = "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const DAI: &str = "6B175474E89094C44Da98b954EedeAC495271d0F";

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ETH and Token Balance Queries ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === ETH Balances ===
    println!("1. ETH Balances");
    println!("-" .repeat(40));
    
    // Some interesting addresses
    let addresses = vec![
        ("Vitalik", "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
        ("Ethereum Foundation", "de0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"),
        ("Binance Hot Wallet", "F977814e90dA44bFA03b6295A0616a897441aceC"),
    ];
    
    for (name, addr_str) in &addresses {
        let address = Address::from_str(addr_str)?;
        let account = provider.get_account(address, None).await?;
        
        println!("{:20} {} ETH", 
            format!("{}:", name),
            format_ether(account.balance)
        );
    }
    
    println!();
    
    // === Token Balances ===
    println!("2. Token Balances");
    println!("-" .repeat(40));
    
    // Check Vitalik's token balances
    let vitalik = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?;
    
    let tokens = vec![
        (USDC, "USDC", 6),
        (USDT, "USDT", 6),
        (WETH, "WETH", 18),
        (DAI, "DAI", 18),
    ];
    
    println!("Vitalik's token balances:");
    for (token_addr, symbol, decimals) in tokens {
        let token = Address::from_str(token_addr)?;
        let balance = provider.get_token_balance(token, vitalik, None).await?;
        
        if balance > U256::ZERO {
            let formatted = format_units(balance, decimals)?;
            println!("  {}: {}", symbol, formatted);
        }
    }
    
    println!();
    
    // === Historical Balances ===
    println!("3. Historical Balance Query");
    println!("-" .repeat(40));
    
    let address = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?;
    
    // Query at different points in Ethereum history
    let checkpoints = vec![
        (1_000_000, "Early Ethereum"),
        (10_000_000, "DeFi Summer 2020"),
        (15_537_393, "The Merge"),
        (17_000_000, "Post-Shanghai"),
    ];
    
    println!("Vitalik's balance over time:");
    for (block, label) in checkpoints {
        match provider.get_account(address, Some(block)).await {
            Ok(account) => {
                println!("  Block {:>10} ({:20}): {} ETH", 
                    block,
                    label,
                    format_ether(account.balance)
                );
            }
            Err(_) => {
                println!("  Block {:>10} ({:20}): Not synced yet", block, label);
            }
        }
    }
    
    println!();
    
    // === Check Contract vs EOA ===
    println!("4. Account Types");
    println!("-" .repeat(40));
    
    let test_addresses = vec![
        ("Vitalik (EOA)", "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
        ("USDC (Contract)", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        ("Uniswap V3 Router", "E592427A0AEce92De3Edee1F18E0157C05861564"),
    ];
    
    for (name, addr_str) in test_addresses {
        let address = Address::from_str(addr_str)?;
        let account = provider.get_account(address, None).await?;
        
        let account_type = if account.code_hash.is_some() {
            "Contract"
        } else {
            "EOA"
        };
        
        println!("{:25} {} (nonce: {})", 
            format!("{}:", name),
            account_type,
            account.nonce
        );
    }
    
    println!();
    
    // === Batch Balance Check ===
    println!("5. Batch Balance Query");
    println!("-" .repeat(40));
    
    // Get top exchange addresses
    let exchanges = vec![
        ("Binance", "F977814e90dA44bFA03b6295A0616a897441aceC"),
        ("Coinbase", "A9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
        ("Kraken", "53d284357ec70cE289D6D64134DfAc8E511c8a3D"),
    ];
    
    let mut total_exchange_eth = U256::ZERO;
    
    for (name, addr_str) in exchanges {
        let address = Address::from_str(addr_str)?;
        let account = provider.get_account(address, None).await?;
        total_exchange_eth = total_exchange_eth + account.balance;
        
        println!("{:15} {} ETH", 
            format!("{}:", name),
            format_ether(account.balance)
        );
    }
    
    println!("{:15} {} ETH", 
        "Total:",
        format_ether(total_exchange_eth)
    );
    
    println!("\n✅ Balance queries complete!");
    
    Ok(())
}