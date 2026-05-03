use alloy_primitives::{utils::format_ether, Address, U256};
/// Batch operations for efficient multi-address and multi-token queries
///
/// This example demonstrates:
/// 1. batch_get_eth_balances - Multiple addresses ETH balances
/// 2. batch_get_token_balances - Multiple token-holder pairs  
/// 3. batch_get_portfolio_balances - One address, multiple tokens
/// 4. Performance comparison vs sequential queries
///
/// Run with: cargo run --example batch_operations
use reth_chain_query::{Result, RethQueryProvider};
use std::str::FromStr;
use std::time::Instant;

// Well-known addresses for testing
const EXCHANGES: &[(&str, &str)] = &[
    ("Binance", "F977814e90dA44bFA03b6295A0616a897441aceC"),
    ("Coinbase", "A9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
    ("Kraken", "53d284357ec70cE289D6D64134DfAc8E511c8a3D"),
    ("OKX", "98EC059Dc3aDFBdd63ce1e1d64F55754E87FD5Aa"),
    ("Bitfinex", "876EabF441B2EE5B5b0554Fd502a8E0600950cFa"),
];

// Major tokens to check
const TOKENS: &[(&str, &str)] = &[
    ("USDC", "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
    ("USDT", "dAC17F958D2ee523a2206206994597C13D831ec7"),
    ("WETH", "C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
];

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Batch Operations Examples ===\n");

    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let provider = RethQueryProvider::new(&reth_datadir)?;

    // === 1. Batch ETH Balances ===
    println!("1. Batch ETH Balance Query");
    println!("{}", "-".repeat(60));

    let exchange_addresses: Vec<Address> = EXCHANGES
        .iter()
        .map(|(_, addr)| Address::from_str(addr).unwrap())
        .collect();

    println!("Fetching ETH balances for {} exchanges...", EXCHANGES.len());
    let start = Instant::now();

    let eth_balances = provider
        .get_eth_balances_for_multiple_addresses(exchange_addresses.clone(), None)
        .await?;

    let batch_time = start.elapsed();

    println!("\nExchange ETH Holdings:");
    let mut total_eth = U256::ZERO;
    for ((name, _), balance) in EXCHANGES.iter().zip(eth_balances.iter()) {
        println!(
            "  {:12} {} ETH",
            format!("{}:", name),
            format_ether(*balance)
        );
        total_eth = total_eth + *balance;
    }
    println!("  {:12} {} ETH", "Total:", format_ether(total_eth));
    println!("\nBatch query time: {:?}", batch_time);

    // Compare with sequential
    println!("\nComparing with sequential queries...");
    let start = Instant::now();
    for addr in &exchange_addresses[..3] {
        // Just first 3 for comparison
        let _ = provider.get_account(*addr, None).await?;
    }
    let seq_time = start.elapsed();
    println!("Sequential time (3 addresses): {:?}", seq_time);
    println!(
        "Batch is {:.1}x faster per address",
        seq_time.as_millis() as f64 / batch_time.as_millis() as f64 * EXCHANGES.len() as f64 / 3.0
    );

    println!();

    // === 2. Batch Token Balances (Multiple Pairs) ===
    println!("2. Batch Token Balance Query (Multiple Pairs)");
    println!("{}", "-".repeat(60));

    // Check USDC balance for all exchanges
    let usdc = Address::from_str(TOKENS[0].1)?;
    let token_holder_pairs: Vec<(Address, Address)> = exchange_addresses
        .iter()
        .map(|&addr| (usdc, addr))
        .collect();

    println!("Fetching USDC balances for all exchanges...");
    let start = Instant::now();

    let usdc_balances = provider
        .batch_get_balances_for_token_holder_pairs(token_holder_pairs, None)
        .await?;

    println!("Query time: {:?}\n", start.elapsed());

    println!("Exchange USDC Holdings:");
    for ((name, _), balance) in EXCHANGES.iter().zip(usdc_balances.iter()) {
        if *balance > U256::ZERO {
            let formatted = alloy_primitives::utils::format_units(*balance, 6)?;
            println!("  {:12} {} USDC", format!("{}:", name), formatted);
        }
    }

    println!();

    // === 3. Portfolio Balances (One Address, Multiple Tokens) ===
    println!("3. Batch Portfolio Query");
    println!("{}", "-".repeat(60));

    let binance = Address::from_str(EXCHANGES[0].1)?;
    let token_addresses: Vec<Address> = TOKENS
        .iter()
        .map(|(_, addr)| Address::from_str(addr).unwrap())
        .collect();

    println!("Fetching all token balances for Binance...");
    let start = Instant::now();

    let portfolio = provider
        .get_multiple_token_balances_for_address(binance, token_addresses, None)
        .await?;

    println!("Query time: {:?}\n", start.elapsed());

    println!("Binance Token Holdings:");
    for (token_addr, balance) in portfolio {
        if let Some((symbol, _)) = TOKENS
            .iter()
            .find(|(_, addr)| Address::from_str(addr).unwrap() == token_addr)
        {
            let decimals = match symbol {
                &"USDC" | &"USDT" => 6,
                _ => 18,
            };
            let formatted = alloy_primitives::utils::format_units(balance, decimals)?;
            println!("  {}: {}", symbol, formatted);
        }
    }

    println!();

    // === 4. Batch Operations at Historical Block ===
    println!("4. Historical Batch Query");
    println!("{}", "-".repeat(60));

    let block = 17_000_000; // Post-Shanghai
    println!("Fetching balances at block {}...", block);

    let historical_balances = provider
        .get_eth_balances_for_multiple_addresses(exchange_addresses[..3].to_vec(), Some(block))
        .await?;

    println!("\nExchange ETH at block {}:", block);
    for ((name, _), balance) in EXCHANGES[..3].iter().zip(historical_balances.iter()) {
        println!(
            "  {:12} {} ETH",
            format!("{}:", name),
            format_ether(*balance)
        );
    }

    println!();

    // === 5. Batch Nonce Query ===
    println!("5. Batch Nonce Query");
    println!("{}", "-".repeat(60));

    println!("Fetching nonces for all exchanges...");
    let nonces = provider
        .get_transaction_counts_for_multiple_addresses(exchange_addresses.clone(), None)
        .await?;

    println!("\nExchange Transaction Counts:");
    for (name, addr_str) in EXCHANGES {
        let address = Address::from_str(addr_str)?;
        if let Some(nonce) = nonces.get(&address) {
            println!("  {:12} {} transactions", format!("{}:", name), nonce);
        }
    }

    println!("\n✅ Batch operations complete!");
    println!("\nKey Insights:");
    println!("- Batch operations are significantly faster than sequential queries");
    println!("- Use get_eth_balances_for_multiple_addresses for multiple addresses");
    println!("- Use batch_get_balances_for_token_holder_pairs for specific token-holder pairs");
    println!("- Use get_multiple_token_balances_for_address for one address, multiple tokens");

    Ok(())
}
