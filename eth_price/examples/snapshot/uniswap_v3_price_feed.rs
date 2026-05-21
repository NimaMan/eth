use alloy_primitives::Address;
/// Uniswap V3 Price Feed Example
///
/// Shows ETH prices from Uniswap V3 pools with different fee tiers
/// Uses the modular tx_simulator and shared provider pattern
use eth_price::UniswapV3Reader;
use hex::decode as hex_decode;
use reth_chain_query::provider_factory_from_datadir;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦄 Uniswap V3 Price Feed - Concentrated Liquidity");
    println!("{}", "=".repeat(50));

    // Create shared provider factory
    let provider_factory = provider_factory_from_datadir("/home/nima/.local/share/reth/mainnet")?;

    // Initialize Uniswap V3 reader with shared provider
    let reader = UniswapV3Reader::from_provider(provider_factory.clone())?;

    println!("\n📊 ETH Price from Uniswap V3 (generic, by pool address):");
    println!("{}", "-".repeat(50));

    // USDC/WETH pools (token0=USDC(6), token1=WETH(18))
    let usdc_weth_005 =
        Address::from_slice(&hex_decode("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640").unwrap());
    let usdc_weth_03 =
        Address::from_slice(&hex_decode("8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8").unwrap());
    let usdc_weth_1 =
        Address::from_slice(&hex_decode("7BeA39867e4169DBe237d55C8242a8f2fcdcc387").unwrap());
    // WETH/USDT pool (token0=WETH(18), token1=USDT(6))
    let weth_usdt_03 =
        Address::from_slice(&hex_decode("11b815efB8f581194ae79006d24E0d814B7697F6").unwrap());
    // DAI/USDC 0.01% (token0=DAI(18), token1=USDC(6)) for deriving ETH/DAI if desired
    let dai_usdc_001 =
        Address::from_slice(&hex_decode("5777d92f208679DB4b9778590Fa3CAB3aC9e2168").unwrap());

    let print_price =
        |label: &str, pool: Address, token0_dec: u8, token1_dec: u8, base_is_token0: bool| {
            let start = Instant::now();
            match reader.get_latest_price_by_pool(pool, token0_dec, token1_dec, base_is_token0) {
                Ok((raw_price, block)) => {
                    let dt = start.elapsed();
                    let (base_dec, quote_dec) = if base_is_token0 {
                        (token0_dec, token1_dec)
                    } else {
                        (token1_dec, token0_dec)
                    };
                    let price = raw_price.to_scaled_f64(base_dec, quote_dec);
                    let inverse = raw_price.inverse();
                    println!(
                        "  💰 {:<12}: {:>10.6} ({:.3}ms)  pool=0x{:x}  block={}",
                        label,
                        price,
                        dt.as_secs_f64() * 1000.0,
                        pool,
                        block
                    );
                    println!(
                        "      raw: {}/{} quote/base, inverse {}/{} base/quote",
                        raw_price.numerator,
                        raw_price.denominator,
                        inverse.numerator,
                        inverse.denominator
                    );
                }
                Err(e) => println!("  ❌ {:<12}: {}", label, e),
            }
        };

    // ETH quoted in USDC and USDT
    print_price("ETH/USDC 0.05%", usdc_weth_005, 6, 18, false);
    print_price("ETH/USDC 0.30%", usdc_weth_03, 6, 18, false);
    print_price("ETH/USDC 1.00%", usdc_weth_1, 6, 18, false);
    print_price("ETH/USDT 0.30%", weth_usdt_03, 18, 6, true);

    // Optionally derive ETH/DAI via ETH/USDC and DAI/USDC
    if let (Ok((usdc_per_eth_raw, _)), Ok((usdc_per_dai_raw, _))) = (
        reader.get_latest_price_by_pool(usdc_weth_005, 6, 18, false),
        reader.get_latest_price_by_pool(dai_usdc_001, 18, 6, true),
    ) {
        let usdc_per_eth = usdc_per_eth_raw.to_scaled_f64(18, 6);
        let usdc_per_dai = usdc_per_dai_raw.to_scaled_f64(18, 6);
        if usdc_per_dai > 0.0 {
            let dai_per_eth = usdc_per_eth / usdc_per_dai;
            println!(
                "  💰 {:<12}: {:>10.6} (derived via USDC)",
                "ETH/DAI", dai_per_eth
            );
        }
    }

    println!("\n✅ Uniswap V3 generic price feed ready");
    Ok(())
}
