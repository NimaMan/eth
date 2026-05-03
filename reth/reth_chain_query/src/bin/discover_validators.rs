use std::{collections::HashMap, path::PathBuf, process::Command};

use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_eth::BlockNumberOrTag;
use clap::Parser;
use eyre::{bail, Result};
use reth_chain_query::common_addresses::validators::FEE_RECIPIENT_LIST;
use reth_chain_query::to_checksum_address;
use tokio::time::{sleep, Duration};

#[derive(Parser, Debug)]
#[command(about = "Discover new validator or builder addresses", version)]
struct Args {
    /// Number of recent blocks to scan
    #[arg(long, default_value_t = 10_000)]
    blocks: u64,
    /// RPC endpoint to query (defaults to ETH_RPC_URL or http://localhost:8545)
    #[arg(long)]
    rpc: Option<String>,
    /// Append new entries to validators.rs and run cargo fmt
    #[arg(long)]
    write: bool,
    /// Milliseconds to sleep between block batches (prevents rate limiting)
    #[arg(long, default_value_t = 0)]
    sleep_ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().ok();
    let args = Args::parse();
    let rpc_url = args
        .rpc
        .or_else(|| std::env::var("ETH_RPC_URL").ok())
        .unwrap_or_else(|| "http://localhost:8545".to_string());

    println!("Connecting to {}", rpc_url);
    let provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);

    let latest = provider.get_block_number().await?;
    if latest == 0 {
        println!("RPC returned block height 0; aborting");
        return Ok(());
    }
    let blocks = args.blocks.min(latest + 1);
    let start = latest + 1 - blocks;
    println!("Scanning blocks {} -> {} ({} total)", start, latest, blocks);

    let mut counts: HashMap<Address, u64> = HashMap::new();
    let mut number = latest;
    let batch_size = 100u64;

    while number >= start {
        let batch_start = number.saturating_sub(batch_size - 1);
        let mut handles = Vec::new();
        for block_num in (batch_start..=number).rev() {
            let provider = provider.clone();
            handles.push(tokio::spawn(async move {
                provider
                    .get_block_by_number(BlockNumberOrTag::Number(block_num))
                    .await
            }));
        }

        for handle in handles {
            if let Ok(Ok(Some(block))) = handle.await {
                *counts.entry(block.header.beneficiary).or_default() += 1;
            }
        }

        if args.sleep_ms > 0 {
            sleep(Duration::from_millis(args.sleep_ms)).await;
        }

        if batch_start == start {
            break;
        }
        number = batch_start - 1;
    }

    let existing: std::collections::HashSet<_> = FEE_RECIPIENT_LIST
        .iter()
        .map(|entry| entry.address)
        .collect();

    let mut new_entries: Vec<_> = counts
        .into_iter()
        .filter(|(addr, _)| !existing.contains(addr))
        .collect();

    if new_entries.is_empty() {
        println!("No new validators detected.");
        return Ok(());
    }

    new_entries.sort_by(|a, b| b.1.cmp(&a.1));

    if args.write {
        update_validators_file(&new_entries)?;
        run_cargo_fmt()?;
        println!(
            "Inserted {} new entries into validators.rs. Rebuild pyreth to propagate changes.",
            new_entries.len()
        );
    } else {
        println!("Discovered new validators (rerun with --write to append):");
        for (addr, count) in &new_entries {
            println!(
                "  {} | {:>4} blocks | {}",
                to_checksum_address(addr),
                count,
                default_builder_name(addr)
            );
        }
    }

    Ok(())
}

fn default_builder_name(address: &Address) -> String {
    let checksum = to_checksum_address(address);
    match checksum.as_str() {
        "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5" => "beaverbuild".to_string(),
        "0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5" => "Flashbots: Builder".to_string(),
        "0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97" => "Titan Builder".to_string(),
        _ => format!(
            "MEV Builder: {}...{}",
            &checksum[0..6],
            &checksum[checksum.len() - 3..]
        ),
    }
}

fn validators_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/common_addresses/validators.rs")
}

fn update_validators_file(entries: &[(Address, u64)]) -> Result<()> {
    let path = validators_file();
    let content = std::fs::read_to_string(&path)?;
    let marker = "pub const FEE_RECIPIENT_LIST: &[FeeRecipient] = &[";
    let start = content
        .find(marker)
        .ok_or_else(|| eyre::eyre!("Could not locate FeeRecipient list"))?;
    let insert_pos = content[start..]
        .find("];")
        .map(|idx| start + idx)
        .ok_or_else(|| eyre::eyre!("Malformed validators file"))?;

    let mut updated = String::with_capacity(content.len() + entries.len() * 160);
    updated.push_str(&content[..insert_pos]);
    updated.push('\n');

    for (addr, count) in entries {
        let hex_addr = hex::encode_upper(addr);
        let name = default_builder_name(addr);
        updated.push_str(&format!(
            "    FeeRecipient {{\n        address: address!(\"{}\"),\n        name: \"{}\",\n    }}, // discovered in {} blocks\n",
            hex_addr, name, count
        ));
    }

    updated.push_str(&content[insert_pos..]);
    std::fs::write(&path, updated)?;
    Ok(())
}

fn run_cargo_fmt() -> Result<()> {
    let status = Command::new("cargo")
        .arg("fmt")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        bail!("cargo fmt failed")
    }
}
