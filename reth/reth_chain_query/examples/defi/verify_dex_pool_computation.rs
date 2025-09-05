//! Verify DEX pool address computation
//!
//! This example verifies that our pool address computation functions are correct
//! by computing addresses for well-known pools and comparing them to the actual
//! on-chain addresses.
//!
//! Algorithm:
//! 1. Use known token addresses (WETH, USDC, USDT, DAI)
//! 2. Compute pool addresses using our functions
//! 3. Compare with known on-chain pool addresses
//! 4. Test multiple protocols (Uniswap V2, V3, SushiSwap)
//! 5. Verify all V3 fee tiers

use alloy_primitives::{address, Address};
use reth_chain_query::common_addresses::dex_pools::*;
use reth_chain_query::common_addresses::denom_tokens::get_address_by_name;
use reth_chain_query::TxSimulator;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 DEX Pool Address Computation Verification");
    println!("{}", "=".repeat(60));
    
    // Known token addresses
    let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    let usdt = address!("dAC17F958D2ee523a2206206994597C13D831ec7");
    let dai = address!("6B175474E89094C44Da98b954EedeAC495271d0F");
    
    // Additional tokens for expanded testing
    let steth = address!("ae7ab96520DE3A18E5e111B5EaAb095312D7fE84");
    let wbtc = address!("2260fac5e5542a773aa44fbcfedf7c193bc2c599");
    
    println!("\n📋 Token Addresses:");
    println!("WETH:  0x{:x}", weth);
    println!("USDC:  0x{:x}", usdc);
    println!("USDT:  0x{:x}", usdt);
    println!("DAI:   0x{:x}", dai);
    println!("stETH: 0x{:x}", steth);
    println!("WBTC:  0x{:x}", wbtc);
    
    // Test Uniswap V2
    println!("\n🦄 Uniswap V2 Pool Verification");
    println!("{}", "-".repeat(40));
    test_uniswap_v2_pools(weth, usdc, usdt, dai);
    
    // Test Uniswap V3
    println!("\n🦄 Uniswap V3 Pool Verification");
    println!("{}", "-".repeat(40));
    test_uniswap_v3_pools(weth, usdc, usdt, dai);
    
    // Test SushiSwap
    println!("\n🍣 SushiSwap Pool Verification");
    println!("{}", "-".repeat(40));
    test_sushiswap_pools(weth, usdc, usdt, dai);
    
    // Test using get_address_by_name
    println!("\n🔧 Testing with get_address_by_name");
    println!("{}", "-".repeat(40));
    test_with_name_lookup();
    
    // Test dynamic pool discovery
    println!("\n🔍 Dynamic Pool Discovery Testing");
    println!("{}", "-".repeat(40));
    match test_dynamic_pool_discovery(weth, usdc, usdt, dai, steth, wbtc).await {
        Ok(_) => println!("✅ Dynamic discovery tests completed"),
        Err(e) => println!("⚠️  Dynamic discovery tests failed: {}", e),
    }
    
    println!("\n✅ Verification complete!");
    Ok(())
}

fn test_uniswap_v2_pools(weth: Address, usdc: Address, usdt: Address, dai: Address) {
    // Known Uniswap V2 pool addresses
    let known_weth_usdc = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
    let known_weth_usdt = address!("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852");
    let known_weth_dai = address!("A478c2975Ab1Ea89e8196811F51A7B7Ade33eB11");
    let known_usdc_usdt = address!("3041CbD36888bECc7bbCBc0045E3B1f144466f5f");
    
    // Compute pool addresses
    let computed_weth_usdc = compute_uniswap_v2_pool(weth, usdc);
    let computed_weth_usdt = compute_uniswap_v2_pool(weth, usdt);
    let computed_weth_dai = compute_uniswap_v2_pool(weth, dai);
    let computed_usdc_usdt = compute_uniswap_v2_pool(usdc, usdt);
    
    // Verify
    print_verification("WETH/USDC", known_weth_usdc, computed_weth_usdc);
    print_verification("WETH/USDT", known_weth_usdt, computed_weth_usdt);
    print_verification("WETH/DAI", known_weth_dai, computed_weth_dai);
    print_verification("USDC/USDT", known_usdc_usdt, computed_usdc_usdt);
}

