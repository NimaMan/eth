use alloy_primitives::Address;
/// SushiSwap Price Feed Example
///
/// Shows ETH price from SushiSwap pools (Uniswap V2 fork)
/// Uses the modular tx_simulator and shared provider pattern
use eth_price::core::RawChainPrice;
use eth_price::SushiSwapReader;
use hex::decode as hex_decode;
use reth_chain_query::provider_factory_from_datadir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🍣 SushiSwap Price Feed - Uniswap V2 Fork");
    println!("{}", "=".repeat(50));

    // Create shared provider factory
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Initialize SushiSwap reader with shared provider
    let reader = SushiSwapReader::from_provider(provider_factory.clone())?;

    println!("\n📊 ETH Price from SushiSwap (generic, by pool address):");
    println!("{}", "-".repeat(50));

    // SushiSwap pool addresses (mainnet)
    let usdc_weth =
        Address::from_slice(&hex_decode("397FF1542f962076d0BFE58eA045FfA2d347ACa0").unwrap()); // token0=USDC(6), token1=WETH(18)
    let weth_usdt =
        Address::from_slice(&hex_decode("06da0fd433C1A5d7a4faa01111c044910A184553").unwrap()); // token0=WETH(18), token1=USDT(6)
    let dai_weth =
        Address::from_slice(&hex_decode("C3D03e4F041Fd4cD388c549Ee2A29a9E5075882f").unwrap()); // token0=DAI(18),  token1=WETH(18)

    let print_price =
        |label: &str, pool: Address, token0_dec: u8, token1_dec: u8, base_is_token0: bool| {
            match reader.get_reserves_latest_raw(pool) {
                Ok((r0, r1)) => {
                    let (base_reserve, quote_reserve, base_dec, quote_dec) = if base_is_token0 {
                        (r0, r1, token0_dec, token1_dec)
                    } else {
                        (r1, r0, token1_dec, token0_dec)
                    };
                    let raw = RawChainPrice::from_reserves(quote_reserve, base_reserve);
                    let inverse = raw.inverse();
                    let price = raw.to_scaled_f64(base_dec, quote_dec);
                    println!("  💰 {:<10}: {:>10.6}  pool=0x{:x}", label, price, pool);
                    println!(
                        "      raw: {}/{} quote/base, inverse {}/{} base/quote",
                        raw.numerator, raw.denominator, inverse.numerator, inverse.denominator
                    );
                }
                Err(e) => println!("  ❌ {:<10}: {}", label, e),
            }
        };

    print_price("ETH/USDC", usdc_weth, 6, 18, false);
    print_price("ETH/USDT", weth_usdt, 18, 6, true);
    print_price("ETH/DAI", dai_weth, 18, 18, false);

    println!("\n✅ SushiSwap generic price feed ready");
    Ok(())
}
