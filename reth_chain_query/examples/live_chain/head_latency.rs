use std::time::Duration;

use clap::Parser;
use color_eyre::Result;
use reth_chain_query::live_chain::HeadLatencyTracker;

#[derive(Debug, Parser)]
struct Args {
    #[arg(
        long,
        default_value = "http://127.0.0.1:5052",
        help = "Beacon REST base URL (e.g. http://127.0.0.1:5052)"
    )]
    beacon_api: String,

    #[arg(
        long,
        default_value = "http://127.0.0.1:8545",
        help = "Execution RPC endpoint (HTTP or engine proxy)"
    )]
    execution_rpc: String,

    #[arg(
        long,
        default_value = "120",
        help = "How long to collect samples (seconds)"
    )]
    duration: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    let tracker = HeadLatencyTracker::new(&args.beacon_api, &args.execution_rpc)?;
    let samples = tracker
        .measure_for_duration(Duration::from_secs(args.duration))
        .await?;

    if samples.is_empty() {
        println!("No head samples collected");
        return Ok(());
    }

    println!("Collected {} head samples", samples.len());
    let mut delays: Vec<f64> = samples
        .iter()
        .filter_map(|s| s.rpc_fetch_delay_secs)
        .collect();
    delays.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if !delays.is_empty() {
        let avg: f64 = delays.iter().sum::<f64>() / delays.len() as f64;
        let median = delays[delays.len() / 2];
        println!(
            "RPC readiness delay: mean {:.3}s median {:.3}s min {:.3}s max {:.3}s",
            avg,
            median,
            delays.first().unwrap(),
            delays.last().unwrap()
        );
    }

    for sample in samples {
        println!(
            "slot={} block={:?} arrival={} fetch_delay={:?}",
            sample.observation.slot,
            sample.observation.block_root,
            sample.observation.arrival_time,
            sample.rpc_fetch_delay_secs
        );
    }

    Ok(())
}
