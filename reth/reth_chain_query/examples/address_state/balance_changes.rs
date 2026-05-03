use alloy_primitives::{utils::format_ether, utils::format_units, Address, U256};
use reth_chain_query::provider::BalanceDiff;
/// Track balance changes between blocks
///
/// This example demonstrates:
/// 1. get_balance_changes - Track ETH and token changes
/// 2. Identifying deposits and withdrawals
/// 3. Multi-token change tracking
/// 4. Exchange flow monitoring
///
/// Run with: cargo run --example balance_changes
use reth_chain_query::{Result, RethQueryProvider};
use std::str::FromStr;

// Tokens to track
const TOKENS: &[(&str, &str, u8)] = &[
    ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
    ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7", 6),
    ("WETH", "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", 18),
];

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Balance Change Tracking ===\n");

    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let provider = RethQueryProvider::new(&reth_datadir)?;
    let latest_block = provider.get_latest_block()?;

    // === 1. Single Address Balance Changes ===
    println!("1. Address Balance Changes");
    println!("{}", "-".repeat(60));

    // Track Binance hot wallet changes over recent blocks
    let binance = Address::from_str("F977814e90dA44bFA03b6295A0616a897441aceC")?;
    let from_block = latest_block - 1000; // Last ~3.3 hours
    let to_block = latest_block;

    println!(
        "Tracking Binance changes from block {} to {}",
        from_block, to_block
    );
    println!(
        "Time period: ~{:.1} hours\n",
        (to_block - from_block) as f64 * 12.0 / 3600.0
    );

    let token_addresses: Vec<Address> = TOKENS
        .iter()
        .map(|(_, addr, _)| Address::from_str(addr).unwrap())
        .collect();

    let changes = provider
        .calculate_eth_and_token_balance_diff_between_blocks(
            binance,
            token_addresses.clone(),
            from_block,
            to_block,
        )
        .await?;

    // Display ETH changes
    println!("ETH Balance Changes:");
    println!("  Before: {} ETH", format_ether(changes.eth_change.before));
    println!("  After:  {} ETH", format_ether(changes.eth_change.after));

    match &changes.eth_change.difference {
        BalanceDiff::Increase(amount) => {
            println!("  Change: +{} ETH ✅", format_ether(*amount));
        }
        BalanceDiff::Decrease(amount) => {
            println!("  Change: -{} ETH ❌", format_ether(*amount));
        }
    }

    println!();

    // Display token changes
    if !changes.token_changes.is_empty() {
        println!("Token Balance Changes:");

        for (token_addr, change) in &changes.token_changes {
            if let Some((symbol, _, decimals)) = TOKENS
                .iter()
                .find(|(_, addr, _)| Address::from_str(addr).unwrap() == *token_addr)
            {
                println!("\n  {}:", symbol);
                println!("    Before: {}", format_units(change.before, *decimals)?);
                println!("    After:  {}", format_units(change.after, *decimals)?);

                match &change.difference {
                    BalanceDiff::Increase(amount) => {
                        println!("    Change: +{} ✅", format_units(*amount, *decimals)?);
                    }
                    BalanceDiff::Decrease(amount) => {
                        println!("    Change: -{} ❌", format_units(*amount, *decimals)?);
                    }
                }
            }
        }
    }

    println!();

    // === 2. Multiple Exchange Comparison ===
    println!("2. Exchange Flow Comparison");
    println!("{}", "-".repeat(60));

    let exchanges = vec![
        ("Binance", "F977814e90dA44bFA03b6295A0616a897441aceC"),
        ("Coinbase", "A9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
        ("Kraken", "53d284357ec70cE289D6D64134DfAc8E511c8a3D"),
    ];

    println!("Comparing ETH flows over last 100 blocks (~20 minutes):\n");
    let recent_from = latest_block - 100;

    for (name, addr_str) in &exchanges {
        let address = Address::from_str(addr_str)?;

        let changes = provider
            .calculate_eth_and_token_balance_diff_between_blocks(
                address,
                vec![], // Only ETH for this comparison
                recent_from,
                latest_block,
            )
            .await?;

        print!("{:10} ", format!("{}:", name));

        match &changes.eth_change.difference {
            BalanceDiff::Increase(amount) => {
                println!("+{:>12} ETH ⬆️", format_ether(*amount));
            }
            BalanceDiff::Decrease(amount) => {
                println!("-{:>12} ETH ⬇️", format_ether(*amount));
            }
        }
    }

    println!();

    // === 3. Historical Balance Evolution ===
    println!("3. Balance Evolution Over Time");
    println!("{}", "-".repeat(60));

    let address = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?; // Vitalik

    println!("Tracking Vitalik's ETH balance evolution:\n");

    // Define checkpoints (roughly every 1000 blocks)
    let checkpoints = vec![
        latest_block - 3000,
        latest_block - 2000,
        latest_block - 1000,
        latest_block,
    ];

    let mut previous_balance = None;

    for &block in &checkpoints {
        let account = provider.get_account(address, Some(block)).await?;
        let balance = account.balance;

        print!("Block {:8}: {:>15} ETH", block, format_ether(balance));

        if let Some(prev) = previous_balance {
            if balance > prev {
                let diff = balance - prev;
                print!(" (+{} ETH)", format_ether(diff));
            } else if balance < prev {
                let diff = prev - balance;
                print!(" (-{} ETH)", format_ether(diff));
            } else {
                print!(" (unchanged)");
            }
        }

        println!();
        previous_balance = Some(balance);
    }

    println!();

    // === 4. Net Flow Analysis ===
    println!("4. Net Flow Summary");
    println!("{}", "-".repeat(60));

    println!("Calculating net flows for major exchanges (last 500 blocks):\n");

    let flow_from = latest_block - 500;
    let mut total_inflow = U256::ZERO;
    let mut total_outflow = U256::ZERO;

    for (name, addr_str) in &exchanges {
        let address = Address::from_str(addr_str)?;

        let changes = provider
            .calculate_eth_and_token_balance_diff_between_blocks(
                address,
                vec![],
                flow_from,
                latest_block,
            )
            .await?;

        match &changes.eth_change.difference {
            BalanceDiff::Increase(amount) => {
                total_inflow = total_inflow + *amount;
                println!("{:10} Net inflow:  +{} ETH", name, format_ether(*amount));
            }
            BalanceDiff::Decrease(amount) => {
                total_outflow = total_outflow + *amount;
                println!("{:10} Net outflow: -{} ETH", name, format_ether(*amount));
            }
        }
    }

    println!("\nAggregate Flow:");
    println!("  Total Inflow:  {} ETH", format_ether(total_inflow));
    println!("  Total Outflow: {} ETH", format_ether(total_outflow));

    if total_inflow > total_outflow {
        println!(
            "  Net Result:    +{} ETH (Accumulation)",
            format_ether(total_inflow - total_outflow)
        );
    } else {
        println!(
            "  Net Result:    -{} ETH (Distribution)",
            format_ether(total_outflow - total_inflow)
        );
    }

    println!("\n✅ Balance change tracking complete!");
    println!("\nKey Insights:");
    println!("- Track balance changes between any two blocks");
    println!("- Monitor exchange inflows and outflows");
    println!("- Analyze token movements alongside ETH");
    println!("- Useful for whale watching and flow analysis");

    Ok(())
}
