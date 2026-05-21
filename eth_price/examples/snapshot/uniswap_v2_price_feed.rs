use alloy_primitives::Address;
/// Uniswap V2 Price Feed Example
///
/// Shows ETH price from Uniswap V2 pools (constant product AMM)
/// Uses the modular tx_simulator and shared provider pattern
use eth_price::{RawChainPrice, UniswapV2Reader};
use hex::decode as hex_decode;
use reth_chain_query::provider_factory_from_datadir;
use reth_provider::BlockNumReader;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦄 Uniswap V2 Price Feed - Constant Product AMM");
    println!("{}", "=".repeat(50));

    // Create shared provider factory
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Initialize Uniswap V2 reader with shared provider
    let reader = UniswapV2Reader::from_provider(provider_factory.clone())?;

    println!("\n📊 ETH Price from Uniswap V2 (generic, by pool address):");
    println!("{}", "-".repeat(50));

    // Uniswap V2 pool addresses (mainnet)
    let usdc_weth =
        Address::from_slice(&hex_decode("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap()); // token0=USDC(6), token1=WETH(18)
    let usdt_weth =
        Address::from_slice(&hex_decode("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852").unwrap()); // token0=WETH(18), token1=USDT(6)
    let dai_weth =
        Address::from_slice(&hex_decode("A478c2975Ab1Ea89e8196811F51A7B7Ade33eB11").unwrap()); // token0=DAI(18),  token1=WETH(18)

    // Helper to fetch and compute quote-per-ETH using raw reserves
    let print_price = |label: &str,
                       pool: Address,
                       base_is_token0: bool,
                       token0_decimals: u8,
                       token1_decimals: u8| {
        let start = Instant::now();
        match reader.get_reserves_latest_raw(pool) {
            Ok((r0, r1)) => {
                let price = reader.compute_price_from_reserves(
                    r0,
                    r1,
                    token0_decimals,
                    token1_decimals,
                    base_is_token0,
                );
                let raw = if base_is_token0 {
                    RawChainPrice::from_reserves(r1, r0)
                } else {
                    RawChainPrice::from_reserves(r0, r1)
                };
                let inverse = raw.inverse();
                let query_time = start.elapsed();
                println!(
                    "  💰 {:<8}: {:>10.6} ({:.3}ms)  pool=0x{:x}",
                    label,
                    price,
                    query_time.as_secs_f64() * 1000.0,
                    pool
                );
                println!(
                    "      raw: {}/{} quote/base, inverse {}/{} base/quote",
                    raw.numerator, raw.denominator, inverse.numerator, inverse.denominator
                );
            }
            Err(e) => {
                println!("  ❌ {:<8}: {}", label, e);
            }
        }
    };

    // ETH quoted in USDC, USDT, and DAI
    // USDC/WETH: token0=USDC(6), token1=WETH(18); base=WETH is token1 => base_is_token0=false
    print_price("ETH/USDC", usdc_weth, false, 6, 18);
    // WETH/USDT: token0=WETH(18), token1=USDT(6); base=WETH is token0 => base_is_token0=true
    print_price("ETH/USDT", usdt_weth, true, 18, 6);
    // DAI/WETH:   token0=DAI(18), token1=WETH(18); base=WETH is token1 => base_is_token0=false
    print_price("ETH/DAI", dai_weth, false, 18, 18);

    // Show latest block for reference
    if let Ok(latest_block) = provider_factory.last_block_number() {
        println!("\n📍 Latest block: {}", latest_block);
    }

    println!("\n✅ Uniswap V2 generic price feed ready");
    Ok(())
}
