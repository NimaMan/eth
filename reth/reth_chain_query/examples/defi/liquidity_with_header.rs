//! Liquidity checks that reuse a freshly-fetched block header.
//!
//! This example demonstrates how callers that already hold the latest sealed
//! header (e.g. live pipelines) can pass it through to view-call helpers so we
//! avoid re-reading metadata from MDBX. We fetch a header once, reuse it for a
//! couple of token metadata calls, and still run the broader liquidity reader
//! to show the data matches.

use alloy_primitives::address;
use eyre::Result;
use reth_chain_query::{AmmSwapRoute, RethQueryProvider};
use reth_primitives::SealedHeader;
use reth_provider::HeaderProvider;

fn default_reth_db() -> String {
    std::env::var("RETH_DB_PATH").unwrap_or_else(|_| {
        format!(
            "{}/.local/share/reth/mainnet",
            std::env::var("HOME").unwrap_or_else(|_| "/home/nima".into())
        )
    })
}

fn fetch_sealed_header(provider: &RethQueryProvider, block: u64) -> Result<SealedHeader> {
    let header_provider = provider.provider_factory().provider()?;
    let header = header_provider
        .header_by_number(block)?
        .ok_or_else(|| eyre::eyre!("Missing header for block {block}"))?;
    Ok(SealedHeader::new_unhashed(header))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("📦 Liquidity snapshot with cached block header");
    println!("{}", "=".repeat(72));

    let reth_db = default_reth_db();
    let provider = RethQueryProvider::new(&reth_db)?;
    let latest_block = provider.get_latest_block()?;
    println!("Reth DB: {reth_db}");
    println!("Latest block: {latest_block}");

    // Fetch the header once; live callers would already have this in memory.
    let sealed_header = fetch_sealed_header(&provider, latest_block)?;
    println!(
        "Header gas_limit={} timestamp={}",
        sealed_header.header().gas_limit,
        sealed_header.header().timestamp
    );

    // Known pools/tokens for verification.
    let uni_v2_usdc_weth = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
    let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

    // Run the standard liquidity reader (no header required for the state lookups).
    let route = AmmSwapRoute::UniswapV2 {
        pool: uni_v2_usdc_weth,
    };
    let liquidity = provider
        .get_route_liquidity(&route, Some(latest_block))
        .await?;
    println!(
        "UniswapV2 reserves @{} → reserve0={} reserve1={}",
        liquidity.block_number,
        liquidity.reserve0.unwrap_or_default(),
        liquidity.reserve1.unwrap_or_default()
    );

    // Use the cached header for token metadata calls so the simulator does not
    // need to fetch it again.
    let header_for_weth = Some(sealed_header.clone());
    let header_for_usdc = Some(sealed_header);

    let weth_meta = provider
        .get_token_metadata(weth, Some(latest_block), header_for_weth)
        .await?;
    let usdc_decimals = provider
        .get_token_decimals(usdc, Some(latest_block), header_for_usdc)
        .await?;

    println!(
        "WETH metadata: name={} symbol={} decimals={} total_supply={}",
        weth_meta.name, weth_meta.symbol, weth_meta.decimals, weth_meta.total_supply
    );
    println!("USDC decimals (header-backed call): {usdc_decimals}");

    println!("\n✅ Header-aware liquidity example completed.");
    Ok(())
}
