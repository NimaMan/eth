use chrono::Utc;
use eyre::Result;
use mempool_processor::config::{
    DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL, DEFAULT_REDIS_TOKEN_PREFIX,
    DEFAULT_TOKEN_CACHE_PUB_ENDPOINT, DEFAULT_TOKEN_CACHE_REP_ENDPOINT,
};
use mempool_processor::function_detector::FunctionDetector;
/// Transaction Router with Token Cache Example
///
/// This example demonstrates the complete pipeline:
/// 1. Connects to Python publisher to get token/pool data
/// 2. Builds token cache from published data
/// 3. Processes 1000 transactions through IPC → Function Detector → TX Router
/// 4. Uses token cache to properly classify creator transactions
/// 5. Logs detailed results for verification
///
/// Algorithm:
/// - Initialize token tracking subscriber to receive pool/creator data from Python
/// - Wait for initial cache population (2 seconds)
/// - Process transactions in batches through function detector
/// - Use TransactionRouter with populated token cache for accurate classification
/// - Track statistics: creator txs, dex interactions, contract creations
/// - Log routing decisions with token cache hits/misses
use mempool_processor::mempool_fetcher::MempoolFetcherIPCClient;
use mempool_processor::token_tracking::TokenTrackingSubscriber;
use mempool_processor::tx_router::{SimulationPriority, TransactionCategory, TransactionRouter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting Transaction Router with Token Cache Example");

    // Create log directory and file
    let log_dir = PathBuf::from(mempool_processor::config::DEFAULT_LOG_DIR).join("tx_router");
    fs::create_dir_all(&log_dir)?;

    let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let log_path = log_dir.join(format!("tx_router_token_cache_1k_{}.log", timestamp));
    let mut log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&log_path)?;

    writeln!(
        log_file,
        "Transaction Router with Token Cache Log - {}",
        timestamp
    )?;
    writeln!(log_file, "=========================================")?;
    info!("📁 Logging to: {}", log_path.display());

    // Initialize token tracker to get cache from Python publisher
    info!("📊 Initializing token tracker to receive pool/creator data...");
    let mut token_tracker = build_token_subscriber(0.1); // 0.1 ETH threshold
    let token_cache = token_tracker.get_cache();

    // Start listening for token updates in background
    tokio::spawn(async move {
        if let Err(e) = token_tracker.start_listening().await {
            warn!("Token tracker error: {}", e);
        }
    });

    // Give it time to load initial data from Python publisher
    info!("⏳ Waiting for initial token cache population...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let initial_pools = token_cache.get_pool_count().await;
    let initial_creators = token_cache.get_creator_count().await;

    // Get some sample entries to verify cache is populated
    let sample_pools = token_cache.pools_snapshot().await; // Snapshot of cached pools
    let sample_creators = token_cache.creator_addresses().await;

    // Count unique tokens from pools
    let initial_tokens: usize = sample_pools
        .iter()
        .map(|(_, pool)| &pool.token_address)
        .collect::<std::collections::HashSet<_>>()
        .len();

    info!("✅ Token cache initialized:");
    info!("   📊 Pools: {}", initial_pools);
    info!("   👤 Creators: {}", initial_creators);
    info!("   🪙 Unique Tokens: {}", initial_tokens);

    writeln!(log_file, "\nInitial Cache State:")?;
    writeln!(log_file, "===================")?;
    writeln!(log_file, "  Total Pools: {}", initial_pools)?;
    writeln!(log_file, "  Total Creators: {}", initial_creators)?;
    writeln!(log_file, "  Unique Tokens: {}", initial_tokens)?;

    // Log first 5 pools as examples
    if !sample_pools.is_empty() {
        writeln!(log_file, "\n  Sample Pools (first 5):")?;
        for (i, (pool_addr, pool_info)) in sample_pools.iter().take(5).enumerate() {
            writeln!(log_file, "    {}. Pool: {}", i + 1, pool_addr)?;
            writeln!(log_file, "       Token: {}", pool_info.token_address)?;
            writeln!(
                log_file,
                "       ETH Reserve: {:.4} ETH",
                pool_info.eth_reserve
            )?;
            writeln!(
                log_file,
                "       Token Reserve: {:.0}",
                pool_info.token_reserve
            )?;
            writeln!(
                log_file,
                "       Last Updated Block: {}",
                pool_info.last_updated_block
            )?;
        }
    }

    // Log first 5 creators as examples
    if !sample_creators.is_empty() {
        writeln!(log_file, "\n  Sample Creators (first 5):")?;
        for (i, creator_addr) in sample_creators.iter().take(5).enumerate() {
            writeln!(log_file, "    {}. Creator: {}", i + 1, creator_addr)?;
        }
    }
    writeln!(log_file, "===================")?;

    // Initialize IPC client
    info!("🔌 Connecting to Reth IPC...");
    let ipc_client =
        MempoolFetcherIPCClient::new(Some("/home/nima/.local/share/reth/mainnet/reth.ipc"))?;
    ipc_client.start().await?;
    info!("✅ Connected to IPC");

    // Initialize function detector with token cache
    let function_detector = FunctionDetector::new_with_cache(Some(token_cache.clone()));

    // Initialize transaction router with token cache
    let tx_router = TransactionRouter::new(Some(token_cache.clone()));
    info!("✅ Router initialized with token cache");

    // Process 1000 transactions
    let target_count = 1000u64;
    let mut processed = 0u64;
    let start_time = Instant::now();

    // Statistics
    let mut contract_creations = 0u64;
    let mut creator_actions = 0u64;
    let mut dex_interactions = 0u64;
    let mut regular_txs = 0u64;
    let mut high_priority = 0u64;
    let mut requires_simulation = 0u64;

    // Token cache hit tracking
    let mut creator_cache_hits = 0u64;
    let mut pool_cache_hits = 0u64;

    info!("📊 Processing {} transactions...", target_count);
    writeln!(log_file, "\n\nTransaction Processing Started")?;
    writeln!(log_file, "=============================")?;

    while processed < target_count {
        let new_txs = ipc_client.get_transactions(50).await?;

        if new_txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }

        // Detect functions
        let transactions_with_functions = function_detector.detect_batch(new_txs);

        // Route each transaction
        for tx in transactions_with_functions {
            let routing_result = tx_router.classify(&tx).await;

            // Log transaction details
            writeln!(log_file, "\n[TX #{}] {}", processed + 1, tx.hash)?;
            writeln!(log_file, "  From: 0x{}", hex::encode(&tx.from))?;
            writeln!(
                log_file,
                "  To: {}",
                tx.to
                    .as_ref()
                    .map(|addr| format!("0x{}", hex::encode(addr)))
                    .unwrap_or_else(|| "CONTRACT_CREATION".to_string())
            )?;

            if !tx.functions.is_empty() {
                writeln!(log_file, "  Functions: {:?}", tx.functions)?;
            }

            // Log routing result and check cache usage
            match &routing_result.category {
                TransactionCategory::ContractCreation {
                    deployer,
                    contract_address,
                    is_token,
                    has_liquidity_in_calldata,
                } => {
                    contract_creations += 1;
                    writeln!(log_file, "  Category: CONTRACT_CREATION")?;
                    writeln!(log_file, "    Is Token: {}", is_token)?;
                    writeln!(log_file, "    Has Liquidity: {}", has_liquidity_in_calldata)?;
                }
                TransactionCategory::CreatorTransaction {
                    creator,
                    target_address,
                    target_token,
                    function_type,
                } => {
                    creator_actions += 1;
                    writeln!(log_file, "  Category: CREATOR_TRANSACTION")?;
                    writeln!(log_file, "    Creator: {}", creator)?;
                    writeln!(log_file, "    Target Token: {:?}", target_token)?;
                    writeln!(log_file, "    Function: {:?}", function_type)?;

                    // Check if this was a cache hit
                    if token_cache.is_creator(creator).await {
                        creator_cache_hits += 1;
                        writeln!(log_file, "    Cache: HIT (creator known)")?;
                    } else {
                        writeln!(log_file, "    Cache: MISS (creator unknown)")?;
                    }
                }
                // DexInteraction category no longer exists - handle as regular transaction
                TransactionCategory::Regular {
                    is_transfer,
                    is_approval,
                } => {
                    regular_txs += 1;
                    if processed < 10 || *is_transfer || *is_approval {
                        // Log first 10 or interesting ones
                        writeln!(log_file, "  Category: REGULAR")?;
                        writeln!(
                            log_file,
                            "    Transfer: {}, Approval: {}",
                            is_transfer, is_approval
                        )?;
                    }
                }
            }

            // Track priority
            match routing_result.priority {
                SimulationPriority::High => high_priority += 1,
                _ => {}
            }

            if routing_result.requires_simulation {
                requires_simulation += 1;
            }

            processed += 1;

            // Progress update every 100 transactions
            if processed % 100 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let rate = processed as f64 / elapsed;
                info!(
                    "Progress: {}/{} transactions ({:.0} tx/sec)",
                    processed, target_count, rate
                );

                // Check cache growth
                let current_pools = token_cache.get_pool_count().await;
                let current_creators = token_cache.get_creator_count().await;
                if current_pools > initial_pools || current_creators > initial_creators {
                    info!(
                        "  Cache updated: {} pools (+{}), {} creators (+{})",
                        current_pools,
                        current_pools - initial_pools,
                        current_creators,
                        current_creators - initial_creators
                    );
                }
            }

            if processed >= target_count {
                break;
            }
        }
    }

    let total_elapsed = start_time.elapsed();

    // Final cache state
    let final_pools = token_cache.get_pool_count().await;
    let final_creators = token_cache.get_creator_count().await;

    // Count unique tokens from pools for final state
    let final_sample_pools = token_cache.pools_snapshot().await;
    let final_tokens: usize = final_sample_pools
        .iter()
        .map(|(_, pool)| &pool.token_address)
        .collect::<std::collections::HashSet<_>>()
        .len();

    // Write detailed summary
    writeln!(log_file, "\n\n=======================================")?;
    writeln!(log_file, "FINAL SUMMARY")?;
    writeln!(log_file, "=======================================")?;
    writeln!(log_file, "\nProcessing Statistics:")?;
    writeln!(log_file, "  Total Transactions: {}", processed)?;
    writeln!(
        log_file,
        "  Time Elapsed: {:.2}s",
        total_elapsed.as_secs_f64()
    )?;
    writeln!(
        log_file,
        "  Processing Rate: {:.0} tx/sec",
        processed as f64 / total_elapsed.as_secs_f64()
    )?;

    writeln!(log_file, "\nTransaction Categories:")?;
    writeln!(
        log_file,
        "  Contract Creations: {} ({:.1}%)",
        contract_creations,
        (contract_creations as f64 / processed as f64) * 100.0
    )?;
    writeln!(
        log_file,
        "  Creator Actions: {} ({:.1}%)",
        creator_actions,
        (creator_actions as f64 / processed as f64) * 100.0
    )?;
    writeln!(
        log_file,
        "  DEX Interactions: {} ({:.1}%)",
        dex_interactions,
        (dex_interactions as f64 / processed as f64) * 100.0
    )?;
    writeln!(
        log_file,
        "  Regular Transactions: {} ({:.1}%)",
        regular_txs,
        (regular_txs as f64 / processed as f64) * 100.0
    )?;

    writeln!(log_file, "\nPriority Analysis:")?;
    writeln!(
        log_file,
        "  High Priority: {} ({:.1}%)",
        high_priority,
        (high_priority as f64 / processed as f64) * 100.0
    )?;
    writeln!(
        log_file,
        "  Requiring Simulation: {} ({:.1}%)",
        requires_simulation,
        (requires_simulation as f64 / processed as f64) * 100.0
    )?;

    writeln!(log_file, "\nToken Cache Performance:")?;
    writeln!(
        log_file,
        "  Initial State: {} pools, {} creators, {} tokens",
        initial_pools, initial_creators, initial_tokens
    )?;
    writeln!(
        log_file,
        "  Final State: {} pools, {} creators, {} tokens",
        final_pools, final_creators, final_tokens
    )?;
    writeln!(
        log_file,
        "  New Pools Added: {}",
        final_pools.saturating_sub(initial_pools)
    )?;
    writeln!(
        log_file,
        "  New Creators Added: {}",
        final_creators.saturating_sub(initial_creators)
    )?;
    writeln!(
        log_file,
        "  New Tokens Added: {}",
        final_tokens.saturating_sub(initial_tokens)
    )?;
    writeln!(
        log_file,
        "  Creator Cache Hits: {} / {} ({:.1}%)",
        creator_cache_hits,
        creator_actions,
        if creator_actions > 0 {
            (creator_cache_hits as f64 / creator_actions as f64) * 100.0
        } else {
            0.0
        }
    )?;
    writeln!(
        log_file,
        "  Pool Cache Hits: {} / {} ({:.1}%)",
        pool_cache_hits,
        dex_interactions,
        if dex_interactions > 0 {
            (pool_cache_hits as f64 / dex_interactions as f64) * 100.0
        } else {
            0.0
        }
    )?;

    log_file.flush()?;

    // Console summary
    info!("\n📊 === Transaction Routing Complete ===");
    info!(
        "✅ Processed: {} transactions in {:.2}s ({:.0} tx/sec)",
        processed,
        total_elapsed.as_secs_f64(),
        processed as f64 / total_elapsed.as_secs_f64()
    );
    info!("\n📈 Categories:");
    info!(
        "   Contract Creations: {} ({:.1}%)",
        contract_creations,
        (contract_creations as f64 / processed as f64) * 100.0
    );
    info!(
        "   Creator Actions: {} ({:.1}%)",
        creator_actions,
        (creator_actions as f64 / processed as f64) * 100.0
    );
    info!(
        "   DEX Interactions: {} ({:.1}%)",
        dex_interactions,
        (dex_interactions as f64 / processed as f64) * 100.0
    );
    info!(
        "   Regular: {} ({:.1}%)",
        regular_txs,
        (regular_txs as f64 / processed as f64) * 100.0
    );
    info!("\n💾 Token Cache:");
    info!(
        "   Pools: {} → {} (+{})",
        initial_pools,
        final_pools,
        final_pools.saturating_sub(initial_pools)
    );
    info!(
        "   Creators: {} → {} (+{})",
        initial_creators,
        final_creators,
        final_creators.saturating_sub(initial_creators)
    );
    info!(
        "   Tokens: {} → {} (+{})",
        initial_tokens,
        final_tokens,
        final_tokens.saturating_sub(initial_tokens)
    );
    info!(
        "   Cache Hit Rate: {:.1}% creators, {:.1}% pools",
        if creator_actions > 0 {
            (creator_cache_hits as f64 / creator_actions as f64) * 100.0
        } else {
            0.0
        },
        if dex_interactions > 0 {
            (pool_cache_hits as f64 / dex_interactions as f64) * 100.0
        } else {
            0.0
        }
    );
    info!("\n📁 Full log written to: {}", log_path.display());

    Ok(())
}

fn build_token_subscriber(threshold: f64) -> TokenTrackingSubscriber {
    let pub_endpoint = std::env::var("TOKEN_CACHE_PUB_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_TOKEN_CACHE_PUB_ENDPOINT.to_string());
    let rep_endpoint = std::env::var("TOKEN_CACHE_REP_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_TOKEN_CACHE_REP_ENDPOINT.to_string());
    let redis_url = std::env::var("TOKEN_SNAPSHOT_REDIS_URL")
        .unwrap_or_else(|_| DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string());
    let redis_prefix = std::env::var("TOKEN_SNAPSHOT_REDIS_PREFIX")
        .unwrap_or_else(|_| DEFAULT_REDIS_TOKEN_PREFIX.to_string());

    println!(
        "Token snapshot sources: redis={}, pub={}, rep={}",
        redis_url, pub_endpoint, rep_endpoint
    );

    TokenTrackingSubscriber::with_sources(
        threshold,
        &pub_endpoint,
        &rep_endpoint,
        &redis_url,
        &redis_prefix,
    )
}
