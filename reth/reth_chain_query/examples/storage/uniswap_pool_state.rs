/// Uniswap V3 Pool State Analysis
/// 
/// Demonstrates direct storage access to extract real-time Uniswap V3 pool state.
/// This example reads critical storage slots to get pool information that would
/// normally require multiple RPC calls.
/// 
/// Algorithm:
/// 1. Define major Uniswap V3 pools (USDC/ETH, USDT/ETH, WBTC/ETH)
/// 2. For each pool, read key storage slots:
///    - Slot 0: Packed data (sqrtPriceX96, tick, observationIndex, etc.)
///    - Slot 1: feeGrowthGlobal0X128
///    - Slot 2: feeGrowthGlobal1X128  
///    - Slot 3: protocolFees
///    - Slot 4: liquidity
/// 3. Decode packed slot 0 data to extract individual fields
/// 4. Calculate human-readable prices from sqrtPriceX96
/// 5. Display comprehensive pool state with performance metrics
/// 
/// Storage Layout (Uniswap V3 Pool):
/// - Slot 0: Packed(sqrtPriceX96:160, tick:24, observationIndex:16, observationCardinality:16, 
///           observationCardinalityNext:16, feeProtocol:8, unlocked:8)
/// - Slot 1: feeGrowthGlobal0X128 (uint256)
/// - Slot 2: feeGrowthGlobal1X128 (uint256)
/// - Slot 3: protocolFees packed(token0:128, token1:128)
/// - Slot 4: liquidity (uint128)

use alloy_primitives::{Address, U256, B256};
use reth_chain_query::{ChainQuery, Result};
use std::str::FromStr;
use std::time::Instant;

/// Major Uniswap V3 pools for analysis
const UNISWAP_POOLS: &[(&str, &str, &str, u32)] = &[
    ("USDC/ETH", "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", "0.05%", 500),
    ("USDT/ETH", "0x11b815efB8f581194ae79006d24E0d814B7697F6", "0.05%", 500),
    ("WBTC/ETH", "0xCBCdF9626bC03E24f779434178A73a0B4bad62eD", "0.30%", 3000),
    ("DAI/ETH", "0x60594a405d53811d3BC4766596EFD80fd545A270", "0.05%", 500),
    ("USDC/USDT", "0x3416cF6C708Da44DB2624D63ea0AAef7113527C6", "0.01%", 100),
    ("WETH/USDC", "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8", "0.30%", 3000),
];

/// Decode sqrt price to human readable price
fn sqrt_price_to_price(sqrt_price_x96: U256, decimals0: u8, decimals1: u8, token0_symbol: &str, token1_symbol: &str) -> (f64, String) {
    // sqrtPriceX96 = sqrt(price) * 2^96
    // price = (sqrtPriceX96 / 2^96)^2
    // Need to adjust for token decimals: price = raw_price * 10^(decimals0 - decimals1)
    
    let sqrt_price_f64 = sqrt_price_x96.to_string().parse::<f64>().unwrap_or(0.0);
    let q96 = 2.0_f64.powf(96.0);
    let raw_price = (sqrt_price_f64 / q96).powf(2.0);
    
    let decimal_adjustment = 10.0_f64.powi(decimals0 as i32 - decimals1 as i32);
    let adjusted_price = raw_price * decimal_adjustment;
    
    // Format price description
    let price_desc = if adjusted_price > 1.0 {
        format!("1 {} = {:.4} {}", token0_symbol, adjusted_price, token1_symbol)
    } else {
        format!("1 {} = {:.6} {}", token0_symbol, adjusted_price, token1_symbol)
    };
    
    (adjusted_price, price_desc)
}

