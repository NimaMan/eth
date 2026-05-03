//! Replay a contract-creation tx using the same simulation path as the live pipeline:
//! 1) Load the tx from the local Reth DB
//! 2) Rebuild the unsigned tx (keeps original gas caps)
//! 3) Simulate at block-1 (pre-state) via ProcessedTxProvider
//! 4) Resolve contract address (processed/event/derived) and fetch basic metadata
//!
//! Usage:
//!   cargo run -p mempool_processor --example replay_contract_creation \
//!     -- --tx 0xdd784414edbdae33fe7d5c0b1a2d4e0319592b817372325408f5c3f3b2fd84b2 \
//!     --datadir ~/.local/share/reth/mainnet

use alloy_primitives::{keccak256, Address, B256};
use clap::Parser;
use eyre::Result;
use mempool_processor::token_tracking::token_parameter_extraction::fetch_token_metadata;
use reth_chain_query::{provider::RethQueryProvider, to_checksum_address};
use rlp::RlpStream;
use std::str::FromStr;
use std::sync::Arc;
use tx_processor::processed_tx_provider::ProcessedTxProvider;
use tx_processor::UnsignedTxBuilder;
use tx_simulator::TxSimulator;

#[derive(Parser, Debug)]
struct Args {
    /// Reth datadir (contains db/static_files)
    #[arg(long, default_value = "~/.local/share/reth/mainnet")]
    datadir: String,
    /// Transaction hash of the contract creation
    #[arg(long, value_name = "TX_HASH")]
    tx: String,
}

fn expand_tilde(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, stripped);
        }
    }
    path.to_string()
}

fn derive_contract_address(sender: Address, nonce: u64) -> Address {
    let mut stream = RlpStream::new_list(2);
    stream.append(&sender.as_slice());
    stream.append(&nonce);
    let hash = keccak256(stream.out());
    Address::from_slice(&hash[12..])
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let tx_hash = B256::from_str(&args.tx)?;
    let datadir = expand_tilde(&args.datadir);

    println!("Datadir: {}", datadir);
    println!("Tx hash: {:?}", tx_hash);

    // Core providers
    let simulator = Arc::new(TxSimulator::new(&datadir)?);
    let processed_provider = Arc::new(ProcessedTxProvider::with_provider_factory(
        simulator.provider_factory().clone(),
    )?);

    // Fetch processed transaction from DB + partial replay
    let processed = processed_provider
        .process_transaction_by_hash(tx_hash)
        .await?;
    // Rebuild unsigned tx as the pipeline would
    let unsigned = UnsignedTxBuilder::build_unsigned_from_processed_tx(&processed);
    let sim_block = processed.block_number.saturating_sub(1);
    let replayed = processed_provider
        .process_transaction_from_unsigned_tx(unsigned, Some(sim_block))
        .await?;

    // Resolve contract address
    let contract_addr = replayed
        .contract_address
        .or_else(|| {
            replayed
                .contract_creation_events
                .first()
                .map(|e| e.contract_address)
        })
        .unwrap_or_else(|| derive_contract_address(replayed.from_address, replayed.nonce));

    println!("Sim block:   {} (pre-state)", sim_block);
    println!(
        "From:        {}",
        to_checksum_address(&replayed.from_address)
    );
    println!("Nonce:       {}", replayed.nonce);
    println!(
        "To:          {:?}",
        replayed.to_address.map(|a| to_checksum_address(&a))
    );
    println!("Contract:    {}", to_checksum_address(&contract_addr));
    println!(
        "Events:      {:?}",
        replayed
            .contract_creation_events
            .iter()
            .map(|e| to_checksum_address(&e.contract_address))
            .collect::<Vec<_>>()
    );

    // Try to pull metadata for the deployed contract (same helper as pipeline)
    let rq = RethQueryProvider::with_simulator(simulator.clone())?;
    match fetch_token_metadata(&rq, contract_addr, None).await {
        Ok(Some(meta)) => println!(
            "Metadata: symbol={} name={} decimals={}",
            meta.symbol, meta.name, meta.decimals
        ),
        Ok(None) => println!("Metadata: non-ERC20 bytecode or no metadata available"),
        Err(err) => println!("Metadata lookup failed: {}", err),
    }
    // Avoid dropping internal blocking runtimes inside async context
    std::mem::forget(rq);
    std::mem::forget(processed_provider);
    std::mem::forget(simulator);

    Ok(())
}
