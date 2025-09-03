/// Monitor centralized exchange holdings
/// 
/// This example shows how to:
/// 1. Track CEX wallet balances
/// 2. Monitor exchange holdings
/// 3. Analyze CEX dominance
/// 4. Track fund movements
/// 
/// Run with: cargo run --example cex_monitoring

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, U256, utils::format_ether};
use std::str::FromStr;

// Known CEX addresses (simplified list)
const CEX_WALLETS: &[(&str, &str)] = &[
    ("Binance Hot", "F977814e90dA44bFA03b6295A0616a897441aceC"),
    ("Binance Cold", "28C6c06298d514Db089934071355E5743bf21d60"),
    ("Coinbase", "A9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
    ("Kraken", "53d284357ec70cE289D6D64134DfAc8E511c8a3D"),
    ("Bitfinex", "C61b9BB3A7a0767E0C829db25bAda34Fc69D22dc"),
    ("OKX", "06959153B974D5BdD3506d77303d7305462b5f96"),
    ("Crypto.com", "6262998Ced004146417bEF0F61C905FECBF223fa"),
    ("KuCoin", "D6216fC19DB775Df9774a6E33526131dA7D19a2c"),
];

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== CEX Holdings Monitor ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === Current CEX Holdings ===
    println!("1. Current Exchange Holdings");
    println!("-" .repeat(60));
    
    let mut total_cex_eth = U256::ZERO;
    let mut exchange_balances = Vec::new();
    
    for (name, addr_str) in CEX_WALLETS {
        let address = Address::from_str(addr_str)?;
        let account = provider.get_account(address, None).await?;
        
        total_cex_eth = total_cex_eth + account.balance;
        exchange_balances.push((name, account.balance));
        
        println!("{:20} {} ETH", 
            format!("{}:", name),
            format_ether(account.balance)
        );
    }
    
    println!("{:20} {} ETH", 
        "TOTAL:",
        format_ether(total_cex_eth)
    );
    
    // Calculate percentage of ETH supply
    // ETH total supply is approximately 120M ETH
    let eth_supply = U256::from(120_000_000u64) * U256::from(10u64.pow(18));
    let cex_percentage = (total_cex_eth * U256::from(10000) / eth_supply).to::<u64>() as f64 / 100.0;
    
    println!("\nCEX Control: {:.2}% of ETH supply", cex_percentage);
    
    println!();
    
    // === Exchange Rankings ===
    println!("2. Exchange Rankings by ETH Holdings");
    println!("-" .repeat(60));
    
    // Sort by balance
    exchange_balances.sort_by(|a, b| b.1.cmp(&a.1));
    
    for (i, (name, balance)) in exchange_balances.iter().enumerate() {
        let percentage = (*balance * U256::from(10000) / total_cex_eth).to::<u64>() as f64 / 100.0;
        println!("#{:2}. {:20} {} ETH ({:.1}%)", 
            i + 1,
            name,
            format_ether(*balance),
            percentage
        );
    }
    
    println!();
    
    // === Historical Comparison ===
    println!("3. Historical CEX Holdings");
    println!("-" .repeat(60));
    
    // Compare holdings at different points
    let checkpoints = vec![
        (15_537_393, "The Merge"),
        (17_000_000, "Post-Shanghai"),
        (None, "Current"),
    ];
    
    for (block, label) in checkpoints {
        let mut checkpoint_total = U256::ZERO;
        
        for (_, addr_str) in CEX_WALLETS.iter().take(3) { // Just top 3 for speed
            let address = Address::from_str(addr_str)?;
            match provider.get_account(address, block).await {
                Ok(account) => checkpoint_total = checkpoint_total + account.balance,
                Err(_) => continue,
            }
        }
        
        println!("{:20} {} ETH (top 3 exchanges)", 
            format!("{}:", label),
            format_ether(checkpoint_total)
        );
    }
    
    println!();
    
    // === Stablecoin Holdings ===
    println!("4. CEX Stablecoin Holdings");
    println!("-" .repeat(60));
    
    let usdc = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    let usdt = Address::from_str("dAC17F958D2ee523a2206206994597C13D831ec7")?;
    
    let mut total_usdc = U256::ZERO;
    let mut total_usdt = U256::ZERO;
    
    println!("Top 3 Exchange Stablecoin Holdings:");
    for (name, addr_str) in CEX_WALLETS.iter().take(3) {
        let address = Address::from_str(addr_str)?;
        
        let usdc_balance = provider.get_token_balance(usdc, address, None).await?;
        let usdt_balance = provider.get_token_balance(usdt, address, None).await?;
        
        total_usdc = total_usdc + usdc_balance;
        total_usdt = total_usdt + usdt_balance;
        
        if usdc_balance > U256::ZERO || usdt_balance > U256::ZERO {
            println!("  {}:", name);
            if usdc_balance > U256::ZERO {
                println!("    USDC: ${}", usdc_balance / U256::from(10u64.pow(6)));
            }
            if usdt_balance > U256::ZERO {
                println!("    USDT: ${}", usdt_balance / U256::from(10u64.pow(6)));
            }
        }
    }
    
    println!("\nTotal Stablecoins on Top 3 CEXs:");
    println!("  USDC: ${}", total_usdc / U256::from(10u64.pow(6)));
    println!("  USDT: ${}", total_usdt / U256::from(10u64.pow(6)));
    
    println!();
    
    // === Risk Metrics ===
    println!("5. Centralization Risk Metrics");
    println!("-" .repeat(60));
    
    // Calculate concentration metrics
    let top_exchange = exchange_balances[0].1;
    let top_3_total = exchange_balances.iter().take(3).map(|(_, b)| *b).sum::<U256>();
    
    let top_1_dominance = (top_exchange * U256::from(10000) / total_cex_eth).to::<u64>() as f64 / 100.0;
    let top_3_dominance = (top_3_total * U256::from(10000) / total_cex_eth).to::<u64>() as f64 / 100.0;
    
    println!("Concentration Metrics:");
    println!("  Largest exchange holds: {:.1}% of CEX ETH", top_1_dominance);
    println!("  Top 3 exchanges hold: {:.1}% of CEX ETH", top_3_dominance);
    println!("  Total CEX ETH: {} ETH", format_ether(total_cex_eth));
    println!("  Number of tracked exchanges: {}", CEX_WALLETS.len());
    
    // Risk assessment
    println!("\nRisk Assessment:");
    if top_1_dominance > 50.0 {
        println!("  ⚠️  HIGH RISK: Single exchange controls majority");
    } else if top_3_dominance > 80.0 {
        println!("  ⚠️  MEDIUM RISK: High concentration in top 3");
    } else {
        println!("  ✅ LOW RISK: Reasonable distribution");
    }
    
    println!("\n✅ CEX monitoring complete!");
    
    Ok(())
}