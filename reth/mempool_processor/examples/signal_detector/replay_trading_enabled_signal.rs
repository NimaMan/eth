use alloy_primitives::{Address as AlloyAddress, B256, U256};
use clap::Parser;
use eyre::{eyre, Context, Result};
use mempool_processor::function_detector::CreatorFunctionType;
use mempool_processor::mempool_fetcher::MempoolTransaction;
use mempool_processor::signal_detector::signal_manager::{SignalManager, SignalManagerConfig};
use mempool_processor::signal_detector::types::Signal;
use mempool_processor::simulator::pool_buy_sell_simulator::PoolBuySellSimulator;
use mempool_processor::simulator::simulation_manager::{
    SimulationResult, SimulationType, TxSimulationJob,
};
use mempool_processor::token_tracking::token_parameter_extraction::{
    fetch_token_decimals, fetch_token_metadata,
};
use mempool_processor::token_tracking::TokenTrackingCache;
use mempool_processor::tx_router::{SimulationPriority, TransactionCategory};
use reth_chain_query::provider::{RethQueryProvider, TransactionData};
use reth_chain_query::to_checksum_address;
use serde_json::json;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Builder;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use tx_processor::simulator::types::{
    PoolBuySellParameters, PoolBuySellSimulationResult, PoolType,
};
use tx_processor::tx_processor::TxProcessor;
use tx_processor::ProcessedTransaction;
use tx_simulator::{TxSimulator, UnsignedTransaction};

/// Replay the trading-enabled setup from `replay_trading_status`, then feed the
/// resulting pool probe through the signal manager to emit a
/// `TradingEnabled` signal.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long, default_value = "/home/nima/.local/share/reth/mainnet")]
    datadir: PathBuf,

    #[arg(long)]
    reth_index: Option<PathBuf>,

    #[arg(long, default_value_t = 23647431)]
    block: u64,

    #[arg(long)]
    discovery_block: Option<u64>,

    #[arg(long)]
    token: String,

    #[arg(long)]
    pool: String,

    #[arg(long)]
    creator: Option<String>,

    #[arg(long, value_delimiter = ',')]
    hashes: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run(args))
}

async fn run(args: Args) -> Result<()> {
    info!(
        "🔄 Initialising TxSimulator from {}",
        args.datadir.display()
    );
    let tx_simulator = Arc::new(
        TxSimulator::new(args.datadir.to_str().unwrap()).context("failed to open Reth database")?,
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

    info!("📘 Loading canonical header for block {}", args.block);

    let mut chain = tx_simulator
        .start_simulation_chain(Some(args.block))
        .await
        .context("failed to initialize sequential simulator")?;
    let tx_processor = TxProcessor::new();

    let (prior_txs, tx_datas) = load_helper_transactions(
        provider.clone(),
        &mut chain,
        &tx_processor,
        args.block,
        &args,
    )
    .await?;

    if prior_txs.is_empty() {
        return Err(eyre!("no helper transactions available to replay"));
    }

    let token_address = AlloyAddress::from_str(&args.token)
        .with_context(|| format!("invalid token address {}", args.token))?;
    let pool_address = AlloyAddress::from_str(&args.pool)
        .with_context(|| format!("invalid pool address {}", args.pool))?;
    let weth_address = AlloyAddress::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
        .context("invalid WETH address literal")?;

    let decimals = fetch_token_decimals_or_metadata(&provider, token_address, args.block).await?;

    let mut params = PoolBuySellParameters::new(token_address, pool_address, PoolType::UniswapV2);
    params.prior_txs = prior_txs;
    params.block_number = Some(args.block);
    params.token_decimals = decimals;
    params.test_amount = U256::from(10_000_000_000_000_000u64); // 0.01 ETH probe
    params.weth_address = weth_address;
    params.denom_address = weth_address;
    params.denom_decimals = 18;

    info!("🚦 Running buy/approve/sell viability probe");
    let pool_result = pool_simulator
        .simulate_pool_with_config(params)
        .await
        .context("pool buy/sell simulation failed")?;

    log_pool_result(&pool_result);

    let last_tx = tx_datas
        .last()
        .cloned()
        .ok_or_else(|| eyre!("missing transaction data for trading signal"))?;

    let job = build_simulation_job(&last_tx, token_address)?;

    let simulation_result = SimulationResult {
        request: job,
        error: None,
        simulation_time_ms: 42.0,
        token_address: Some(token_address),
        pool_address: Some(pool_address),
        pool_type: Some(format!("{:?}", pool_result.pool_type)),
        debug_info: None,
        pool_viability_result: Some(pool_result.clone()),
        liquidity_removal_result: None,
    };

    let mut manager = SignalManager::new(SignalManagerConfig::default());
    manager.set_token_cache(Arc::new(TokenTrackingCache::with_defaults()));

    let signals = manager.process_simulation_result(&simulation_result).await;

    println!("\nSignals emitted:");
    if signals.is_empty() {
        println!("  (none)");
    } else {
        for signal in &signals {
            match signal {
                Signal::TradingEnabled(s) => {
                    println!(
                        "  - TradingEnabled: token={} pool={} buy_tax={:.1}% sell_tax={:.1}%",
                        s.token_address, s.pool_address, s.buy_tax, s.sell_tax
                    );
                }
                other => println!("  - {:?}", other),
            }
        }
    }

    Ok(())
}

async fn load_helper_transactions(
    provider: Arc<RethQueryProvider>,
    chain: &mut tx_simulator::UnsignedTxChainSimulation,
    tx_processor: &TxProcessor,
    block_number: u64,
    args: &Args,
) -> Result<(Vec<ProcessedTransaction>, Vec<TransactionData>)> {
    let mut hash_inputs = args.hashes.clone();
    let mut prior_txs = Vec::new();
    let mut tx_datas = Vec::new();

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
                block_number,
                prior_txs.len() as u64,
            )
            .await
            .with_context(|| format!("failed to process {}", hash_hex))?;

        log_processed_transaction(&tx_data, &processed);
        prior_txs.push(processed);
        tx_datas.push(tx_data);
    }

    Ok((prior_txs, tx_datas))
}

