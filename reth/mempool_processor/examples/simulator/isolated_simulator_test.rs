/// Isolated Simulation Test (no Signal Manager)
///
/// Runs a simple pool buy/sell simulation using a shared TxSimulator
/// to verify the simulation step of the pipeline works without any
/// signal manager or queue orchestration.

use std::sync::Arc;
use std::str::FromStr;
use clap::Parser;
use eyre::Result;
use tracing::info;

use alloy_primitives::Address;

use mempool_processor::simulator::MempoolSimulator;
use tx_simulator::TxSimulator;

#[derive(Parser, Debug)]
struct Args {
    /// Reth database path (datadir)
    #[arg(long, env = "RETH_DB_PATH", default_value = "/home/nima/.local/share/reth/mainnet")]
    reth_db_path: String,

    /// Token address to test (default: USDC)
    #[arg(long, default_value = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")]
    token: String,

    /// Pool address to test (default: USDC/WETH V2)
    #[arg(long, default_value = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc")]
    pool: String,

    /// Optional block number (default: latest)
    #[arg(long)]
    block: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("Starting isolated simulation (no signal manager)...");

    // Parse addresses
    let token_address = Address::from_str(&args.token)?;
    let pool_address = Address::from_str(&args.pool)?;

    // Create a single shared simulator for all components
    let shared = Arc::new(TxSimulator::new(&args.reth_db_path)?);
    let mempool_simulator = MempoolSimulator::from_shared_simulator(shared.clone())?;

    let latest = mempool_simulator.get_latest_block()?;
    println!("Config:");
    println!("  Reth DB: {}", args.reth_db_path);
    println!("  Latest block: {}", latest);
    println!("  Token: {:?}", token_address);
    println!("  Pool:  {:?}", pool_address);
    println!("  Block: {:?}", args.block.unwrap_or(latest));

    // Run simple pool simulation to verify DB sharing + simulation
    match mempool_simulator
        .simulate_pool_buy_sell_simple(token_address, pool_address, args.block)
        .await
    {
        Ok(result) => {
            println!("\n✅ Simulation succeeded");
            println!("  Can Buy: {}", result.can_buy);
            println!("  Can Approve: {}", result.can_approve);
            println!("  Can Sell: {}", result.can_sell);
            println!("  Tradeable: {}", result.is_tradeable);
            if result.buy_tax_percent >= 0.0 {
                println!("  Buy Tax: {:.2}%", result.buy_tax_percent);
            }
            if result.sell_tax_percent >= 0.0 {
                println!("  Sell Tax: {:.2}%", result.sell_tax_percent);
            }
        }
        Err(e) => {
            println!("\n❌ Simulation failed: {}", e);
            println!("  (If using a pruned node, historical blocks may be pruned)");
        }
    }

    Ok(())
}
