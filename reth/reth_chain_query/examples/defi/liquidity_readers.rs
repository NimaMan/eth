//! Liquidity Readers Smoke Test
//!
//! Queries normalized liquidity info for known AMM pools using
//! RethQueryProvider::get_route_liquidity and prints results.

use alloy_primitives::address;
use eyre::Result;
use reth_chain_query::{Address, AmmSwapRoute, RethQueryProvider, B256};

fn default_reth_db() -> String {
    std::env::var("RETH_DB_PATH").unwrap_or_else(|_| {
        format!(
            "{}/.local/share/reth/mainnet",
            std::env::var("HOME").unwrap_or_else(|_| "/home/nima".into())
        )
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("🔎 Liquidity Readers Smoke Test");
    println!("{}", "=".repeat(60));

    let reth_db = default_reth_db();
    let provider = RethQueryProvider::new(&reth_db)?;
    println!(
        "Reth DB: {} | latest {}",
        reth_db,
        provider.get_latest_block()?
    );

    // Known pools
    let uni_v2_usdc_weth: Address = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
    let sushi_v2_usdc_weth: Address = address!("397FF1542f962076d0BFE58eA045FfA2d347ACa0");
    let uni_v3_weth_usdc_500: Address = address!("88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640");
    let uni_v3_weth_usdc_3000: Address = address!("8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8");
    let uni_v3_usdc_usdt_100: Address = address!("3416cF6C708Da44DB2624D63ea0AAef7113527C6");

    // Balancer V2 poolId: WETH/USDC weighted pool (example)
    // 0x96646936b91d6b9d7d0c47c496afbf3d6ec7b6f8000200000000000000000019
    let bal_weth_usdc_pool_id = B256::from_slice(&[
        0x96, 0x64, 0x69, 0x36, 0xb9, 0x1d, 0x6b, 0x9d, 0x7d, 0x0c, 0x47, 0xc4, 0x96, 0xaf, 0xbf,
        0x3d, 0x6e, 0xc7, 0xb6, 0xf8, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x19,
    ]);

    // Curve V1 TriCrypto pool (USDT/WBTC/WETH)
    let curve_tricrypto_pool: Address = address!("D51a44d3FaE010294C616388b506AcdA1bfAAE46");

    let routes = vec![
        AmmSwapRoute::UniswapV2 {
            pool: uni_v2_usdc_weth,
        },
        AmmSwapRoute::SushiswapV2 {
            pool: sushi_v2_usdc_weth,
        },
        AmmSwapRoute::UniswapV3 {
            pool: uni_v3_weth_usdc_500,
            fee_tier: 500,
        },
        AmmSwapRoute::UniswapV3 {
            pool: uni_v3_weth_usdc_3000,
            fee_tier: 3000,
        },
        AmmSwapRoute::UniswapV3 {
            pool: uni_v3_usdc_usdt_100,
            fee_tier: 100,
        },
        // Balancer: query pair reserves for WETH/USDC within the pool
        AmmSwapRoute::BalancerV2 {
            pool_id: bal_weth_usdc_pool_id,
            token_in: address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
            token_out: address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        },
        // Curve: TriCrypto USDT (i=0) vs WETH (j=2)
        AmmSwapRoute::CurveV1 {
            pool: curve_tricrypto_pool,
            i: 0,
            j: 2,
            use_underlying: false,
        },
    ];

    for route in routes {
        let info = provider.get_route_liquidity(&route, None).await?;
        println!("\nProtocol: {}", info.protocol);
        println!("Pool:     0x{:x}", info.pool);
        if let (Some(t0), Some(t1)) = (info.token0, info.token1) {
            println!(
                "Tokens:   0x{:x} ({:?}) / 0x{:x} ({:?})",
                t0, info.token0_symbol, t1, info.token1_symbol
            );
        }
        match info.protocol {
            "Uniswap-V2" | "SushiswapV2" | "Balancer-V2" | "Curve-V1" => {
                println!(
                    "Reserves: reserve0={} reserve1={}",
                    info.reserve0.unwrap_or_default(),
                    info.reserve1.unwrap_or_default()
                );
            }
            "Uniswap-V3" | "Uniswap-V4" => {
                println!(
                    "Liquidity: {} | tick={:?}",
                    info.v3_liquidity.unwrap_or_default(),
                    info.tick
                );
            }
            _ => {}
        }
        println!("Block:    {}", info.block_number);
    }

    println!("\n✅ Liquidity reader checks completed.");
    Ok(())
}
