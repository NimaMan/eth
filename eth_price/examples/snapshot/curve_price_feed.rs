/// Curve Finance Price Feed Example
///
/// Shows ETH prices from Curve pools (stableswap and volatile pools)
/// Uses the modular tx_simulator for view function calls
use eth_price::CurveReader;
use reth_chain_query::provider_factory_from_datadir;
use reth_provider::BlockNumReader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌀 Curve Finance Price Feed - Stablecoin Optimized AMM");
    println!("{}", "=".repeat(55));

    // Create shared provider factory
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Initialize Curve reader with shared provider
    let reader = CurveReader::from_provider(provider_factory.clone())?;

    println!("\n📊 ETH Price from Curve TriCrypto2:");
    println!("{}", "-".repeat(55));

    match reader.get_latest_price("ETH/USDT_CURVE").await {
        Ok(price_data) => {
            println!(
                "  💰 ETH/USDT: {:>10.6}  block={}",
                price_data.price_as_f64(),
                price_data.block_number
            );
            println!(
                "  🧾 Base: {} ({}d) | Quote: {} ({}d)",
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
        }
        Err(e) => println!("  ❌ ETH/USDT: {}", e),
    }

    if let Ok(latest_block) = provider_factory.last_block_number() {
        println!("\n📍 Latest block: {}", latest_block);
    }

    println!("\n✅ Curve generic price feed ready");
    Ok(())
}
