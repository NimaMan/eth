/// Export All Tokens From Cache to CSV
///
/// This example connects to the token tracking cache and exports comprehensive data for all tokens to a CSV file.
/// Each row contains complete information for one token including its creator, metadata, tax settings,
/// and up to 3 pools with their liquidity and trading status.
///
/// Usage: cargo run --example export_all_tokens_from_cache_to_csv [output.csv]
use mempool_processor::token_tracking::TokenTrackingSubscriber;
use std::fs::File;
use std::io::Write;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("📊 Token Data CSV Exporter");
    info!("==========================");

    // Get output filename from args or use default
    let args: Vec<String> = std::env::args().collect();
    let output_file = if args.len() > 1 {
        &args[1]
    } else {
        "token_data_export.csv"
    };

    // Create subscriber with 0.05 ETH threshold
    let mut subscriber = TokenTrackingSubscriber::new(0.05);
    let cache = subscriber.get_cache();

    // Start the subscriber in background
    info!("Starting token tracking subscriber...");
    tokio::spawn(async move {
        if let Err(e) = subscriber.start_listening().await {
            error!("Subscriber error: {}", e);
        }
    });

    // Wait for initial data to load
    info!("Waiting for cache to populate...");
    sleep(Duration::from_secs(5)).await;

    // Get cache statistics
    let stats = cache.stats().await;
    info!(
        "Cache loaded: {} tokens, {} pools, {} creators",
        stats.total_tokens, stats.total_pools, stats.total_creators
    );

    if stats.total_tokens == 0 {
        warn!("No tokens found in cache. Is the Python publisher running?");
        warn!("Make sure the token tracking publisher is running on ports 5557/5558");
        return Ok(());
    }

    // Open CSV file for writing
    let mut file = File::create(output_file)?;

    // Write CSV header
    writeln!(file, "creator_address,token_address,token_symbol,token_name,decimals,total_supply,buy_tax,sell_tax,ownership_renounced,renouncement_block,creation_block,creation_txn,latest_activity_block,is_scam,total_liquidity,pool1_address,pool1_type,pool1_eth_reserve,pool1_token_reserve,pool1_trading_enabled,pool1_trading_block,pool2_address,pool2_type,pool2_eth_reserve,pool2_token_reserve,pool2_trading_enabled,pool2_trading_block,pool3_address,pool3_type,pool3_eth_reserve,pool3_token_reserve,pool3_trading_enabled,pool3_trading_block")?;

    // Get all creators
    let all_creators = cache.creator_addresses().await;
    info!("Processing {} creators...", all_creators.len());

    let mut total_tokens = 0;
    let mut total_pools = 0;

    // Process each creator
    for creator in all_creators.iter() {
        // Get all tokens created by this address
        let tokens = cache.get_tokens_by_creator(creator).await;

        for token in tokens {
            total_tokens += 1;

            // Get all pools for this token
            let pools = cache.get_pools_for_token(&token.address).await;
            total_pools += pools.len();

            // Start building the CSV row
            let mut row = format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6}",
                creator,
                token.address,
                csv_escape(&token.symbol),
                csv_escape(&token.name),
                token.decimals,
                token.total_supply.as_ref().unwrap_or(&String::new()),
                token.buy_tax.map_or(String::new(), |t| format!("{:.2}", t)),
                token
                    .sell_tax
                    .map_or(String::new(), |t| format!("{:.2}", t)),
                token.ownership_renounced,
                token
                    .renouncement_block
                    .map_or(String::new(), |b| b.to_string()),
                token.creation_block,
                token.creation_txn,
                token.latest_activity_block,
                token.is_scam,
                pools.iter().map(|p| p.eth_reserve).sum::<f64>()
            );

            // Add pool data (up to 3 pools)
            for pool in pools.iter().take(3) {
                row.push_str(&format!(
                    ",{},{},{:.6},{:.2},{},{}",
                    pool.address,
                    format!("{:?}", pool.pool_type),
                    pool.eth_reserve,
                    pool.token_reserve,
                    pool.trading_enabled,
                    pool.trading_enabled_block
                        .map_or(String::new(), |b| b.to_string())
                ));
            }

            // Fill empty pool columns if less than 3 pools
            for _ in pools.len()..3 {
                row.push_str(",,,,,,");
            }

            writeln!(file, "{}", row)?;

            // Progress indicator every 100 tokens
            if total_tokens % 100 == 0 {
                info!("  Processed {} tokens...", total_tokens);
            }
        }
    }

    info!("✅ Export complete!");
    info!("  Total tokens exported: {}", total_tokens);
    info!("  Total pools included: {}", total_pools);
    info!("  Output file: {}", output_file);

    Ok(())
}

/// Escape CSV fields that contain commas or quotes
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
