use std::{collections::HashMap, str::FromStr};

use alloy_primitives::B256;
use chrono::{DateTime, Utc};
use clap::Parser;
use color_eyre::Result;
use jsonrpsee::{core::client::SubscriptionClientT, rpc_params, ws_client::WsClientBuilder};
use reth_chain_query::live_chain::{beacon::BeaconRestClient, lighthouse::LighthouseHeadListener};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::warn;

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
        default_value = "ws://127.0.0.1:8546",
        help = "Execution websocket endpoint (reth --ws ...)"
    )]
    execution_ws: String,

    #[arg(
        long,
        default_value_t = 10,
        help = "Number of head samples to collect before exiting"
    )]
    samples: usize,
}

#[derive(Clone, Copy, Debug)]
enum HeadSource {
    BeaconSse,
    ExecutionWs,
}

#[derive(Clone, Debug)]
struct ArrivalEvent {
    hash: B256,
    number: Option<u64>,
    slot: Option<u64>,
    arrival: DateTime<Utc>,
    source: HeadSource,
}

#[derive(Default)]
struct ArrivalAccumulator {
    slot: Option<u64>,
    number: Option<u64>,
    beacon_time: Option<DateTime<Utc>>,
    ws_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct WsHead {
    hash: String,
    number: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    if args.samples == 0 {
        println!("Sample count must be > 0");
        return Ok(());
    }

    let (tx, mut rx) = mpsc::channel(256);
    let beacon_task = tokio::spawn(run_beacon_stream(args.beacon_api.clone(), tx.clone()));
    let ws_task = tokio::spawn(run_ws_stream(args.execution_ws.clone(), tx.clone()));

    let mut records: HashMap<B256, ArrivalAccumulator> = HashMap::new();
    let mut deltas = Vec::new();
    let mut completed = 0usize;

    println!(
        "Measuring head arrival differences using SSE ({}) vs websocket ({})",
        args.beacon_api, args.execution_ws
    );

    while let Some(event) = rx.recv().await {
        let entry = records
            .entry(event.hash)
            .or_insert_with(ArrivalAccumulator::default);
        if entry.number.is_none() {
            entry.number = event.number;
        }
        if entry.slot.is_none() {
            entry.slot = event.slot;
        }

        match event.source {
            HeadSource::BeaconSse => entry.beacon_time = Some(event.arrival),
            HeadSource::ExecutionWs => entry.ws_time = Some(event.arrival),
        }

        if let (Some(beacon_time), Some(ws_time)) = (entry.beacon_time, entry.ws_time) {
            let diff = ws_time - beacon_time;
            let diff_secs = (diff.num_microseconds().unwrap_or(0) as f64) / 1_000_000f64;
            let block_desc = entry
                .number
                .map(|n| n.to_string())
                .or_else(|| entry.slot.map(|s| format!("slot {}", s)))
                .unwrap_or_else(|| "unknown".into());
            println!(
                "Block {} hash {hash:#x}: websocket minus SSE = {diff_secs:.3}s (ws {} vs beacon {})",
                block_desc,
                entry.ws_time.unwrap(),
                entry.beacon_time.unwrap(),
                hash = event.hash
            );
            deltas.push(diff_secs);
            completed += 1;
            records.remove(&event.hash);
            if completed >= args.samples {
                break;
            }
        }
    }

    // Stop background tasks.
    beacon_task.abort();
    ws_task.abort();

    if deltas.is_empty() {
        println!("No paired head arrivals collected.");
        return Ok(());
    }

    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg = deltas.iter().sum::<f64>() / deltas.len() as f64;
    let median = deltas[deltas.len() / 2];
    let min = deltas.first().cloned().unwrap_or(0.0);
    let max = deltas.last().cloned().unwrap_or(0.0);
    println!(
        "Collected {} samples. Websocket - SSE delay stats: mean {:.3}s median {:.3}s min {:.3}s max {:.3}s",
        deltas.len(),
        avg,
        median,
        min,
        max
    );

    Ok(())
}

async fn run_beacon_stream(beacon_api: String, tx: mpsc::Sender<ArrivalEvent>) -> Result<()> {
    let beacon = BeaconRestClient::new(&beacon_api)?;
    let events_url = beacon.sse_endpoint();
    let mut listener = LighthouseHeadListener::connect(&events_url).await?;
    loop {
        match listener.next_observation().await {
            Ok(obs) => match beacon.fetch_execution_info(obs.block_root).await {
                Ok(exec) => {
                    let event = ArrivalEvent {
                        hash: exec.block_hash,
                        number: Some(exec.block_number),
                        slot: Some(obs.slot),
                        arrival: obs.arrival_time,
                        source: HeadSource::BeaconSse,
                    };
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
                Err(err) => warn!(
                    "failed to fetch execution info for slot {}: {}",
                    obs.slot, err
                ),
            },
            Err(err) => {
                warn!("lighthouse SSE listener error: {}", err);
            }
        }
    }
    Ok(())
}

async fn run_ws_stream(ws_url: String, tx: mpsc::Sender<ArrivalEvent>) -> Result<()> {
    let client = WsClientBuilder::default().build(&ws_url).await?;
    let mut sub = client
        .subscribe::<WsHead, _>("eth_subscribe", rpc_params!["newHeads"], "eth_unsubscribe")
        .await?;

    while let Some(message) = sub.next().await {
        let head = match message {
            Ok(value) => value,
            Err(err) => {
                warn!("websocket subscription error: {}", err);
                continue;
            }
        };

        let hash = match B256::from_str(&head.hash) {
            Ok(value) => value,
            Err(err) => {
                warn!("invalid block hash {}: {}", head.hash, err);
                continue;
            }
        };
        let number = parse_hex_u64_opt(head.number.as_deref());
        let arrival = Utc::now();
        let event = ArrivalEvent {
            hash,
            number,
            slot: None,
            arrival,
            source: HeadSource::ExecutionWs,
        };
        if tx.send(event).await.is_err() {
            break;
        }
    }

    Ok(())
}

fn parse_hex_u64_opt(input: Option<&str>) -> Option<u64> {
    input.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        let digits = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        u64::from_str_radix(if digits.is_empty() { "0" } else { digits }, 16).ok()
    })
}
