/// Example demonstrating the MultiVenuePriceReader to fetch ETH prices from all sources
///
/// This example shows how to:
/// - Get prices from all AMMs and view-call feeds
/// - Calculate median and average prices
/// - Display price statistics
/// - Group prices by source type
use eth_price::{MultiVenuePriceReader, PriceData};
use reth_chain_query::provider_factory_from_datadir;
use std::collections::HashMap;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 ETH Price Aggregator - Fetching from All Sources\n");
    println!("{}", "=".repeat(60));

    // Initialize database connection via helper
    let start = Instant::now();
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    println!("✅ Database connected in {:?}", start.elapsed());

    // Create multi-venue reader with all sources
    let mut reader = MultiVenuePriceReader::new(provider_factory.clone());

    // Initialize all AMMs
    println!("\n📊 Initializing Price Sources...");
    reader = reader.with_all_amms(provider_factory.clone())?;
    reader = reader.with_all_oracles(provider_factory.clone())?;
    println!("✅ All price sources initialized");

    // Fetch all prices
    println!("\n💰 Fetching ETH/USD Prices from All Sources:");
    println!("{}", "-".repeat(60));

    let fetch_start = Instant::now();
    let all_prices = reader.get_all_prices_async("ETH/USD").await;

    // Display individual prices
    let mut successful_prices = Vec::new();
    for (source, result) in &all_prices {
        match result {
            Ok(price_data) => {
                print_price_line(&format!("  📈 {:<12}", source), price_data);
                successful_prices.push((source.clone(), price_data.price_as_f64()));
            }
            Err(e) => {
                println!("  ❌ {:<12} Error: {}", source, e);
            }
        }
    }

    println!("\n⏱️  Fetch time: {:?}", fetch_start.elapsed());

    // Calculate and display statistics
    if !successful_prices.is_empty() {
        println!("\n📊 Price Statistics:");
        println!("{}", "-".repeat(60));

        // Get median
        match reader.get_median_price("ETH/USD").await {
            Ok(median) => println!("  📍 Median Price:   ${:.2}", median),
            Err(e) => println!("  ❌ Median Error: {}", e),
        }

        // Get weighted average (equal weights)
        match reader.get_weighted_average("ETH/USD", None).await {
            Ok(avg) => println!("  📊 Average Price:  ${:.2}", avg),
            Err(e) => println!("  ❌ Average Error: {}", e),
        }

        // Get weighted average with custom weights (favor oracles)
        let mut weights = HashMap::new();
        weights.insert("Chainlink".to_string(), 2.0); // Give Chainlink double weight
        weights.insert("UniswapV3".to_string(), 1.5); // Give Uniswap V3 1.5x weight

        match reader.get_weighted_average("ETH/USD", Some(weights)).await {
            Ok(weighted) => println!(
                "  ⚖️  Weighted Avg:   ${:.2} (Chainlink 2x, UniV3 1.5x)",
                weighted
            ),
            Err(e) => println!("  ❌ Weighted Error: {}", e),
        }

        // Get full statistics
        match reader.get_price_stats("ETH/USD").await {
            Ok(stats) => {
                println!("\n📈 Detailed Statistics:");
                println!("  • Min Price:      ${:.2}", stats.min);
                println!("  • Max Price:      ${:.2}", stats.max);
                println!("  • Price Range:    ${:.2}", stats.max - stats.min);
                println!("  • Std Deviation:  ${:.2}", stats.std_dev);
                println!("  • Sources Count:  {}", stats.count);

                // Calculate spread percentage
                let spread_pct = ((stats.max - stats.min) / stats.avg) * 100.0;
                println!("  • Spread:         {:.2}%", spread_pct);
            }
            Err(e) => println!("  ❌ Stats Error: {}", e),
        }
    }

    // Group by type
    let (amm_prices, oracle_prices) = reader.get_prices_by_type("ETH/USD").await;

    println!("\n🏭 Prices by Source Type:");
    println!("{}", "-".repeat(60));

    if !amm_prices.is_empty() {
        println!("  AMMs ({} sources):", amm_prices.len());
        for (source, price_data) in &amm_prices {
            print_price_line(&format!("    • {}", source), price_data);
        }

        // Calculate AMM average
        let amm_avg: f64 = amm_prices
            .iter()
            .map(|(_, p)| p.price_as_f64())
            .sum::<f64>()
            / amm_prices.len() as f64;
        println!("    Average AMM:   ${:.2}", amm_avg);
    }

    if !oracle_prices.is_empty() {
        println!("\n  Oracles ({} sources):", oracle_prices.len());
        for (source, price_data) in &oracle_prices {
            print_price_line(&format!("    • {}", source), price_data);
        }
    }

    // Show arbitrage opportunity if significant spread
    if successful_prices.len() >= 2 {
        let min_price = successful_prices
            .iter()
            .map(|(_, p)| p)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_price = successful_prices
            .iter()
            .map(|(_, p)| p)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let spread = max_price - min_price;

        if spread > 10.0 {
            // More than $10 spread
            println!("\n💡 Arbitrage Alert!");
            println!("{}", "-".repeat(60));

            let min_source = successful_prices
                .iter()
                .find(|(_, p)| *p == *min_price)
                .unwrap()
                .0
                .clone();
            let max_source = successful_prices
                .iter()
                .find(|(_, p)| *p == *max_price)
                .unwrap()
                .0
                .clone();

            println!("  🔄 Potential arbitrage opportunity detected:");
            println!("    Buy at:  {} for ${:.2}", min_source, min_price);
            println!("    Sell at: {} for ${:.2}", max_source, max_price);
            println!(
                "    Spread:  ${:.2} ({:.2}%)",
                spread,
                (spread / min_price) * 100.0
            );
            println!("    ⚠️  Note: Consider gas costs and slippage");
        }
    }

    println!("\n✅ Total execution time: {:?}", start.elapsed());
    println!("{}", "=".repeat(60));

    Ok(())
}

fn print_price_line(label: &str, price_data: &PriceData) {
    println!(
        "{} ${:>10.6} (block {}, protocol {})",
        label,
        price_data.price_as_f64(),
        price_data.block_number,
        price_data.source.protocol()
    );
    println!(
        "      raw: {}/{} quote/base, inverse {}/{} base/quote",
        price_data.price.numerator,
        price_data.price.denominator,
        price_data.inverse_price.numerator,
        price_data.inverse_price.denominator
    );
}
