use alloy_primitives::{utils::format_ether, utils::format_units, Address, U256};
/// Historical balance analysis across Ethereum milestones
///
/// This example demonstrates:
/// 1. get_historical_balance - Query balances at specific blocks
/// 2. Time-series balance tracking
/// 3. Balance evolution over key Ethereum events
/// 4. Token balance history
///
/// Run with: cargo run --example historical_analysis
use reth_chain_query::{Result, RethQueryProvider};
use std::str::FromStr;

// Key Ethereum milestones
const MILESTONES: &[(&str, u64)] = &[
    ("Genesis", 1),
    ("The DAO Hack", 1_920_000),
    ("Byzantium Fork", 4_370_000),
    ("Constantinople", 7_280_000),
    ("Istanbul Fork", 9_069_000),
    ("Berlin Fork", 12_244_000),
    ("London Fork (EIP-1559)", 12_965_000),
    ("The Merge", 15_537_393),
    ("Shanghai Upgrade", 17_034_870),
    ("Cancun Upgrade", 19_426_587),
];

// Tokens for historical analysis
const TOKENS: &[(&str, &str, u8, u64)] = &[
    (
        "USDC",
        "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        6,
        6_445_813,
    ), // USDC launch block
    (
        "USDT",
        "dAC17F958D2ee523a2206206994597C13D831ec7",
        6,
        4_634_748,
    ), // USDT launch block
    (
        "DAI",
        "6B175474E89094C44Da98b954EedeAC495271d0F",
        18,
        8_928_158,
    ), // DAI launch block
];

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Historical Balance Analysis ===\n");

    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let provider = RethQueryProvider::new(&reth_datadir)?;
    let latest_block = provider.get_latest_block()?;

    // === 1. ETH Balance Through History ===
    println!("1. ETH Balance at Ethereum Milestones");
    println!("{}", "-".repeat(70));

    // Ethereum Foundation address
    let eth_foundation = Address::from_str("de0B295669a9FD93d5F28D9Ec85E40f4cb697BAe")?;

    println!("Ethereum Foundation Balance History:\n");
    println!("{:<25} {:>12} {:>20}", "Milestone", "Block", "ETH Balance");
    println!("{}", "-".repeat(70));

    for (name, block) in MILESTONES {
        if *block > latest_block {
            println!("{:<25} {:>12} {:>20}", name, block, "Not synced yet");
            continue;
        }

        match provider
            .get_eth_or_token_balance_at_block(eth_foundation, None, *block)
            .await
        {
            Ok(balance) => {
                println!(
                    "{:<25} {:>12} {:>20}",
                    name,
                    format!("#{}", block),
                    format!("{} ETH", format_ether(balance))
                );
            }
            Err(_) => {
                println!("{:<25} {:>12} {:>20}", name, block, "Error");
            }
        }
    }

    println!();

    // === 2. Token Balance Evolution ===
    println!("2. Token Balance History");
    println!("{}", "-".repeat(70));

    // Track Vitalik's stablecoin holdings
    let vitalik = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?;

    println!("Vitalik's Stablecoin Holdings Over Time:\n");

    for (symbol, token_addr, decimals, launch_block) in TOKENS {
        let token = Address::from_str(token_addr)?;

        println!("{}:", symbol);
        println!("  Launch Block: #{}", launch_block);

        // Check balance at key points after token launch
        let checkpoints = MILESTONES
            .iter()
            .filter(|(_, block)| *block > *launch_block && *block <= latest_block)
            .take(5); // Last 5 milestones

        for (milestone, block) in checkpoints {
            match provider
                .get_eth_or_token_balance_at_block(vitalik, Some(token), *block)
                .await
            {
                Ok(balance) => {
                    if balance > U256::ZERO {
                        println!(
                            "  {:20} {} {}",
                            milestone,
                            format_units(balance, *decimals)?,
                            symbol
                        );
                    } else {
                        println!("  {:20} 0 {}", milestone, symbol);
                    }
                }
                Err(_) => {
                    println!("  {:20} Error", milestone);
                }
            }
        }
        println!();
    }

    // === 3. Exchange Balance Timeline ===
    println!("3. Exchange Balance Timeline");
    println!("{}", "-".repeat(70));

    let binance = Address::from_str("F977814e90dA44bFA03b6295A0616a897441aceC")?;

    println!("Binance Hot Wallet ETH Balance Evolution:\n");

    // Monthly snapshots for the last year
    let blocks_per_month = 30 * 24 * 60 * 5; // ~30 days worth of blocks
    let months = 12;

    println!(
        "{:<15} {:>12} {:>20} {:>15}",
        "Time Ago", "Block", "ETH Balance", "Change"
    );
    println!("{}", "-".repeat(70));

    let mut previous_balance: Option<U256> = None;

    for i in (0..=months).rev() {
        let block = latest_block.saturating_sub(i as u64 * blocks_per_month);

        match provider
            .get_eth_or_token_balance_at_block(binance, None, block)
            .await
        {
            Ok(balance) => {
                let time_label = if i == 0 {
                    "Current".to_string()
                } else {
                    format!("{} months ago", i)
                };

                let change_str = if let Some(prev) = previous_balance {
                    if balance > prev {
                        format!("+{} ETH", format_ether(balance - prev))
                    } else if balance < prev {
                        format!("-{} ETH", format_ether(prev - balance))
                    } else {
                        "No change".to_string()
                    }
                } else {
                    "-".to_string()
                };

                println!(
                    "{:<15} {:>12} {:>20} {:>15}",
                    time_label,
                    format!("#{}", block),
                    format!("{} ETH", format_ether(balance)),
                    change_str
                );

                previous_balance = Some(balance);
            }
            Err(_) => {
                println!(
                    "{:<15} {:>12} {:>20} {:>15}",
                    format!("{} months ago", i),
                    format!("#{}", block),
                    "Error",
                    "-"
                );
            }
        }
    }

    println!();

    // === 4. Multi-Asset Historical Portfolio ===
    println!("4. Historical Portfolio Value");
    println!("{}", "-".repeat(70));

    println!("Building historical portfolio for analysis...\n");

    let address = eth_foundation;
    let checkpoints = vec![
        (latest_block - 365 * 7200, "1 Year Ago"),
        (latest_block - 180 * 7200, "6 Months Ago"),
        (latest_block - 90 * 7200, "3 Months Ago"),
        (latest_block - 30 * 7200, "1 Month Ago"),
        (latest_block, "Current"),
    ];

    println!("Ethereum Foundation Portfolio Evolution:\n");
    println!(
        "{:<15} {:>20} {:>15}",
        "Period", "ETH Balance", "USDC Balance"
    );
    println!("{}", "-".repeat(50));

    let usdc = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;

    for (block, label) in checkpoints {
        let eth_balance = provider
            .get_eth_or_token_balance_at_block(address, None, block)
            .await?;
        let usdc_balance = provider
            .get_eth_or_token_balance_at_block(address, Some(usdc), block)
            .await?;

        println!(
            "{:<15} {:>20} {:>15}",
            label,
            format!("{} ETH", format_ether(eth_balance)),
            format!("{} USDC", format_units(usdc_balance, 6)?)
        );
    }

    println!();

    // === 5. Balance at Specific Timestamp ===
    println!("5. Balance Query Flexibility");
    println!("{}", "-".repeat(70));

    println!("Demonstrating flexible historical queries:\n");

    let test_address = vitalik;

    // Show both ETH and token queries
    println!("Using get_historical_balance with different parameters:\n");

    let block = latest_block - 100;

    // ETH balance (token = None)
    let eth_balance = provider
        .get_eth_or_token_balance_at_block(test_address, None, block)
        .await?;
    println!("ETH at block {}:  {} ETH", block, format_ether(eth_balance));

    // USDC balance (token = Some(usdc))
    let usdc_balance = provider
        .get_eth_or_token_balance_at_block(test_address, Some(usdc), block)
        .await?;
    println!(
        "USDC at block {}: {} USDC",
        block,
        format_units(usdc_balance, 6)?
    );

    println!("\n✅ Historical analysis complete!");
    println!("\nKey Insights:");
    println!("- Query any balance at any historical block");
    println!("- Track portfolio evolution through major events");
    println!("- Analyze long-term accumulation/distribution patterns");
    println!("- get_eth_or_token_balance_at_block works for both ETH and tokens");

    Ok(())
}
