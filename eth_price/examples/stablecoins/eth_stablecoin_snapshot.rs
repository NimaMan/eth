/// Fetch ETH prices quoted in USDC, USDT, and DAI by reading pool state directly.
///
/// Notes
/// - Uses `StablecoinPriceReader`, which stitches together the snapshot readers for Uniswap V2/V3,
///   SushiSwap, and Curve.
/// - Coverage differs per stablecoin: ETH/USDC is widely supported; ETH/USDT and especially ETH/DAI
///   may be available on fewer venues (e.g., ETH/DAI commonly via SushiSwap; stablecoin-stable pairs
///   like DAI/USDC exist on Uniswap V3 and Curve but this example does not derive multi-hop prices).
use eth_price::stablecoins::StablecoinPriceReader;
use reth_chain_query::provider_factory_from_datadir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Shared provider factory from local reth datadir
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Initialize the stablecoin reader (AMMs only)
    let reader = StablecoinPriceReader::from_provider(provider_factory.clone())?;

    // Helper to fetch prices for a generic pair using the mapper
    // Collect ETH prices in USDC, USDT, and DAI
    let prices = reader.eth_stablecoin_prices().await;

    // Pretty print as JSON-like output
    println!("\nETH Prices by Stablecoin (per protocol):\n");
    for (stable, map) in &prices {
        println!("{}:", stable);
        if map.is_empty() {
            println!("  (no direct venues mapped for {})", stable);
        } else {
            for (source, price_data) in map {
                println!(
                    "  - {:<16}: {:.6} (block {})",
                    source,
                    price_data.price_as_f64(),
                    price_data.block_number
                );
                println!(
                    "      raw: {}/{} quote/base, inverse {}/{} base/quote",
                    price_data.price.numerator,
                    price_data.price.denominator,
                    price_data.inverse_price.numerator,
                    price_data.inverse_price.denominator
                );
            }
        }
    }

    // Optional hint for DAI coverage
    if prices.get("DAI").map(|m| m.is_empty()).unwrap_or(true) {
        eprintln!(
            "\nNote: Direct ETH/DAI venues are limited; tried deriving via ETH/USDC and DAI/USDC where possible."
        );
    }

    Ok(())
}
