/// Liquidity Removal Detection (up to simulation only)
///
/// This simplified example fetches a transaction by hash via JSON-RPC,
/// converts it into an unsigned call, and runs the dedicated
/// LiquidityRemovalSimulator to verify that simulation works without
/// the full signal manager pipeline.
use eyre::Result;
use std::sync::Arc;
use tracing::{error, info, Level};

use mempool_processor::common::convert::ipc_to_unsigned_tx;
use mempool_processor::simulator::liquidity_removal_simulator::LiquidityRemovalSimulator;
use tx_simulator::TxSimulator;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🔍 Testing Liquidity Removal Simulation (no signal manager)");

    // Config
    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let rpc_url =
        std::env::var("ETH_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());

    // Test transactions
    let test_cases = vec![
        (
            "0x6e115e08f7832ba8291e507c3b44b529163ef1302cba8aa39a433ba700e2b214",
            "MEV bot with 0 balance",
        ),
        (
            "0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966",
            "99.9% drain (should be detected)",
        ),
        (
            "0x88c91cac79fdabdef1fde0933ab8afc13c094e6fdf556375f81d3a306915dc4b",
            "Known scam transaction",
        ),
    ];

    // Shared simulator + liquidity removal simulator
    let shared = Arc::new(TxSimulator::new(&datadir)?);
    let liquidity_sim = LiquidityRemovalSimulator::new(shared.clone());

    // JSON-RPC client
    use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
    let rpc = HttpClientBuilder::default().build(&rpc_url)?;

    for (tx_hash, label) in test_cases {
        info!("\n============================================================");
        info!("Testing: {}", label);
        info!("TX: {}", tx_hash);

        // Fetch raw tx
        let tx_data: serde_json::Value = match rpc
            .request("eth_getTransactionByHash", rpc_params![tx_hash])
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("RPC request failed: {}", e);
                continue;
            }
        };

        if tx_data.is_null() {
            error!("Transaction not found or RPC unavailable");
            continue;
        }

        // Convert to unsigned tx
        let unsigned = match ipc_to_unsigned_tx(&tx_data) {
            Ok(u) => u,
            Err(e) => {
                error!("Failed to convert tx: {}", e);
                continue;
            }
        };

        // Detect the original block number to avoid nonce/basefee validation issues
        let block_opt = tx_data
            .get("blockNumber")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());
        if let Some(block) = block_opt {
            info!("🧪 Simulating liquidity removal at tx block {}...", block);
            match liquidity_sim
                .simulate_removal(unsigned.clone(), Some(block))
                .await
            {
                Ok(res) => {
                    println!("\n✅ Removal simulation (at block {})", block);
                    println!("  Success: {}", res.success);
                    println!(
                        "  Pool:   {:?}",
                        res.pool_address.map(|a| format!("0x{}", hex::encode(a)))
                    );
                    println!("  ETH Removed: {:.4}", res.eth_removed);
                    println!("  Remaining:   {:.4} ETH", res.remaining_eth);
                    println!("  Drained:     {:.2}%", res.drain_percentage);
                    println!("  Scam:        {}", res.is_scam);
                }
                Err(e) => {
                    println!("\n❌ Removal simulation at block {} failed: {}", block, e);
                    // Fall back to latest if pruned
                    info!("Falling back to latest block simulation...");
                    match liquidity_sim.simulate_removal(unsigned, None).await {
                        Ok(res) => {
                            println!("\n✅ Removal simulation complete (latest)");
                            println!("  Success: {}", res.success);
                            println!(
                                "  Pool:   {:?}",
                                res.pool_address.map(|a| format!("0x{}", hex::encode(a)))
                            );
                            println!("  ETH Removed: {:.4}", res.eth_removed);
                            println!("  Remaining:   {:.4} ETH", res.remaining_eth);
                            println!("  Drained:     {:.2}%", res.drain_percentage);
                            println!("  Scam:        {}", res.is_scam);
                        }
                        Err(e2) => println!("\n❌ Removal simulation failed (latest): {}", e2),
                    }
                }
            }
            continue;
        }

        // If no block is present, simulate at latest
        // Simulate liquidity removal
        info!("🧪 Simulating liquidity removal...");
        match liquidity_sim.simulate_removal(unsigned, None).await {
            Ok(res) => {
                println!("\n✅ Removal simulation complete");
                println!("  Success: {}", res.success);
                println!(
                    "  Pool:   {:?}",
                    res.pool_address.map(|a| format!("0x{}", hex::encode(a)))
                );
                println!("  ETH Removed: {:.4}", res.eth_removed);
                println!("  Remaining:   {:.4} ETH", res.remaining_eth);
                println!("  Drained:     {:.2}%", res.drain_percentage);
                println!("  Scam:        {}", res.is_scam);
            }
            Err(e) => {
                println!("\n❌ Removal simulation failed: {}", e);
            }
        }
    }

    Ok(())
}
