use alloy_primitives::{Address, B256, U256};
use clap::Parser;
use eyre::{eyre, Result};
use reth_chain_query::RethQueryProvider;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tx_processor::{
    ProcessedTransaction, ProcessedTxProvider, TxSimulator, UnsignedTransaction, UnsignedTxBuilder,
};

#[derive(Debug, Parser)]
#[command(
    name = "token_metadata_from_processed_tx",
    about = "Replay processed contract creation tx and fetch ERC-20 metadata"
)]
struct Args {
    /// Transaction hash (contract creation) that should be replayed before querying metadata.
    #[arg(long, value_name = "TX_HASH")]
    tx_hash: String,

    /// Contract address to query. Defaults to the transaction's contract_address/to.
    #[arg(long, value_name = "ADDRESS")]
    contract: Option<String>,

    /// Block number to run the metadata call against (defaults to tx_block - 1).
    #[arg(long)]
    metadata_block: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let datadir = resolve_datadir()?;
    let datadir_str = datadir
        .to_str()
        .ok_or_else(|| eyre!("Invalid datadir path: {}", datadir.display()))?
        .to_string();

    let simulator = Arc::new(TxSimulator::new(&datadir_str)?);
    let processed_provider = Arc::new(ProcessedTxProvider::with_simulator(simulator.clone())?);
    let query_provider = Arc::new(RethQueryProvider::with_simulator(simulator.clone())?);

    let rt = tokio::runtime::Runtime::new()?;
    let outcome = rt.block_on(async_main(
        args,
        processed_provider.clone(),
        query_provider.clone(),
    ));
    drop(rt);
    drop(processed_provider);
    drop(query_provider);
    drop(simulator);
    outcome
}

async fn async_main(
    args: Args,
    processed_provider: Arc<ProcessedTxProvider>,
    query_provider: Arc<RethQueryProvider>,
) -> Result<()> {
    let Args {
        tx_hash,
        contract,
        metadata_block,
    } = args;

    let tx_hash = B256::from_str(&tx_hash)?;
    let processed_tx = processed_provider
        .load_transaction_from_hash_db_only(tx_hash)
        .await?;

    let contract_address = resolve_contract_address(contract.as_deref(), &processed_tx)?;
    let metadata_block = resolve_metadata_block(metadata_block, processed_tx.block_number)?;

    println!(
        "📦 Loaded processed tx {tx_hash:?} (block #{})",
        processed_tx.block_number
    );
    println!("   Contract address: {contract_address:?}");
    println!("   Metadata block  : {}", metadata_block);

    let mut pending_tx = UnsignedTxBuilder::build_unsigned_from_processed_tx(&processed_tx);
    if pending_tx.nonce.is_none() {
        pending_tx.nonce = Some(processed_tx.nonce);
    }
    println!(
        "Pending fee fields: gas_price={:?}, max_fee_per_gas={:?}, max_priority_fee={:?}",
        pending_tx.gas_price, pending_tx.max_fee_per_gas, pending_tx.max_priority_fee_per_gas
    );

    print_fee_diagnostics(&query_provider, &processed_tx, metadata_block).await?;

    let pending: Vec<UnsignedTransaction> = vec![pending_tx];
    println!(
        "▶️  Replaying {} pending transaction(s) before metadata call",
        pending.len()
    );

    let metadata = query_provider
        .get_token_metadata(contract_address, Some(metadata_block), &pending)
        .await?;

    match metadata {
        Some(token) => {
            println!("\n✅ Token metadata fetched successfully:");
            println!("   Name     : {}", token.name);
            println!("   Symbol   : {}", token.symbol);
            println!("   Decimals : {}", token.decimals);
            println!("   Supply   : {}", token.total_supply);
        }
        None => {
            println!("\n⚠️  No metadata returned; contract may not follow ERC-20 conventions.");
        }
    }

    Ok(())
}

fn resolve_datadir() -> Result<PathBuf> {
    if let Ok(from_env) = std::env::var("RETH_DATADIR") {
        return Ok(PathBuf::from(from_env));
    }
    Ok(PathBuf::from("/home/nima/.local/share/reth/mainnet"))
}

fn resolve_contract_address(cli_value: Option<&str>, tx: &ProcessedTransaction) -> Result<Address> {
    if let Some(addr) = cli_value {
        return Address::from_str(addr).map_err(|err| eyre!("Invalid --contract: {err}"));
    }
    if let Some(contract) = tx.contract_address {
        return Ok(contract);
    }
    tx.to_address
        .ok_or_else(|| eyre!("Transaction is not a contract creation; pass --contract explicitly"))
}

fn resolve_metadata_block(cli_value: Option<u64>, tx_block: u64) -> Result<u64> {
    if let Some(block) = cli_value {
        return Ok(block);
    }
    if tx_block == 0 {
        return Err(eyre!("Cannot infer metadata block when tx is in block 0"));
    }
    Ok(tx_block - 1)
}

async fn print_fee_diagnostics(
    provider: &RethQueryProvider,
    processed_tx: &ProcessedTransaction,
    metadata_block: u64,
) -> Result<()> {
    let from = processed_tx.from_address;
    let gas_limit = processed_tx.fees.gas_limit;
    let price_cap = processed_tx
        .fees
        .max_fee_per_gas
        .unwrap_or(processed_tx.fees.gas_price);
    let required_fee = price_cap * U256::from(gas_limit);

    println!("\n💰 Fee diagnostics:");
    println!("   Gas limit      : {} (0x{:x})", gas_limit, gas_limit);
    println!(
        "   Price cap      : {} wei (~{} Gwei)",
        price_cap,
        format_gwei(&price_cap)
    );
    println!(
        "   Required fee   : {} wei ({})",
        required_fee,
        format_eth(&required_fee)
    );

    let before = provider.get_account(from, Some(metadata_block)).await?;
    println!(
        "   Balance @{} : {} wei ({})",
        metadata_block,
        before.balance,
        format_eth(&before.balance)
    );
    if processed_tx.block_number >= metadata_block {
        let after = provider
            .get_account(from, Some(processed_tx.block_number))
            .await?;
        println!(
            "   Balance @{} : {} wei ({})",
            processed_tx.block_number,
            after.balance,
            format_eth(&after.balance)
        );
    }

    let deficit = before.balance.checked_sub(required_fee);
    match deficit {
        Some(remaining) => println!(
            "   Balance - fee  : {} wei ({})",
            remaining,
            format_eth(&remaining)
        ),
        None => println!(
            "   Balance - fee  : deficit of {} wei",
            required_fee.saturating_sub(before.balance)
        ),
    }

    Ok(())
}

fn format_eth(value: &U256) -> String {
    if let Ok(as_u128) = u128::try_from(*value) {
        format!("{:.6} ETH", (as_u128 as f64) / 1e18)
    } else {
        "~? ETH".to_string()
    }
}

fn format_gwei(value: &U256) -> String {
    if let Ok(as_u128) = u128::try_from(*value) {
        format!("{:.3}", (as_u128 as f64) / 1e9)
    } else {
        "~?".to_string()
    }
}