fn test_uniswap_v3_pools(weth: Address, usdc: Address, usdt: Address, dai: Address) {
    // Known Uniswap V3 pool addresses
    // WETH/USDC pools at different fee tiers
    let known_weth_usdc_500 = address!("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"); // 0.05%
    let known_weth_usdc_3000 = address!("8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8"); // 0.3%
    let known_weth_usdc_10000 = address!("7BeA39867e4169DBe237d55C8242a8f2fcDcc387"); // 1%
    
    // WETH/USDT 0.05% pool
    let known_weth_usdt_500 = address!("11b815efB8f581194ae79006d24E0d814B7697F6");
    
    // USDC/USDT pools
    let known_usdc_usdt_100 = address!("3416cF6C708Da44DB2624D63ea0AAef7113527C6"); // 0.01%
    let known_usdc_usdt_500 = address!("EEa0E9f5b321E72C7f9810F7D08B93c8316cF8F1"); // 0.05% (might be different)
    
    // Compute pool addresses
    let computed_weth_usdc_500 = compute_uniswap_v3_pool(weth, usdc, 500);
    let computed_weth_usdc_3000 = compute_uniswap_v3_pool(weth, usdc, 3000);
    let computed_weth_usdc_10000 = compute_uniswap_v3_pool(weth, usdc, 10000);
    let computed_weth_usdt_500 = compute_uniswap_v3_pool(weth, usdt, 500);
    let computed_usdc_usdt_100 = compute_uniswap_v3_pool(usdc, usdt, 100);
    
    // Verify
    print_verification("WETH/USDC 0.05%", known_weth_usdc_500, computed_weth_usdc_500);
    print_verification("WETH/USDC 0.3%", known_weth_usdc_3000, computed_weth_usdc_3000);
    print_verification("WETH/USDC 1%", known_weth_usdc_10000, computed_weth_usdc_10000);
    print_verification("WETH/USDT 0.05%", known_weth_usdt_500, computed_weth_usdt_500);
    print_verification("USDC/USDT 0.01%", known_usdc_usdt_100, computed_usdc_usdt_100);
    
    // Test get_all_v3_pools
    println!("\n📊 Testing get_all_v3_pools for WETH/USDC:");
    let all_pools = get_all_v3_pools(weth, usdc);
    for (pool_addr, fee_tier) in all_pools {
        println!("  Fee {:.2}%: 0x{:x}", fee_tier as f64 / 10000.0, pool_addr);
    }
}

fn test_sushiswap_pools(weth: Address, usdc: Address, usdt: Address, dai: Address) {
    // Known SushiSwap pool addresses
    let known_weth_usdc = address!("397FF1542f962076d0BFE58eA045FfA2d347ACa0");
    let known_weth_usdt = address!("06da0fd433C1A5d7a4faa01111c044910A184553");
    let known_weth_dai = address!("C3D03e4F041Fd4cD388c549Ee2A29a9E5075882f");
    
    // Compute pool addresses
    let computed_weth_usdc = compute_sushiswap_pool(weth, usdc);
    let computed_weth_usdt = compute_sushiswap_pool(weth, usdt);
    let computed_weth_dai = compute_sushiswap_pool(weth, dai);
    
    // Verify
    print_verification("WETH/USDC", known_weth_usdc, computed_weth_usdc);
    print_verification("WETH/USDT", known_weth_usdt, computed_weth_usdt);
    print_verification("WETH/DAI", known_weth_dai, computed_weth_dai);
}

fn test_with_name_lookup() {
    // Try to get addresses by name (if available)
    match (get_address_by_name("WETH"), get_address_by_name("USDC")) {
        (Some(weth), Some(usdc)) => {
            println!("✅ Successfully retrieved WETH and USDC addresses by name");
            
            let pool = compute_uniswap_v2_pool(weth, usdc);
            println!("  Computed WETH/USDC V2 pool: 0x{:x}", pool);
            
            let expected = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
            if pool == expected {
                println!("  ✅ Matches known pool address!");
            } else {
                println!("  ❌ Does not match known pool address");
            }
        }
        _ => {
            println!("⚠️  Could not retrieve token addresses by name");
            println!("    Make sure denom_tokens has WETH and USDC entries");
        }
    }
}

