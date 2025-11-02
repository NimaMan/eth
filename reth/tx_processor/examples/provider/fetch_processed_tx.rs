use alloy_primitives::B256;
use eyre::Result;
use std::str::FromStr;
use tx_processor::ProcessedTxProvider;

fn parse_args() -> Result<B256> {
    let Some(arg) = std::env::args().nth(1) else {
        eyre::bail!(
            "usage: cargo run --example fetch_processed_tx --release -- <transaction_hash>"
        );
    };
    Ok(B256::from_str(arg.trim_start_matches("0x"))?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let tx_hash = parse_args()?;

    let datadir = std::env::var("RETH_DATADIR")?;
    let provider = ProcessedTxProvider::new(&datadir)?;

    let processed = provider.process_transaction_by_hash(tx_hash).await?;

    println!("hash:   0x{}", hex::encode(processed.hash));
    println!("block:  {}", processed.block_number);
    println!("index:  {}", processed.tx_index);
    println!("status: {}", processed.status);
    println!("nonce:  {}", processed.nonce);
    println!("from:   0x{}", hex::encode(processed.from_address));
    if let Some(to) = processed.to_address {
        println!("to:     0x{}", hex::encode(to));
    } else {
        println!("to:     <contract creation>");
    }
    println!("access_list entries: {}", processed.access_list.len());
    println!(
        "internal transactions: {}",
        processed.internal_transactions.len()
    );
    for (idx, internal) in processed.internal_transactions.iter().enumerate() {
        println!(
            "  #{idx:02}: depth={} type={} from=0x{} to=0x{} value={} gas={} gas_used={:?}",
            internal.depth,
            internal.trace_type,
            hex::encode(internal.from_address),
            internal
                .to_address
                .map(|addr| hex::encode(addr))
                .unwrap_or_else(|| "<none>".into()),
            internal.value,
            internal.gas,
            internal.gas_used
        );
    }

    Ok(())
}
