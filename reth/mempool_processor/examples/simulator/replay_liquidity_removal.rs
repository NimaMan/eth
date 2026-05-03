//! Replay a liquidity-removal transaction through the same simulator used in the live pipeline.
//! Flow:
//! 1) Load the tx from local Reth DB (ProcessedTxProvider)
//! 2) Rebuild the unsigned tx
//! 3) Simulate at block-1 (pre-state) with LiquidityRemovalSimulator
//! 4) Retry at block if needed
//! 5) Print drain metrics + top ETH deltas
//!
//! Usage:
//!   cargo run -p mempool_processor --example replay_liquidity_removal \
//!     -- --tx 0xa6e069cb77a177d7adadf23bfd6fa043a7430e321dc8f0f2d2b45d7ce57f6239 \
//!     --datadir ~/.local/share/reth/mainnet

use alloy_primitives::{B256, I256};
use clap::Parser;
use eyre::Result;
use mempool_processor::simulator::liquidity_removal_simulator::LiquidityRemovalSimulator;
use reth_chain_query::to_checksum_address;
use std::str::FromStr;
use std::sync::Arc;
use tx_processor::{ProcessedTxProvider, UnsignedTxBuilder};
use tx_simulator::TxSimulator;

#[derive(Parser, Debug)]
struct Args {
    /// Liquidity removal tx hash to replay
    #[arg(long, value_name = "B256")]
    tx: String,

    /// Reth DB path
    #[arg(
        long,
        env = "RETH_DB_PATH",
        default_value = "/home/nima/.local/share/reth/mainnet"
    )]
    datadir: String,
}

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, stripped);
        }
    }
    path.to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let tx_hash = B256::from_str(&args.tx)?;
    let datadir = expand_tilde(&args.datadir);

    println!("Datadir: {}", datadir);
    println!("Tx hash: {:?}", tx_hash);

    // Core simulator + providers
    let simulator = Arc::new(TxSimulator::new(&datadir)?);
    let processed_provider =
        ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
    let processed = processed_provider
        .process_transaction_by_hash(tx_hash)
        .await?;

    let unsigned = UnsignedTxBuilder::build_unsigned_from_processed_tx(&processed);
    let sim_block = processed.block_number.saturating_sub(1);

    let lr = LiquidityRemovalSimulator::new(simulator.clone());

    // Try pre-state first
    let mut result = lr
        .simulate_removal(unsigned.clone(), Some(sim_block), None)
        .await?;
    if !result.success {
        result = lr
            .simulate_removal(unsigned.clone(), Some(processed.block_number), None)
            .await?;
    }

    println!("\n✅ LiquidityRemovalResult:");
    println!("  success: {}", result.success);
    if let Some(r) = &result.revert_reason {
        println!("  revert_reason: {}", r);
    }
    println!(
        "  pool_address: {}",
        result
            .pool_address
            .map(|a| to_checksum_address(&a))
            .unwrap_or_else(|| "None".into())
    );
    println!("  eth_removed: {:.6}", result.eth_removed);
    println!("  drain_percentage: {:.2}%", result.drain_percentage);
    println!("  remaining_eth: {:.6}", result.remaining_eth);
    println!("  is_scam: {}", result.is_scam);
    println!(
        "  address_balance_changes_addrs: {}",
        result.address_balance_changes.len()
    );
    if let Some(info) = &result.debug_info {
        println!("  debug_info: {}", info);
    }

    // Cross-check: top ETH deltas from processed tx
    println!("\nTop ETH balance deltas from processed tx:");
    let mut entries: Vec<_> = processed.address_balance_changes.iter().collect();
    entries.sort_by(|a, b| {
        let av = a.1.currency_net.get("ETH").copied().unwrap_or(I256::ZERO);
        let bv = b.1.currency_net.get("ETH").copied().unwrap_or(I256::ZERO);
        let ai = av.unsigned_abs();
        let bi = bv.unsigned_abs();
        bi.cmp(&ai)
    });
    for (addr, ch) in entries.iter().take(10) {
        let signed = ch.currency_net.get("ETH").copied().unwrap_or(I256::ZERO);
        let eth = signed.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  {}: {:+.6} ETH", to_checksum_address(addr), eth);
    }

    // Avoid dropping blocking runtimes within async context
    std::mem::forget(simulator);
    std::mem::forget(processed_provider);

    Ok(())
}
