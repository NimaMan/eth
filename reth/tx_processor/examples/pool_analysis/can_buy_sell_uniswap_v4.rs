/// Uniswap V4 Pool Viability (Scaffold)
///
/// This example exercises the PoolType::UniswapV4 path through the simulator and
/// prints a clear failure reason until full V4 swap support is implemented
/// (PoolManager lock/unlock + Router integration).

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, B256, U256};
use std::str::FromStr;
use tx_processor::simulator::{
    check_can_buy_sell_pool,
    PoolViabilityConfig,
    PoolType,
};
use tx_simulator::TxSimulator;
use tx_processor::tx_processor::TxProcessor;
// Read-only V4 PoolManager logic will live in reth_chain_query; placeholder here

#[tokio::main]
async fn main() -> Result<()> {
    println!("Uniswap V4 Pool Viability (Scaffold)");
    println!("====================================\n");

    // Reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let processor = Arc::new(TxProcessor::new());

    let latest_block = simulator.get_latest_block()?;
    println!("Latest block: {}", latest_block);

    // PoolManager mainnet address from Uniswap v4 core
    let pool_manager = Address::from_str("0x000000000004444C5DC75cB358380d2E3de08a90").unwrap_or(Address::ZERO);
    // Dummy PoolId (32-bytes zero) — not used yet in the check path
    let _pool_id = B256::ZERO;

    // Common tokens (similar to V2/V3 examples)
    let tokens: &[(&str, &str, u8)] = &[
        ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
        ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7", 6),
        ("DAI",  "0x6B175474E89094C44Da98b954EedeAC495271d0F", 18),
        ("UNI",  "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984", 18),
        ("LINK", "0x514910771AF9Ca656af840dff83E8264EcF986CA", 18),
        ("WBTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", 8),
        ("PEPE", "0x6982508145454Ce325dDbE47a25d4ec3d2311933", 18),
    ];

    println!("Running V4 viability checks for common tokens (expected descriptive failures)\n");
    for (i, (sym, token_str, decimals)) in tokens.iter().enumerate() {
        println!("{}", "-".repeat(60));
        println!("[{}] {} ({})", i + 1, sym, token_str);

        let token = match Address::from_str(token_str) {
            Ok(a) => a,
            Err(e) => {
                println!("  Invalid token address: {}", e);
                continue;
            }
        };

        let config = PoolViabilityConfig {
            token_address: token,
            pool_address: pool_manager, // reuse pool_address to carry PoolManager
            pool_type: PoolType::UniswapV4,
            test_amount: U256::from(10_000_000_000_000_000u64), // 0.01 ETH
            buyer_address: Address::from([
                0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8,
                0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D, 0x80, 0x4a,
                0x24, 0xbd, 0x56, 0x89,
            ]),
            prior_tx: None,
            block_number: Some(latest_block),
            slippage_tolerance: 0.5,
            gas_limit: 500_000,
            gas_price: 30_000_000_000,
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D,
                0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA, 0xD9, 0x08,
                0x3C, 0x75, 0x6C, 0xc2
            ]),
            block_delay: 0,
            token_decimals: *decimals,
        };

        match check_can_buy_sell_pool(simulator.clone(), processor.clone(), config).await {
            Ok(res) => {
                println!("  Pool type: {:?}", res.pool_type);
                println!("  Can buy: {}", res.can_buy);
                println!("  Can approve: {}", res.can_approve);
                println!("  Can sell: {}", res.can_sell);
                println!("  Buy tax %: {:.2}", res.buy_tax_percent);
                println!("  Sell tax %: {:.2}", res.sell_tax_percent);
                if let Some(reason) = res.failure_reason {
                    println!("  Failure reason: {}", reason);
                }
            }
            Err(e) => println!("  Error: {}", e),
        }
    }

    println!("\nOK - V4 path exercised for common tokens. For read-only PoolManager state, we'll wire a helper in reth_chain_query next.");
    Ok(())
}