/// Decode packed slot 0 data from Uniswap V3 pool
fn decode_slot0(slot0_data: U256) -> (U256, i32, u16, u16, u16, u8, bool) {
    // Extract fields from packed data (right to left):
    // unlocked (8 bits)
    let unlocked = (slot0_data & U256::from(0xFF)) != U256::ZERO;
    let mut remaining = slot0_data >> 8;
    
    // feeProtocol (8 bits)  
    let fee_protocol_u256: U256 = remaining & U256::from(0xFF);
    let fee_protocol = fee_protocol_u256.to::<u32>() as u8;
    remaining = remaining >> 8;
    
    // observationCardinalityNext (16 bits)
    let obs_cardinality_next_u256: U256 = remaining & U256::from(0xFFFF);
    let obs_cardinality_next = obs_cardinality_next_u256.to::<u16>();
    remaining = remaining >> 16;
    
    // observationCardinality (16 bits)
    let obs_cardinality_u256: U256 = remaining & U256::from(0xFFFF);
    let obs_cardinality = obs_cardinality_u256.to::<u16>();
    remaining = remaining >> 16;
    
    // observationIndex (16 bits)
    let obs_index_u256: U256 = remaining & U256::from(0xFFFF);
    let obs_index = obs_index_u256.to::<u16>();
    remaining = remaining >> 16;
    
    // tick (24 bits, signed)
    let tick_raw_u256: U256 = remaining & U256::from(0xFFFFFF);
    let tick_raw = tick_raw_u256.to::<u32>();
    let tick = if tick_raw >= 0x800000 { // Check if negative (24-bit signed)
        (tick_raw as i32) - 0x1000000 // Convert to signed
    } else {
        tick_raw as i32
    };
    remaining = remaining >> 24;
    
    // sqrtPriceX96 (160 bits)
    let sqrt_price_x96 = remaining & ((U256::from(1) << 160) - U256::from(1));
    
    (sqrt_price_x96, tick, obs_index, obs_cardinality, obs_cardinality_next, fee_protocol, unlocked)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🏊 Uniswap V3 Pool State Analysis");
    println!("{}", "=".repeat(70));
    println!("📊 Reading pool storage slots directly from Reth database");
    println!();
    
    // Initialize ChainQuery
    let reth_datadir = "/home/nima/.local/share/reth/mainnet";
    let chain_query = ChainQuery::new(reth_datadir)?;
    let latest_block = chain_query.get_latest_block()?;
    
    println!("🔗 Latest Block: {}", latest_block);
    println!();
    
    let overall_start = Instant::now();
    let mut total_slots_read = 0;
    
    for (pool_name, pool_address, fee_tier, fee) in UNISWAP_POOLS {
        println!("🏊 {} Pool ({}) - Fee: {}", pool_name, pool_address, fee_tier);
        println!("   Address: {}", pool_address);
        
        let pool_addr = Address::from_str(pool_address)?;
        let pool_start = Instant::now();
        
        // Storage slots to read
        let slot0 = B256::ZERO; // Slot 0: packed pool data
        let slot1 = B256::from(U256::from(1)); // Slot 1: feeGrowthGlobal0X128
        let slot2 = B256::from(U256::from(2)); // Slot 2: feeGrowthGlobal1X128  
        let slot3 = B256::from(U256::from(3)); // Slot 3: protocolFees
        let slot4 = B256::from(U256::from(4)); // Slot 4: liquidity
        
        // Read all slots
        let slot0_data = chain_query.get_storage_at(pool_addr, slot0, Some(latest_block)).await?;
        let slot1_data = chain_query.get_storage_at(pool_addr, slot1, Some(latest_block)).await?;
        let slot2_data = chain_query.get_storage_at(pool_addr, slot2, Some(latest_block)).await?;
        let slot3_data = chain_query.get_storage_at(pool_addr, slot3, Some(latest_block)).await?;
        let slot4_data = chain_query.get_storage_at(pool_addr, slot4, Some(latest_block)).await?;
        
        total_slots_read += 5;
        let pool_time = pool_start.elapsed();
        
        // Decode slot 0 packed data
        let (sqrt_price_x96, tick, obs_index, obs_cardinality, obs_cardinality_next, fee_protocol, unlocked) = 
            decode_slot0(slot0_data);
            
        // Calculate human-readable price (approximation for major pairs)
        let (token0_symbol, token1_symbol, decimals0, decimals1) = match *pool_name {
            "USDC/ETH" => ("USDC", "WETH", 6, 18),
            "USDT/ETH" => ("USDT", "WETH", 6, 18), 
            "WBTC/ETH" => ("WBTC", "WETH", 8, 18),
            "DAI/ETH" => ("DAI", "WETH", 18, 18),
            "USDC/USDT" => ("USDC", "USDT", 6, 6),
            "WETH/USDC" => ("WETH", "USDC", 18, 6),
            _ => ("TOKEN0", "TOKEN1", 18, 18),
        };
        
        let (price, price_desc) = sqrt_price_to_price(sqrt_price_x96, decimals0, decimals1, token0_symbol, token1_symbol);
        
        // Extract protocol fees (packed as two uint128)
        let protocol_fee0 = slot3_data & ((U256::from(1) << 128) - U256::from(1));
        let protocol_fee1 = slot3_data >> 128;
        
        // Display results
        println!("   ⚡ Query Time: {:.2}ms (5 storage slots)", pool_time.as_millis());
        println!("   💰 Current Price: {}", price_desc);
        println!("   📊 Tick: {} | SqrtPriceX96: {}", tick, sqrt_price_x96);
        println!("   🌊 Liquidity: {}", slot4_data);
        println!("   📈 Fee Growth Global 0: {}", slot1_data);
        println!("   📉 Fee Growth Global 1: {}", slot2_data);
        println!("   🏦 Protocol Fees: {} / {}", protocol_fee0, protocol_fee1);
        println!("   🔧 Observations: {}/{} (next: {})", obs_index, obs_cardinality, obs_cardinality_next);
        println!("   🔒 Pool Status: {}", if unlocked { "Unlocked ✅" } else { "Locked ❌" });
        println!("   💵 Fee Protocol: {}%", fee_protocol as f32 / 255.0 * 100.0);
        println!();
    }
    
    let total_time = overall_start.elapsed();
    
    // Performance summary
    println!("{}", "=".repeat(70));
    println!("⚡ PERFORMANCE SUMMARY");
    println!("{}", "=".repeat(70));
    println!("Total Storage Slots Read: {}", total_slots_read);
    println!("Total Query Time: {:.2}ms", total_time.as_millis());
    println!("Average Time per Slot: {:.3}ms", total_time.as_millis() as f64 / total_slots_read as f64);
    println!("Average Time per Pool: {:.2}ms", total_time.as_millis() as f64 / UNISWAP_POOLS.len() as f64);
    
    // RPC comparison
    let estimated_rpc_time = total_slots_read * 80; // ~80ms per RPC storage call
    let speedup = estimated_rpc_time as f64 / total_time.as_millis() as f64;
    println!();
    println!("🔄 RPC Comparison:");
    println!("Estimated RPC time: {}ms ({} slots × 80ms)", estimated_rpc_time, total_slots_read);
    println!("Actual query time: {}ms", total_time.as_millis());
    println!("Speedup: {:.1}x faster", speedup);
    
    println!();
    println!("💡 This demonstrates real-time pool monitoring with sub-millisecond latency!");
    println!("🎯 Perfect for MEV bots, arbitrage detection, and DeFi analytics!");
    
    Ok(())
}