fn build_simulation_job(
    tx: &TransactionData,
    token_address: AlloyAddress,
) -> Result<TxSimulationJob> {
    let hash_hex = format!("{:#x}", tx.hash);
    let mempool_tx = MempoolTransaction {
        hash: hash_hex.clone(),
        data: json!({}),
        detection_ns: 0,
        detection_time: Instant::now(),
        latency_ns: 0,
        from: tx.from.as_slice().to_vec(),
        to: tx.to.map(|addr| addr.as_slice().to_vec()),
        input: tx.input.to_vec(),
        value: tx.value,
        gas_price: Some(tx.gas_price),
        functions: vec!["trading_control".to_string()],
        function_category: Some(CreatorFunctionType::TradingControl),
    };

    let tx_hash = tx.hash;

    Ok(TxSimulationJob {
        tx: mempool_tx,
        category: TransactionCategory::CreatorTransaction {
            creator: to_checksum_address(&tx.from),
            target_address: tx
                .to
                .map(|addr| to_checksum_address(&addr))
                .unwrap_or_else(|| "creation".to_string()),
            target_token: Some(to_checksum_address(&token_address)),
            function_type: CreatorFunctionType::TradingControl,
        },
        priority: SimulationPriority::Critical,
        simulation_type: SimulationType::TransactionWithBuySell,
        tx_hash,
    })
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
        nonce: Some(tx.nonce),
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
}

async fn fetch_token_decimals_or_metadata(
    provider: &RethQueryProvider,
    token_address: AlloyAddress,
    block: u64,
) -> Result<u8> {
    let mut errors = Vec::new();

    for context in [Some(block), None] {
        let label = context
            .map(|b| format!("block {}", b))
            .unwrap_or_else(|| "latest".to_string());

        match fetch_token_metadata(provider, token_address, context).await {
            Ok(Some(meta)) => {
                info!(
                    "ℹ️ Token metadata ({label}): symbol={} decimals={}",
                    meta.symbol, meta.decimals
                );
                return Ok(meta.decimals);
            }
            Ok(None) => {
                warn!("Token metadata lookup ({label}) indicates non-ERC20 bytecode");
                errors.push(format!("metadata {label}: non-erc20"));
            }
            Err(err) => {
                warn!("Token metadata lookup ({label}) failed: {err}");
                errors.push(format!("metadata {label}: {err}"));
            }
        }

        match fetch_token_decimals(provider, token_address, context).await {
            Ok(dec) => {
                info!("ℹ️ decimals() probe ({label}) succeeded: {}", dec);
                return Ok(dec);
            }
            Err(err) => {
                warn!("Token decimals probe ({label}) failed: {err}");
                errors.push(format!("decimals {label}: {err}"));
            }
        }
    }

    Err(eyre!(
        "failed to determine token decimals for {}: {}",
        token_address,
        errors.join("; ")
    ))
}

async fn collect_creator_transactions(
    provider: Arc<RethQueryProvider>,
    creator: &str,
    upto_block: u64,
) -> Result<Vec<B256>> {
    let creator_address = AlloyAddress::from_str(creator)
        .with_context(|| format!("invalid creator address {}", creator))?;
    let tx_numbers = provider
        .get_address_transactions(creator_address)
        .context("failed to fetch creator transaction ids (ensure reth_index is present)")?;

    let mut relevant: Vec<(u64, B256)> = Vec::new();
    for tx_number in tx_numbers {
        let tx = provider
            .get_transaction_by_number(tx_number)
            .await
            .with_context(|| format!("failed to load transaction number {}", tx_number))?;
        if tx.block_number <= upto_block && tx.from == creator_address {
            relevant.push((tx.nonce, tx.hash));
        }
    }

    relevant.sort_by_key(|(nonce, _)| *nonce);
    relevant.into_iter().map(|(_, hash)| Ok(hash)).collect()
}

fn resolve_reth_index_path(args: &Args) -> Option<PathBuf> {
    if let Some(explicit) = &args.reth_index {
        return Some(explicit.clone());
    }
    let default_path = args.datadir.join("reth_index");
    if default_path.exists() {
        Some(default_path)
    } else {
        None
    }
}
