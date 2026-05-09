//! Replay creator helper transactions and run a buy/sell viability check for a pool.
//!
//! This mirrors the production `SimulationManager` flow: we replay the creator's
//! recent transactions against the local Reth database, then execute a pooled
//! buy/approve/sell probe to verify trading status (including taxes).
//!
//! Usage examples:
//! ```bash
//! # Provide hashes explicitly
//! cargo run --example replay_trading_status \
//!   -- --datadir ~/.local/share/reth/mainnet \
//!      --block 23647431 \
//!      --token 0xE875f15C82436c72568a206e50B0e2D44e7ad637 \
//!      --pool 0x33c34c89735a1ECA866f24d18182cAe84377Ad31 \
//!      --hashes 0xf882b0ac9cff7877268f62000a57ac06015c0ca75170e53135e701bd6db9f5ff,\
//!               0x5c8299f80652f2b4671bfae678352b3ce7e586e91b5b1daf5fb95b1917badb74,\
//!               0xbaeafdb46f300bcda848e1ec300152c46a35dc70e1542d19f36012343f071dc4,\
//!               0xef457a1abb9f8c7b5a5700d9e4e0e66c701feadb1888b9652f1938c5a8164923
//!
//! # Or let the example discover helper transactions via RethIndex
//! cargo run --example replay_trading_status \
//!   -- --datadir ~/.local/share/reth/mainnet \
//!      --block 23647431 \
//!      --token 0xE875f15C82436c72568a206e50B0e2D44e7ad637 \
//!      --pool 0x33c34c89735a1ECA866f24d18182cAe84377Ad31 \
//!      --creator 0x1C454F4A6a8348c34078577B525b523Ca1a95174
//! ```

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use alloy_primitives::{Address as AlloyAddress, B256, U256};
use clap::Parser;
use eyre::{eyre, Context, Result};
use mempool_processor::simulator::pool_buy_sell_simulator::PoolBuySellSimulator;
use mempool_processor::token_tracking::token_parameter_extraction::{
    fetch_token_decimals, fetch_token_metadata,
};
use reth_chain_query::provider::{RethQueryProvider, TransactionData};
use reth_chain_query::to_checksum_address;
use tokio::runtime::Runtime;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use tx_processor::simulator::types::{
    PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_processor::ProcessedTransaction;
use tx_simulator::{TxSimulator, UnsignedTransaction};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the local Reth data directory (containing db/ and static_files/)
    #[arg(long, env = "RETH_DATADIR")]
    datadir: Option<PathBuf>,

    /// Optional path to the Reth index (defaults to <datadir>/reth_index when present)
    #[arg(long)]
    reth_index: Option<PathBuf>,

    /// Canonical block number used as the simulation base
    #[arg(long, default_value_t = 23647431)]
    block: u64,

    /// Block number used to discover creator transactions (defaults to block+1)
    #[arg(long)]
    discovery_block: Option<u64>,

    /// Token address we want to analyse
    #[arg(long)]
    token: String,

    /// Liquidity pool address we want to probe
    #[arg(long)]
    pool: String,

    /// Creator address (required when --hashes is omitted)
    #[arg(long)]
    creator: Option<String>,

    /// Comma separated list of helper TX hashes to replay (skip auto-discovery)
    #[arg(long, value_delimiter = ',')]
    hashes: Vec<String>,
}

fn main() -> Result<()> {
    let mut args = Args::parse();
    if args.datadir.is_none() {
        args.datadir = Some(PathBuf::from(
            mempool_processor::config::reth_datadir_from_env(),
        ));
    }

    let rt = Runtime::new().context("failed to create Tokio runtime")?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    rt.block_on(async move { run_example(args).await })
}

