//! Demonstrates a denomination token (USDC) to target token (DAI) viability simulation on Uniswap V3.
//!
//! The example fetches a historical pool snapshot and runs the buy → approve → sell sequence using
//! the new token-to-token pipeline. It relies on a well-funded Binance hot wallet as the buyer so
//! the simulation has sufficient USDC balance to spend.

use alloy_primitives::{address, Address, U256};
use eyre::{Result, WrapErr};
use reth_chain_query::dex::uniswap_v3::compute_uniswap_v3_pool;
use std::sync::Arc;
use tx_processor::simulator::{check_can_buy_sell_pool, PoolBuySellParameters, PoolType};
use tx_processor::tx_processor::TxProcessor;
use tx_simulator::TxSimulator;

const USDC_ADDRESS: Address = address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
const DAI_ADDRESS: Address = address!("0x6B175474E89094C44Da98b954EedeAC495271d0F");

// Binance 14 hot wallet – historically rich in both USDC and DAI balances.
const BUYER_ADDRESS: Address = address!("0x28C6c06298d514Db089934071355E5743bf21d60");

const USDC_DECIMALS: u8 = 6;
const DAI_DECIMALS: u8 = 18;
const TEST_AMOUNT_USDC: u64 = 10_000_000; // 10 USDC (in 6 decimal units)
const ANCHOR_BLOCK: u64 = 20_000_000;
const FEE_TIER_BPS: u32 = 500;

#[tokio::main]
async fn main() -> Result<()> {
    let pool_address = compute_uniswap_v3_pool(USDC_ADDRESS, DAI_ADDRESS, FEE_TIER_BPS);

    println!("🔁 USDC → DAI Uniswap V3 viability check (fee tier 0.05%)");
    println!("Pool : {pool_address:#x}");
    println!("Buyer: {BUYER_ADDRESS:#x}");
    println!("Block: {ANCHOR_BLOCK}\n");

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());

    let config = PoolBuySellParameters::new(
        DAI_ADDRESS,
        pool_address,
        PoolType::UniswapV3 {
            fee_tier: FEE_TIER_BPS,
        },
    )
    .with_test_amount(U256::from(TEST_AMOUNT_USDC))
    .with_denom_address(USDC_ADDRESS)
    .with_denom_decimals(USDC_DECIMALS)
    .with_token_decimals(DAI_DECIMALS)
    .with_buyer(BUYER_ADDRESS)
    .with_block(ANCHOR_BLOCK);

    let result = check_can_buy_sell_pool(simulator, tx_processor, config)
        .await
        .wrap_err("Simulation failed")?;

    println!("✅ Simulation complete");
    println!("  Can Buy     : {}", result.can_buy);
    println!("  Can Approve : {}", result.can_approve);
    println!("  Can Sell    : {}", result.can_sell);
    println!("  Tradeable   : {}", result.is_tradeable);
    println!("  Buy Tax %   : {:.3}", result.buy_tax_percent);
    println!("  Sell Tax %  : {:.3}", result.sell_tax_percent);

    let usdc_spent = result.denom_spent.to::<u128>() as f64 / 10f64.powi(USDC_DECIMALS as i32);
    let usdc_returned =
        result.denom_received.to::<u128>() as f64 / 10f64.powi(USDC_DECIMALS as i32);
    let dai_obtained = result.tokens_received.to::<u128>() as f64 / 10f64.powi(DAI_DECIMALS as i32);

    println!("\n📈 Amounts");
    println!("  USDC spent   : {usdc_spent:.6}");
    println!("  DAI received : {dai_obtained:.6}");
    println!("  USDC returned: {usdc_returned:.6}");

    if let Some(reason) = result.failure_reason.as_ref() {
        println!("\n⚠️  Failure reason: {reason}");
    }

    println!("\nTo inspect raw transactions, rerun with:");
    println!("  BUY trace    : {:?}", result.buy_transaction.hash);
    println!("  APPROVE trace: {:?}", result.approve_transaction.hash);
    println!("  SELL trace   : {:?}", result.sell_transaction.hash);

    Ok(())
}
