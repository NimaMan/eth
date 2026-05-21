/// DODO Price Feed Example
///
/// Shows ETH price from DODO pools (Proactive Market Maker - PMM)
/// Uses the modular tx_simulator and shared provider pattern
use eth_price::{DODOReader, PriceSource};
use reth_chain_query::provider_factory_from_datadir;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦤 DODO Price Feed - Proactive Market Maker");
    println!("{}", "=".repeat(50));

    // Create shared provider factory
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Initialize DODO reader with shared provider
    let reader = DODOReader::from_provider(provider_factory)?;

    println!("\n📊 ETH Price from DODO:");
    println!("{}", "-".repeat(50));

    let start = Instant::now();
    match reader.get_latest_price("ETH/USD").await {
        Ok(price_data) => {
            let query_time = start.elapsed();
            println!(
                "  💰 ETH/USD: ${:.2} ({:.3}ms)",
                price_data.price_as_f64(),
                query_time.as_secs_f64() * 1000.0
            );
            println!(
                "  📍 Block: {} | Base: {} ({}d) | Quote: {} ({}d)",
                price_data.block_number,
                price_data.pair.base.symbol,
                price_data.pair.base.decimals,
                price_data.pair.quote.symbol,
                price_data.pair.quote.decimals
            );
            println!(
                "  🔢 Raw: {}/{} quote/base, inverse {}/{} base/quote",
                price_data.price.numerator,
                price_data.price.denominator,
                price_data.inverse_price.numerator,
                price_data.inverse_price.denominator
            );

            if let PriceSource::Amm(ref source) = price_data.source {
                println!("  🏊 Pool: {:#x}", source.pool.address);
            }
        }
        Err(e) => {
            let query_time = start.elapsed();
            println!(
                "  ❌ Error fetching price: {} ({:.3}ms)",
                e,
                query_time.as_secs_f64() * 1000.0
            );
        }
    }

    println!("\n✅ DODO price feed working with tx_simulator!");
    Ok(())
}