async fn run_example(args: Args) -> Result<()> {
    info!(
        "🔄 Initialising TxSimulator from {}",
        datadir(&args).display()
    );
    let tx_simulator = Arc::new(
        TxSimulator::new(datadir(&args).to_str().unwrap())
            .context("failed to open Reth database")?,
    );

    info!("🔍 Building query provider");
    let provider = Arc::new(match resolve_reth_index_path(&args) {
        Some(index_path) => match RethQueryProvider::with_simulator(tx_simulator.clone())
            .and_then(|provider| provider.with_reth_index(index_path.to_str().unwrap()))
        {
            Ok(with_index) => {
                info!("📚 RethIndex attached from {}", index_path.display());
                with_index
            }
            Err(err) => {
                warn!(
                    "Failed to attach RethIndex at {}: {} (continuing without index)",
                    index_path.display(),
                    err
                );
                RethQueryProvider::with_simulator(tx_simulator.clone())
                    .context("failed to rebuild provider without RethIndex")?
            }
        },
        None => RethQueryProvider::with_simulator(tx_simulator.clone())
            .context("failed to build query provider")?,
    });

    info!("🧪 Initialising pool buy/sell simulator");
    let pool_simulator = PoolBuySellSimulator::with_tx_simulator(tx_simulator.clone())
        .context("failed to initialise PoolBuySellSimulator (is the database accessible?)")?;

    info!(
        "📘 Initialising sequential simulation at block {}",
        args.block
    );
    let mut chain = tx_simulator
        .start_simulation_chain(Some(args.block))
        .await
        .context("failed to initialize sequential simulator")?;
    let tx_processor = TxProcessor::new();

    // Determine the sequence of helper transactions.
    let mut hash_inputs = args.hashes.clone();
    if hash_inputs.is_empty() {
        let creator = args.creator.as_ref().ok_or_else(|| {
            eyre!("either provide --hashes or specify --creator to auto-discover transactions")
        })?;
        let discovery_block = args
            .discovery_block
            .unwrap_or_else(|| args.block.saturating_add(1));
        let derived_hashes =
            collect_creator_transactions(provider.clone(), creator, discovery_block).await?;
        if derived_hashes.is_empty() {
            eyre::bail!(
                "no creator transactions found for {} up to block {}",
                creator,
                discovery_block
            );
        }
        info!(
            "🧾 Derived {} helper transaction(s) for creator {} (<= block {})",
            derived_hashes.len(),
            creator,
            discovery_block
        );
        hash_inputs = derived_hashes
            .into_iter()
            .map(|hash| format!("{:#x}", hash))
            .collect();
    }

    info!("🔁 Replaying {} helper transactions", hash_inputs.len());
    let mut prior_txs = Vec::new();
    for hash_hex in &hash_inputs {
        let hash = parse_hash(hash_hex)?;
        let tx_data = provider
            .get_transaction_by_hash(hash)
            .await
            .with_context(|| format!("failed to load transaction {}", hash_hex))?;

        let unsigned_tx = transaction_to_unsigned(&tx_data)?;
        let full_result = chain
            .step_with_trace(unsigned_tx.clone())
            .await
            .with_context(|| format!("failed to replay {}", hash_hex))?;
        let processed = tx_processor
            .process_transaction_from_simulation_result(
                &unsigned_tx,
                &full_result,
                args.block,
                prior_txs.len() as u64,
            )
            .await
            .with_context(|| format!("failed to process {}", hash_hex))?;

        log_processed_transaction(&tx_data, &processed);
        prior_txs.push(processed);
    }

    // Fetch token decimals so we size trade amounts correctly.
    let token_address = AlloyAddress::from_str(&args.token)
        .with_context(|| format!("invalid token address {}", args.token))?;
    let pool_address = AlloyAddress::from_str(&args.pool)
        .with_context(|| format!("invalid pool address {}", args.pool))?;
    let weth_address = AlloyAddress::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
        .context("invalid WETH address literal")?;

    info!("ℹ️ Fetching token metadata for {}", args.token);
    let decimals = match fetch_token_metadata(&provider, token_address, Some(args.block)).await {
        Ok(Some(meta)) => {
            info!(
                "  token metadata loaded: symbol={} decimals={}",
                meta.symbol, meta.decimals
            );
            Ok(meta.decimals)
        }
        Ok(None) => {
            warn!(
                "  token metadata indicates non-ERC20 contract. Falling back to decimals() probe."
            );
            fetch_token_decimals(&provider, token_address, Some(args.block))
                .await
                .with_context(|| "failed to determine token decimals via fallback decimals() probe")
        }
        Err(meta_err) => {
            warn!(
                "  token metadata lookup failed: {}. Falling back to decimals() probe.",
                meta_err
            );
            fetch_token_decimals(&provider, token_address, Some(args.block))
                .await
                .with_context(|| format!("failed after metadata() error: {meta_err}"))
        }
    }?;

    let mut params = PoolBuySellParameters::new(token_address, pool_address, PoolType::UniswapV2);
    params.prior_txs = prior_txs.clone();
    params.block_number = Some(args.block);
    params.token_decimals = decimals;
    params.test_amount = U256::from(10_000_000_000_000_000u64); // 0.01 ETH probe
    params.weth_address = weth_address;
    params.denom_address = weth_address;
    params.denom_decimals = 18;

    info!("🚦 Running buy/approve/sell viability probe");
    let sim_result = pool_simulator
        .simulate_pool_with_config(params)
        .await
        .context("pool buy/sell simulation failed")?;

    log_pool_result(&sim_result);

    Ok(())
}