async fn test_dynamic_pool_discovery(weth: Address, usdc: Address, usdt: Address, dai: Address, steth: Address, wbtc: Address) -> Result<()> {
    // Initialize simulator - use default database path
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    
    println!("🌀 Testing Curve Pool Discovery:");
    
    // Test USDC/USDT pair (should find 3Pool)
    match find_curve_pool_for_coins(&simulator, usdc, usdt, None).await {
        Ok(Some(pool)) => {
            println!("  ✅ Found Curve USDC/USDT pool: 0x{:x}", pool);
            if pool == address!("bebc44782c7db0a1a60cb6fe97d0b483032ff1c7") {
                println!("      → Confirmed: This is the famous 3Pool (DAI/USDC/USDT)!");
            }
        },
        Ok(None) => println!("  ℹ️  No Curve USDC/USDT pool found"),
        Err(e) => println!("  ❌ Error finding Curve USDC/USDT pool: {}", e),
    }
    
    // Test WETH/stETH pair (should exist - major Curve pool)
    match find_curve_pool_for_coins(&simulator, weth, steth, None).await {
        Ok(Some(pool)) => {
            println!("  ✅ Found Curve WETH/stETH pool: 0x{:x}", pool);
            if pool == address!("21e27a5e5513d6e65c4f830167390997aa84843a") {
                println!("      → Confirmed: This is the ETH/stETH pool!");
            }
        },
        Ok(None) => println!("  ℹ️  No Curve WETH/stETH pool found"),
        Err(e) => println!("  ❌ Error finding Curve WETH/stETH pool: {}", e),
    }
    
    // Test USDT/WBTC pair (part of tricrypto pool)
    match find_curve_pool_for_coins(&simulator, usdt, wbtc, None).await {
        Ok(Some(pool)) => {
            println!("  ✅ Found Curve USDT/WBTC pool: 0x{:x}", pool);
            if pool == address!("D51a44d3FaE010294C616388b506AcdA1bfAAE46") {
                println!("      → Confirmed: This is the TriCrypto pool (USDT/WBTC/WETH)!");
            }
        },
        Ok(None) => println!("  ℹ️  No Curve USDT/WBTC pool found"),
        Err(e) => println!("  ❌ Error finding Curve USDT/WBTC pool: {}", e),
    }
    
    // Test DAI/WETH pair (should NOT exist - Curve doesn't mix stables with volatiles)
    match find_curve_pool_for_coins(&simulator, dai, weth, None).await {
        Ok(Some(pool)) => println!("  🤔 Unexpectedly found Curve DAI/WETH pool: 0x{:x}", pool),
        Ok(None) => println!("  ✅ No Curve DAI/WETH pool found (as expected - different volatility classes)"),
        Err(e) => println!("  ❌ Error finding Curve DAI/WETH pool: {}", e),
    }
    
    println!("\n⚖️  Testing Balancer Pool Verification:");
    
    // Test known Balancer pool (WETH/USDC 80/20 weighted pool)
    // Pool ID: 0x96646936b91d6b9d7d0c47c496afbf3d6ec7b6f8000200000000000000000019
    let weth_usdc_pool_id = [
        0x96, 0x64, 0x69, 0x36, 0xb9, 0x1d, 0x6b, 0x9d, 0x7d, 0x0c, 0x47, 0xc4, 0x96, 0xaf, 0xbf, 0x3d,
        0x6e, 0xc7, 0xb6, 0xf8, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19
    ];
    
    // Test another Balancer pool (DAI/USDC/USDT stable pool, if it exists)
    // Pool ID: 0x06df3b2bbb68adc8b0e302443692037ed9f91b42000000000000000000000063
    let stable_pool_id = [
        0x06, 0xdf, 0x3b, 0x2b, 0xbb, 0x68, 0xad, 0xc8, 0xb0, 0xe3, 0x02, 0x44, 0x36, 0x92, 0x03, 0x7e,
        0xd9, 0xf9, 0x1b, 0x42, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63
    ];
    
    match verify_balancer_pool_tokens(&simulator, weth_usdc_pool_id, weth, usdc, None).await {
        Ok(true) => println!("  ✅ Verified Balancer WETH/USDC pool contains both tokens"),
        Ok(false) => println!("  ❌ Balancer pool does not contain both WETH and USDC"),
        Err(e) => println!("  ❌ Error verifying Balancer pool: {}", e),
    }
    
    // Get all tokens in the pool
    match get_balancer_pool_tokens(&simulator, weth_usdc_pool_id, None).await {
        Ok(Some((tokens, balances))) => {
            println!("  📊 WETH/USDC Pool composition:");
            for (i, token) in tokens.iter().enumerate() {
                if let Some(balance) = balances.get(i) {
                    println!("    Token 0x{:x}: {} units", token, balance);
                }
            }
        },
        Ok(None) => println!("  ⚠️  Could not retrieve pool composition"),
        Err(e) => println!("  ❌ Error getting pool tokens: {}", e),
    }
    
    // Test stable pool (DAI/USDC/USDT if it exists) - test with DAI/USDC first
    match verify_balancer_pool_tokens(&simulator, stable_pool_id, dai, usdc, None).await {
        Ok(true) => {
            println!("  ✅ Verified Balancer DAI/USDC/USDT stable pool contains expected tokens");
            
            // Get composition of stable pool
            match get_balancer_pool_tokens(&simulator, stable_pool_id, None).await {
                Ok(Some((tokens, balances))) => {
                    println!("  📊 Stable Pool composition:");
                    for (i, token) in tokens.iter().enumerate() {
                        if let Some(balance) = balances.get(i) {
                            println!("    Token 0x{:x}: {} units", token, balance);
                        }
                    }
                },
                Ok(None) => println!("  ⚠️  Could not retrieve stable pool composition"),
                Err(e) => println!("  ❌ Error getting stable pool tokens: {}", e),
            }
        },
        Ok(false) => println!("  ❌ Balancer stable pool does not contain expected stablecoin tokens"),
        Err(e) => println!("  ❌ Error verifying Balancer stable pool: {}", e),
    }
    
    Ok(())
}

fn print_verification(pair: &str, known: Address, computed: Address) {
    let matches = known == computed;
    let symbol = if matches { "✅" } else { "❌" };
    
    println!("{} {}:", symbol, pair);
    println!("  Known:    0x{:x}", known);
    println!("  Computed: 0x{:x}", computed);
    if !matches {
        println!("  ⚠️  MISMATCH - Computation may be incorrect!");
    }
}