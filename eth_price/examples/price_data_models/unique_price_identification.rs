//! Unique Price Identification Example
//!
//! Demonstrates how the new PriceId system creates unique identifiers
//! for different USD stablecoin price sources across protocols.

use alloy_primitives::{address, U256};
use eth_price::core::price_data_models::*;

fn main() {
    println!("🆔 Unique Price Identification System");
    println!("{}", "=".repeat(50));

    // Token definitions
    let weth = Token::new(
        address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        "WETH",
        18,
    );

    let usdc = Token::new(
        address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        "USDC",
        6,
    );

    let usdt = Token::new(
        address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
        "USDT",
        6,
    );

    let dai = Token::new(
        address!("6B175474E89094C44Da98b954EedeAC495271d0F"),
        "DAI",
        18,
    );

    println!("\n📋 Available Tokens:");
    println!("  {}", weth);
    println!("  {}", usdc);
    println!("  {}", usdt);
    println!("  {}", dai);

    // Example: Different ways to get ETH/USD prices
    println!("\n💵 Different ETH/USD Price Sources:");

    // 1. ETH/USDC via Uniswap V2
    let eth_usdc_v2 = PriceId::new(
        weth.address,
        usdc.address,
        Protocol::UniswapV2,
        address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"), // WETH/USDC V2 pool
        Some(300),                                            // 0.3% fee
    );

    // 2. ETH/USDC via Uniswap V3 (0.05% fee)
    let eth_usdc_v3_500 = PriceId::new(
        weth.address,
        usdc.address,
        Protocol::UniswapV3,
        address!("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"), // WETH/USDC V3 0.05%
        Some(500),                                            // 0.05% fee
    );

    // 3. ETH/USDC via Uniswap V3 (0.3% fee)
    let eth_usdc_v3_3000 = PriceId::new(
        weth.address,
        usdc.address,
        Protocol::UniswapV3,
        address!("8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8"), // WETH/USDC V3 0.3%
        Some(3000),                                           // 0.3% fee
    );

    // 4. ETH/USDT via SushiSwap
    let eth_usdt_sushi = PriceId::new(
        weth.address,
        usdt.address,
        Protocol::SushiSwap,
        address!("06da0fd433C1A5d7a4faa01111c044910A184553"), // WETH/USDT SushiSwap
        Some(300),                                            // 0.3% fee
    );

    // 5. ETH/DAI via SushiSwap
    let eth_dai_sushi = PriceId::new(
        weth.address,
        dai.address,
        Protocol::SushiSwap,
        address!("C3D03e4F041Fd4cD388c549Ee2A29a9E5075882f"), // WETH/DAI SushiSwap
        Some(300),                                            // 0.3% fee
    );

    println!("  1. {}", eth_usdc_v2.describe("ETH", "USDC"));
    println!("  2. {}", eth_usdc_v3_500.describe("ETH", "USDC"));
    println!("  3. {}", eth_usdc_v3_3000.describe("ETH", "USDC"));
    println!("  4. {}", eth_usdt_sushi.describe("ETH", "USDT"));
    println!("  5. {}", eth_dai_sushi.describe("ETH", "DAI"));

    println!("\n🔢 Unique Hash Values:");
    println!("  V2 USDC:     {}", eth_usdc_v2.hash_value());
    println!("  V3 USDC 500: {}", eth_usdc_v3_500.hash_value());
    println!("  V3 USDC 3000:{}", eth_usdc_v3_3000.hash_value());
    println!("  SUSHI USDT:  {}", eth_usdt_sushi.hash_value());
    println!("  SUSHI DAI:   {}", eth_dai_sushi.hash_value());

    println!("\n🔄 Pair Comparisons:");
    println!(
        "  V2 & V3 same pair?        {}",
        eth_usdc_v2.same_pair(&eth_usdc_v3_500)
    );
    println!(
        "  V3 different fees same?   {}",
        eth_usdc_v3_500.same_pair(&eth_usdc_v3_3000)
    );
    println!(
        "  ETH/USDC & ETH/USDT same? {}",
        eth_usdc_v2.same_pair(&eth_usdt_sushi)
    );
    println!(
        "  ETH/USDT & ETH/DAI same?  {}",
        eth_usdt_sushi.same_pair(&eth_dai_sushi)
    );

    // Demonstrate Protocol categorization
    println!("\n📊 Protocol Classification:");
    let protocols = vec![
        Protocol::UniswapV2,
        Protocol::UniswapV3,
        Protocol::SushiSwap,
        Protocol::Curve,
        Protocol::Balancer,
        Protocol::Chainlink,
        Protocol::OneInch,
        Protocol::Paraswap,
    ];

    println!(
        "  AMMs:        {}",
        protocols
            .iter()
            .filter(|p| p.is_amm())
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    println!(
        "  Oracles:     {}",
        protocols
            .iter()
            .filter(|p| p.is_oracle())
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    println!(
        "  Aggregators: {}",
        protocols
            .iter()
            .filter(|p| p.is_aggregator())
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Show precision-safe pricing basics
    println!("\n💰 Precision-Safe Price Examples:");

    // Example 1: Simple ratio
    let price1 = RawChainPrice::new(U256::from(3000), U256::from(1)); // 3000 USDC per 1 ETH
    println!(
        "  3000 USDC/ETH: {} (display: {:.2})",
        price1,
        price1.to_f64()
    );

    // Example 2: Fraction
    let price2 = RawChainPrice::new(U256::from(1), U256::from(3000)); // 1/3000 ETH per USDC
    println!(
        "  1/3000 ETH/USDC: {} (display: {:.8})",
        price2,
        price2.to_f64()
    );

    // Example 3: Amount calculation
    let one_eth = U256::from(10).pow(U256::from(18)); // 1 ETH in wei
    let usdc_out = price1.calculate_amount_out(one_eth);
    println!("  1 ETH → {} USDC units", usdc_out);

    let one_thousand_usdc = U256::from(1000) * U256::from(10).pow(U256::from(6)); // 1000 USDC
    let eth_needed = price1.calculate_amount_in(one_thousand_usdc);
    println!("  1000 USDC ← {} ETH wei", eth_needed);

    println!("\n✅ Key Benefits Demonstrated:");
    println!("  • Each price source gets a unique, deterministic identifier");
    println!("  • Same token pair from different protocols/fees are distinguished");
    println!("  • Hash-based lookups enable efficient price aggregation");
    println!("  • Precision-safe arithmetic prevents floating-point errors");
    println!("  • Protocol classification supports intelligent routing");
}
