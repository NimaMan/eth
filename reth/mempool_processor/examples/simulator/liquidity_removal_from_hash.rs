/// Simulate liquidity removal for a given transaction hash using the dedicated
/// LiquidityRemovalSimulator, and print the computed drain metrics.
///
/// Usage:
///   cargo run -p mempool_processor --example liquidity_removal_from_hash \
///     -- --tx-hash <TX_HASH> [--reth-db-path /home/nima/.local/share/reth/mainnet]
///
use clap::Parser;
use eyre::Result;
use std::str::FromStr;
use std::sync::Arc;
use alloy_primitives::{Address, B256, Bytes, U256};
use reth_chain_query::to_checksum_address;

use mempool_processor::simulator::liquidity_removal_simulator::LiquidityRemovalSimulator;
use mempool_processor::token_tracking::TokenTrackingSubscriber;
use tx_simulator::{TxSimulator, UnsignedTransaction};
use tx_processor::ProcessedTxProvider;

#[derive(Parser, Debug)]
struct Args {
    /// Transaction hash to analyze
    #[arg(long, value_name = "B256")]
    tx_hash: String,

    /// Reth DB path
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_db_path: String,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_env_filter("debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_env_filter("info")
            .init();
    }

    let tx_hash = B256::from_str(&args.tx_hash)?;

    println!("🔍 Liquidity Removal Simulator (by tx hash)");
    println!("DB: {}", args.reth_db_path);
    println!("TX: 0x{}\n", hex::encode(tx_hash));

    // Shared simulator
    let simulator = Arc::new(TxSimulator::new(&args.reth_db_path)?);

    // Get processed tx to reconstruct the unsigned call and to cross-check deltas
    let provider = ProcessedTxProvider::with_provider_factory(simulator.provider_factory().clone())?;
    let processed = provider.process_transaction_by_hash(tx_hash).await?;

    // Reconstruct UnsignedTransaction from processed
    let unsigned = UnsignedTransaction {
        from: Some(processed.from_address),
        to: processed.to_address,
        gas: None, // let simulator pick a safe limit
        gas_price: None, // base fee aware selection
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(processed.value),
        data: Some(Bytes::from(processed.input.clone())),
        nonce: Some(processed.nonce),
    };

    // Use block-1 to get proper pre-state
    let sim_block = processed.block_number.saturating_sub(1);

    // Start token tracking subscriber to populate the cache (requires Python publisher on 5557/5558)
    let mut subscriber = TokenTrackingSubscriber::new(0.05);
    let cache = subscriber.get_cache();
    tokio::spawn(async move {
        if let Err(e) = subscriber.start_listening().await {
            eprintln!("TokenTrackingSubscriber error: {}", e);
        }
    });
    // Give it a moment to request initial state
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Create removal simulator (reuses same TxSimulator) and attach token cache
    let mut lr = LiquidityRemovalSimulator::new(simulator.clone());
    lr.set_token_cache(cache);

    // NOTE: Token cache optional; not required to get a pool candidate, but helps confirm
    // if this address is a tracked pool with known reserves. We skip here for simplicity.

    let result = lr.simulate_removal(unsigned, Some(sim_block)).await?;

    println!("✅ LiquidityRemovalResult:");
    println!("  success: {}", result.success);
    if let Some(r) = &result.revert_reason { println!("  revert_reason: {}", r); }
    println!("  pool_address: {}", result.pool_address.map(|a| to_checksum_address(&a)).unwrap_or("None".into()));
    println!("  eth_removed: {:.6}", result.eth_removed);
    println!("  drain_percentage: {:.2}%", result.drain_percentage);
    println!("  remaining_eth: {:.6}", result.remaining_eth);
    println!("  is_scam: {}", result.is_scam);
    println!("  address_balance_changes_addrs: {}\n", result.address_balance_changes.len());

    // Quick cross-check: top ETH deltas from processed tx
    println!("Top ETH balance deltas from processed tx:");
    let mut entries: Vec<_> = processed.address_balance_changes.iter().collect();
    entries.sort_by(|a, b| {
        let av = a.1.currency_net.get("ETH").cloned().unwrap_or(U256::ZERO);
        let bv = b.1.currency_net.get("ETH").cloned().unwrap_or(U256::ZERO);
        // compare by absolute value of signed interpretation
        let ai = alloy_primitives::I256::try_from(av).unwrap_or(alloy_primitives::I256::ZERO).unsigned_abs();
        let bi = alloy_primitives::I256::try_from(bv).unwrap_or(alloy_primitives::I256::ZERO).unsigned_abs();
        bi.cmp(&ai)
    });
    for (addr, ch) in entries.iter().take(10) {
        let raw = ch.currency_net.get("ETH").cloned().unwrap_or(U256::ZERO);
        let signed = alloy_primitives::I256::try_from(raw).unwrap_or(alloy_primitives::I256::ZERO);
        let eth = signed.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  {}: {:+.6} ETH", to_checksum_address(addr), eth);
    }

    Ok(())
}
