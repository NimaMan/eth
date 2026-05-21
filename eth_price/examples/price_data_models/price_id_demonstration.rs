//! Demonstrate the new PriceId and RawChainPrice functionality
//!
//! This example shows how unique price identification works with the enhanced
//! price data models, including precision-safe arithmetic and liquidity metrics.

use alloy_primitives::{address, U256};
use eth_price::core::price_data_models::*;

fn main() {
    println!("🔍 Price Data Models Demonstration");
    println!("{}", "=".repeat(50));

    // Create some example tokens
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

    println!("\n📋 Tokens:");
    println!("  {}", weth);
    println!("  {}", usdc);
    println!("  {}", usdt);

    // Create example pools with different types
    let uniswap_v2_pool = Pool::new(
        address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"),
        Protocol::UniswapV2,
        weth.clone(),
        usdc.clone(),
        PoolKind::V2 {
            reserve0: U256::from(50000) * weth.scaling_factor(), // 50,000 WETH
            reserve1: U256::from(150_000_000) * usdc.scaling_factor(), // 150M USDC
            fee_bps: 300,                                        // 0.3%
        },
    );

    let uniswap_v3_pool = Pool::new(
        address!("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"),
        Protocol::UniswapV3,
        weth.clone(),
        usdc.clone(),
        PoolKind::V3 {
            // Using a simpler sqrt price that won't cause overflow
            sqrt_price_x96: U256::from(1946980738007648u64) * U256::from(2).pow(U256::from(64)), // Approximate WETH/USDC price
            tick: 202919,
            fee_bps: 500, // 0.05%
            liquidity: U256::from(5000000000000000000u64),
        },
    );

    let curve_3pool = Pool::new(
        address!("bEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7"),
        Protocol::Curve,
        usdc.clone(),
        usdt.clone(),
        PoolKind::Curve {
            balances: vec![
                U256::from(200_000_000) * usdc.scaling_factor(), // 200M USDC
                U256::from(180_000_000) * usdt.scaling_factor(), // 180M USDT
                U256::from(190_000_000) * U256::from(10).pow(U256::from(18)), // 190M DAI
            ],
            amplification: U256::from(2000),
            fee_bps: 4,          // 0.04%
            admin_fee_bps: 5000, // 50%
        },
    );

    println!("\n🏊 Pools:");
    println!("  {}", uniswap_v2_pool.display_name());
    println!("  {}", uniswap_v3_pool.display_name());
    println!("  {}", curve_3pool.display_name());

    // Create unique price IDs for different USD stablecoin sources
    let v2_price_id = PriceId::new(
        weth.address,
        usdc.address,
        Protocol::UniswapV2,
        uniswap_v2_pool.address,
        Some(300),
    );

    let v3_price_id = PriceId::new(
        weth.address,
        usdc.address,
        Protocol::UniswapV3,
        uniswap_v3_pool.address,
        Some(500),
    );

    let curve_usdc_usdt_id = PriceId::new(
        usdc.address,
        usdt.address,
        Protocol::Curve,
        curve_3pool.address,
        Some(4),
    );

    println!("\n🆔 Unique Price IDs:");
    println!("  V2:    {}", v2_price_id);
    println!("  V3:    {}", v3_price_id);
    println!("  Curve: {}", curve_usdc_usdt_id);

    // Test precision-safe pricing
    if let Some(v2_mid_price) = uniswap_v2_pool.calculate_mid_price() {
        println!("\n💰 Precision-Safe Pricing:");
        println!("  V2 WETH/USDC mid-price: {}", v2_mid_price);
        println!("  As f64 (display only):  {:.6}", v2_mid_price.to_f64());

        // Test amount calculations
        let one_eth = U256::from(10).pow(U256::from(18));
        let usdc_out = v2_mid_price.calculate_amount_out(one_eth);
        println!("  1 ETH would get:        {} raw USDC units", usdc_out);
        println!(
            "  1 ETH would get:        {:.2} USDC",
            usdc.format_amount(usdc_out)
        );

        // Test execution pricing (with V2 pool's get_amount_out)
        if let Some(actual_out) = uniswap_v2_pool.get_amount_out(one_eth, true) {
            println!(
                "  1 ETH actual output:    {:.2} USDC (after 0.3% fee)",
                usdc.format_amount(actual_out)
            );
        }
    }

    if let Some(v3_mid_price) = uniswap_v3_pool.calculate_mid_price() {
        println!("\n  V3 WETH/USDC mid-price: {}", v3_mid_price);
        println!("  As f64 (display only):  {:.6}", v3_mid_price.to_f64());
    }

    // Demonstrate pool state and price observations
    let current_block = 19000000u64;
    let current_timestamp = 1700000000u64;

    let pool_state = PoolState::new(uniswap_v2_pool.clone(), current_block, current_timestamp);

    if let Some(price_obs) =
        PriceObservation::from_pool_state(&pool_state, weth.address, usdc.address)
    {
        println!("\n📊 Price Observation:");
        println!("  {}", price_obs.describe("WETH", "USDC"));
        println!(
            "  Freshness score: {:.2}",
            price_obs.freshness_score(current_timestamp + 300)
        ); // 5 minutes later
        println!(
            "  Suitable for aggregation: {}",
            price_obs.is_suitable_for_aggregation(current_timestamp + 300)
        );
    }

    // Show how same token pair gets different IDs for different protocols
    println!("\n🔄 Same Pair, Different Sources:");
    println!(
        "  V2 and V3 same pair? {}",
        v2_price_id.same_pair(&v3_price_id)
    );
    println!("  V2 hash: {}", v2_price_id.hash_value());
    println!("  V3 hash: {}", v3_price_id.hash_value());
    println!("  Different hashes ensure unique identification!");

    println!("\n✅ Demonstration complete!");
    println!("Key benefits:");
    println!("  • Unique identification for each USD stablecoin source");
    println!("  • Precision-safe arithmetic using U256");
    println!("  • Execution-aware pricing with fee consideration");
    println!("  • Extensible pool types (V2, V3, Curve, Balancer)");
    println!("  • Liquidity metrics for aggregation scoring");
}
