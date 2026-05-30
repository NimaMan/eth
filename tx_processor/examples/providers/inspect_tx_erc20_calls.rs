//! Process a transaction fresh from the reth DB (NOT the disk cache) and print
//! the decoded standard-ERC-20 calls captured from its trace, plus a check for
//! which ones emitted NO matching `Transfer` event (the custody-drain signature).
//!
//! Validates the internal-ERC-20-call capture against the Session drain:
//!   cargo run -p tx_processor --example inspect_tx_erc20_calls -- \
//!     0xf0e8542a57c488121f18bdad4f148799e59ee21b83caa12bdc5b9aadac43d8ba

use alloy_primitives::B256;
use std::str::FromStr;
use tx_processor::processed_tx_provider::ProcessedTxProvider;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <tx_hash> [tx_hash ...]", args[0]);
        std::process::exit(1);
    }

    let reth_datadir = tx_simulator::config::repo::reth_datadir()?;
    let provider = ProcessedTxProvider::new(&reth_datadir)?;

    for hash_str in &args[1..] {
        let hash = B256::from_str(hash_str)
            .map_err(|e| eyre::eyre!("invalid tx hash {hash_str}: {e}"))?;
        // process_transaction_by_hash loads from the reth DB and simulates — no
        // disk-cache involvement.
        let tx = provider.process_transaction_by_hash(hash).await?;

        println!("tx: {hash_str}");
        println!("  block: {}  status: {}", tx.block_number, tx.status);
        println!(
            "  erc20_transfers (Transfer events): {}",
            tx.erc20_transfers.len()
        );
        for t in &tx.erc20_transfers {
            println!(
                "    Transfer token={} from={} to={} amount={}",
                t.token_address, t.from_address, t.to_address, t.amount
            );
        }
        println!(
            "  internal_erc20_calls (decoded from trace): {}",
            tx.internal_erc20_calls.len()
        );
        for call in &tx.internal_erc20_calls {
            // A move with no matching Transfer event is the custody-drain tell.
            let has_matching_transfer = tx.erc20_transfers.iter().any(|t| {
                t.token_address == call.token_address
                    && t.from_address == call.from_address
                    && t.to_address == call.to_address
                    && t.amount == call.amount
            });
            let flag = match call.kind {
                tx_processor::Erc20CallKind::Approve => "",
                _ if has_matching_transfer => "  (matched Transfer)",
                _ => "  <- NO matching Transfer event (event-less move)",
            };
            println!(
                "    {} token={} caller={} from={} to={} amount={} depth={} ok={}{}",
                call.kind.as_str(),
                call.token_address,
                call.caller,
                call.from_address,
                call.to_address,
                call.amount,
                call.depth,
                call.succeeded,
                flag,
            );
        }
        println!();
    }

    Ok(())
}
