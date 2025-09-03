/// Query DEX pool states and prices
/// 
/// This example shows how to:
/// 1. Get current prices from Uniswap V2 pools
/// 2. Get current prices from Uniswap V3 pools
/// 3. Compare liquidity across different pools
/// 4. Track price changes over time
/// 
/// Run with: cargo run --example dex_pools

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, B256, U256, keccak256};
use std::str::FromStr;

// Known Uniswap pools
const WETH_USDC_V2: &str = "B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"; // V2 pool
const WETH_USDC_V3: &str = "88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"; // V3 0.05% fee pool

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Uniswap Pool State Reading ===\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // === Uniswap V2 Pool ===
    println!("1. Uniswap V2 Pool State");
    println!("-" .repeat(60));
    
    let v2_pool = Address::from_str(WETH_USDC_V2)?;
    
    // V2 storage layout:
    // Slot 0-4: Various metadata
    // Slot 5: token0
    // Slot 6: token1
    // Slot 7: Not used
    // Slot 8: reserves (packed: reserve0[112], reserve1[112], blockTimestampLast[32])
    // Slot 9: price0CumulativeLast
    // Slot 10: price1CumulativeLast
    // Slot 11: kLast
    
    // Read token addresses
    let token0_slot = B256::from(U256::from(5));
    let token1_slot = B256::from(U256::from(6));
    let token0_raw = provider.get_storage(v2_pool, token0_slot, None).await?;
    let token1_raw = provider.get_storage(v2_pool, token1_slot, None).await?;
    
    // Convert U256 to Address (last 20 bytes)
    let token0_bytes = token0_raw.to_be_bytes::<32>();
    let token1_bytes = token1_raw.to_be_bytes::<32>();
    let token0 = Address::from_slice(&token0_bytes[12..]);
    let token1 = Address::from_slice(&token1_bytes[12..]);
    
    println!("Uniswap V2 WETH/USDC Pool (0x{})", v2_pool);
    println!("  Token0: 0x{} (USDC)", token0);
    println!("  Token1: 0x{} (WETH)", token1);
    
    // Read reserves (slot 8)
    let reserves_slot = B256::from(U256::from(8));
    let packed_reserves = provider.get_storage(v2_pool, reserves_slot, None).await?;
    
    // Unpack reserves (112 bits each)
    let mask_112 = U256::from_str("0xffffffffffffffffffffffffffff").unwrap();
    let reserve0 = packed_reserves & mask_112;
    let reserve1 = (packed_reserves >> 112) & mask_112;
    let timestamp = packed_reserves >> 224;
    
    println!("\n  Reserves:");
    println!("    Reserve0 (USDC): {} USDC", reserve0 / U256::from(10u64.pow(6)));
    println!("    Reserve1 (WETH): {} WETH", reserve1 / U256::from(10u64.pow(18)));
    println!("    Last update: block timestamp {}", timestamp);
    
    // Calculate price
    if reserve0 > U256::ZERO && reserve1 > U256::ZERO {
        // Price = reserve0 / reserve1 * (10^18 / 10^6) = reserve0 * 10^12 / reserve1
        let price = (reserve0 * U256::from(10u64.pow(12))) / reserve1;
        println!("\n  ETH Price: ${}", price);
        
        // Calculate pool TVL
        let tvl_usd = (reserve0 / U256::from(10u64.pow(6))) * U256::from(2);
        println!("  Pool TVL: ${} million", tvl_usd / U256::from(1_000_000));
    }
    
    println!();
    
    // === Uniswap V3 Pool ===
    println!("2. Uniswap V3 Pool State");
    println!("-" .repeat(60));
    
    let v3_pool = Address::from_str(WETH_USDC_V3)?;
    
    // V3 storage layout (simplified):
    // Slot 0: sqrtPriceX96, tick, observationIndex, etc (packed)
    // Slot 1: fee protocol
    // Slot 2: liquidity
    // Slot 3: ticks mapping
    // Slot 4: tick bitmap
    // Slot 5: positions mapping
    // Slot 6: observations array
    // Slot 7: liquidityCumulative
    
    println!("Uniswap V3 WETH/USDC Pool (0x{})", v3_pool);
    println!("  Fee tier: 0.05%");
    
    // Read slot0 (contains current price and tick)
    let slot0 = B256::from(U256::from(0));
    let slot0_data = provider.get_storage(v3_pool, slot0, None).await?;
    
    // Extract sqrtPriceX96 (first 160 bits)
    let sqrt_price_x96 = slot0_data & U256::from_str("0xffffffffffffffffffffffffffffffffffffffff").unwrap();
    
    // Extract tick (next 24 bits)
    let tick_raw = (slot0_data >> 160) & U256::from(0xffffff);
    // Convert to signed int24
    let tick = if tick_raw & U256::from(0x800000) != U256::ZERO {
        // Negative tick
        -((!tick_raw & U256::from(0xffffff)) + U256::from(1))
    } else {
        tick_raw
    };
    
    println!("\n  Current State:");
    println!("    sqrtPriceX96: {}", sqrt_price_x96);
    println!("    Current tick: {}", tick);
    
    // Calculate actual price from sqrtPriceX96
    // price = (sqrtPriceX96 / 2^96)^2 * (10^18 / 10^6)
    // Simplified: price = sqrtPriceX96^2 / 2^192 * 10^12
    if sqrt_price_x96 > U256::ZERO {
        // This is approximate due to precision limits
        let price_x192 = sqrt_price_x96 * sqrt_price_x96;
        let price = price_x192 >> 192;
        let price_adjusted = price * U256::from(10u64.pow(12));
        println!("    ETH Price: ~${}", price_adjusted);
    }
    
    // Read liquidity
    let liquidity_slot = B256::from(U256::from(2));
    let liquidity = provider.get_storage(v3_pool, liquidity_slot, None).await?;
    println!("    Active liquidity: {}", liquidity);
    
    println!();
    
    // === Compare V2 vs V3 ===
    println!("3. V2 vs V3 Comparison");
    println!("-" .repeat(60));
    
    println!("Pool Comparison:");
    println!("  V2 WETH/USDC:");
    println!("    - Simple x*y=k formula");
    println!("    - 0.30% fixed fee");
    println!("    - Full range liquidity");
    println!("    - Reserves: {} USDC / {} WETH", 
        reserve0 / U256::from(10u64.pow(6)),
        reserve1 / U256::from(10u64.pow(18))
    );
    
    println!("\n  V3 WETH/USDC:");
    println!("    - Concentrated liquidity");
    println!("    - 0.05% fee tier");
    println!("    - Tick-based positions");
    println!("    - Active liquidity: {}", liquidity);
    
    println!();
    
    // === Historical Pool State ===
    println!("4. Historical Pool State");
    println!("-" .repeat(60));
    
    let checkpoints = vec![
        (17_000_000, "April 2023"),
        (18_000_000, "August 2023"),
        (None, "Current"),
    ];
    
    println!("V2 Pool reserves over time:");
    for (block, label) in checkpoints {
        match provider.get_storage(v2_pool, reserves_slot, block).await {
            Ok(packed) => {
                let r0 = packed & mask_112;
                let r1 = (packed >> 112) & mask_112;
                
                let price = if r1 > U256::ZERO {
                    (r0 * U256::from(10u64.pow(12))) / r1
                } else {
                    U256::ZERO
                };
                
                println!("  {} {}: ${} (USDC: {}, WETH: {})",
                    label,
                    block.map(|b| format!("(block {})", b)).unwrap_or_default(),
                    price,
                    r0 / U256::from(10u64.pow(6)),
                    r1 / U256::from(10u64.pow(18))
                );
            }
            Err(_) => {
                println!("  {}: Not available", label);
            }
        }
    }
    
    println!();
    
    // === Pool Factory ===
    println!("5. Finding Pool Addresses");
    println!("-" .repeat(60));
    
    // Calculate V2 pool address from tokens
    let weth = Address::from_str("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;
    let usdc = Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?;
    
    // V2 uses CREATE2 with sorted tokens
    let (token0_sorted, token1_sorted) = if weth < usdc {
        (weth, usdc)
    } else {
        (usdc, weth)
    };
    
    println!("Pool Discovery:");
    println!("  Token0 (sorted): 0x{}", token0_sorted);
    println!("  Token1 (sorted): 0x{}", token1_sorted);
    println!("  V2 Pool: 0x{}", v2_pool);
    println!("  V3 Pool (0.05%): 0x{}", v3_pool);
    
    println!("\n✅ Uniswap pool reading complete!");
    println!("\nNote: For production use, consider using the Uniswap SDK for proper price calculations");
    
    Ok(())
}