fn parse_hash(hash_hex: &str) -> Result<B256> {
    let trimmed = hash_hex.trim();
    B256::from_str(trimmed).with_context(|| format!("invalid hash {}", trimmed))
}

fn transaction_to_unsigned(tx: &TransactionData) -> Result<UnsignedTransaction> {
    let gas_price = tx
        .max_fee_per_gas
        .is_none()
        .then(|| tx.gas_price.try_into().unwrap_or(u128::MAX));

    let max_fee = tx
        .max_fee_per_gas
        .map(|v| v.try_into().unwrap_or(u128::MAX));

    let max_priority = tx
        .max_priority_fee_per_gas
        .map(|v| v.try_into().unwrap_or(u128::MAX));

    Ok(UnsignedTransaction {
        from: Some(tx.from),
        to: tx.to,
        gas: Some(tx.gas_limit),
        gas_price,
        max_fee_per_gas: max_fee,
        max_priority_fee_per_gas: max_priority,
        value: Some(tx.value),
        data: if tx.input.is_empty() {
            None
        } else {
            Some(tx.input.clone())
        },
        nonce: None, // allow the simulator to derive the canonical nonce
        access_list: Vec::new(),
        blob_versioned_hashes: Vec::new(),
        max_fee_per_blob_gas: None,
        signed_authorizations: Vec::new(),
    })
}

fn log_processed_transaction(tx_data: &TransactionData, processed: &ProcessedTransaction) {
    let hash = format!("{:#x}", tx_data.hash);
    let from = to_checksum_address(&tx_data.from);
    let to = tx_data
        .to
        .map(|addr| to_checksum_address(&addr))
        .unwrap_or_else(|| "creation".to_string());

    info!(
        "🧱 Replayed tx {} (nonce {}) from {} -> {} | success={} | gas_used={}",
        hash, tx_data.nonce, from, to, processed.status, processed.fees.gas_used
    );
}

fn log_pool_result(result: &PoolBuySellSimulationResult) {
    info!("📊 Buy/Sell probe summary");
    info!("  can_buy: {}", result.can_buy);
    info!("  can_sell: {}", result.can_sell);
    info!("  is_tradeable: {}", result.is_tradeable);
    info!("  buy_tax: {:.2}%", result.buy_tax_percent);
    info!("  sell_tax: {:.2}%", result.sell_tax_percent);
    info!(
        "  failure_reason: {}",
        result.failure_reason.as_deref().unwrap_or("None")
    );

    if let Some(reason) = &result.failure_reason {
        info!("  buy failure detail: {}", reason);
    }
}

async fn collect_creator_transactions(
    provider: Arc<RethQueryProvider>,
    creator: &str,
    upto_block: u64,
) -> Result<Vec<B256>> {
    let creator_address = AlloyAddress::from_str(creator)
        .with_context(|| format!("invalid creator address {}", creator))?;
    let blocks = provider
        .get_address_participation_blocks(creator_address)
        .context("failed to fetch creator participation blocks (ensure reth_index is present)")?;

    let mut relevant: Vec<(u64, B256)> = Vec::new();
    for block_number in blocks.into_iter().filter(|block| *block <= upto_block) {
        let block = provider
            .get_block_transactions(block_number)
            .await
            .with_context(|| format!("failed to load block {}", block_number))?;
        for tx in block.transactions {
            let tx = tx.tx_metadata;
            if tx.from == creator_address {
                relevant.push((tx.nonce, tx.hash));
            }
        }
    }

    relevant.sort_by_key(|(nonce, _)| *nonce);
    relevant.into_iter().map(|(_, hash)| Ok(hash)).collect()
}

fn resolve_reth_index_path(args: &Args) -> Option<PathBuf> {
    if let Some(explicit) = &args.reth_index {
        return Some(explicit.clone());
    }
    let default_path = datadir(args).join("reth_index");
    if default_path.exists() {
        Some(default_path)
    } else {
        None
    }
}

fn datadir(args: &Args) -> &PathBuf {
    args.datadir
        .as_ref()
        .expect("datadir is initialized after Args::parse")
}
