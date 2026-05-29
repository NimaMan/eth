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

        let tx = provider.process_transaction_by_hash(hash).await?;
        let fees = &tx.fees;

        let priority_gwei = fees
            .max_priority_fee
            .map(|f| f.to::<u128>() as f64 / 1e9)
            .unwrap_or(0.0);
        let bribe_wei = tx.bribe_amount.to::<u128>();
        let bribe_eth = bribe_wei as f64 / 1e18;

        println!("tx:               {}", hash_str);
        println!("  protocol_type:  {}", fees.protocol_type);
        println!("  gas_price:      {} gwei", fees.gas_price.to::<u128>() as f64 / 1e9);
        println!("  max_priority:   {} gwei", priority_gwei);
        println!("  max_fee:        {} gwei", fees.max_fee_per_gas.map(|f| f.to::<u128>() as f64 / 1e9).unwrap_or(0.0));
        println!("  gas_used:       {}", fees.gas_used);
        println!("  bribe_amount:   {} wei  ({:.8} ETH)", bribe_wei, bribe_eth);
        println!("  expected:       {} * {} = {} wei", fees.max_priority_fee.unwrap_or_default(), fees.gas_used, priority_gwei * fees.gas_used as f64 * 1e9);
        println!();
    }

    Ok(())
}
