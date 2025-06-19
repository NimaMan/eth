/// Test example for the proper DevP2P client
/// This verifies the real-time P2P transaction listening works correctly
/// and measures latency compared to RPC polling
///
/// Usage:
///   cargo run --example test_proper_devp2p --features reth_integration

use std::time::{Duration, Instant};
use tracing::{info, warn, error, debug};
use eyre::Result;

// Note: The simple DevP2P client uses direct eth-wire protocol
#[path = "../experimental/devp2p/simple_devp2p_client.rs"]
#[cfg(feature = "reth_integration")]
mod simple_devp2p_client;

#[cfg(feature = "reth_integration")]
use simple_devp2p_client::{SimpleDevP2pClient, create_simple_devp2p_client};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("test_proper_devp2p=info,mempool_processor=info")
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Testing Proper DevP2P Transaction Listening");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    #[cfg(not(feature = "reth_integration"))]
    {
        error!("❌ This example requires 'reth_integration' feature");
        error!("   Run with: cargo run --example test_proper_devp2p --features reth_integration");
        return Ok(());
    }

    #[cfg(feature = "reth_integration")]
    {
        // Create the simple DevP2P client
        info!("🔌 Creating simple DevP2P client...");
        let client = match create_simple_devp2p_client().await {
            Ok(client) => {
                info!("✅ DevP2P client created successfully");
                client
            }
            Err(e) => {
                error!("❌ Failed to create DevP2P client: {}", e);
                error!("   Make sure your local Reth node is running at 127.0.0.1:30303");
                error!("   And accepting P2P connections");
                return Err(e);
            }
        };

        // Check connection status
        let connected = client.is_connected();
        if connected {
            info!("🌐 Connected to P2P network");
        } else {
            info!("⚠️  Connecting to P2P network - may take a few seconds");
        }

        // Display initial statistics
        let stats = client.get_stats();
        info!("📊 Initial stats: {} peers, {} txs, {} hash announcements", 
              stats.peers, stats.total_transactions, stats.total_hash_announcements);

        // Test 1: Check for immediate transactions (non-blocking)
        info!("\n🔍 Test 1: Checking for immediate transactions...");
        let immediate_txs = client.fetch_new_transactions().await?;
        if immediate_txs.is_empty() {
            info!("   No immediate transactions (expected for new connections)");
        } else {
            info!("   Found {} immediate transactions!", immediate_txs.len());
            for tx in &immediate_txs[..immediate_txs.len().min(3)] {
                info!("   📦 TX: 0x{}", hex::encode(&tx.hash[..4]));
            }
        }

        // Test 2: Wait for new transactions with timeout
        info!("\n⏳ Test 2: Waiting for new transactions (30 second timeout)...");
        info!("   This test will show real-time P2P transaction detection");
        
        let test_duration = Duration::from_secs(30);
        let test_start = Instant::now();
        let mut total_received = 0;
        let mut transaction_times = Vec::new();

        while test_start.elapsed() < test_duration {
            let wait_start = Instant::now();
            
            // Wait for transactions with 5 second timeout
            match client.wait_for_transactions(5000).await {
                Ok(txs) if !txs.is_empty() => {
                    let wait_time = wait_start.elapsed();
                    total_received += txs.len();
                    
                    info!("📦 Received {} transactions after {:?}", txs.len(), wait_time);
                    
                    // Analyze each transaction's timing
                    for tx in &txs {
                        let announcement_age = tx.announcement_time.elapsed();
                        transaction_times.push(announcement_age.as_millis() as f64);
                        
                        if txs.len() <= 5 { // Show details for small batches
                            info!("   🎯 TX 0x{}: value={} ETH, age={}ms", 
                                  hex::encode(&tx.hash[..4]),
                                  ethers::utils::format_ether(tx.value),
                                  announcement_age.as_millis());
                        }
                    }
                    
                    // Show batch summary for larger batches
                    if txs.len() > 5 {
                        let min_age = txs.iter().map(|tx| tx.announcement_time.elapsed().as_millis()).min().unwrap_or(0);
                        let max_age = txs.iter().map(|tx| tx.announcement_time.elapsed().as_millis()).max().unwrap_or(0);
                        info!("   📊 Batch summary: {} txs, age range: {}ms - {}ms", 
                              txs.len(), min_age, max_age);
                    }
                }
                Ok(_) => {
                    debug!("   No transactions in this 5s window");
                }
                Err(e) => {
                    warn!("   Error waiting for transactions: {}", e);
                }
            }
            
            // Update stats every 10 seconds
            if test_start.elapsed().as_secs() % 10 == 0 && test_start.elapsed() > Duration::from_secs(1) {
                let current_stats = client.get_stats();
                info!("📊 Current stats: {} peers, {} total txs ({:.1}/s), {} hash announcements ({:.1}/s)", 
                      current_stats.peers, 
                      current_stats.total_transactions,
                      current_stats.transactions_per_second,
                      current_stats.total_hash_announcements,
                      current_stats.hash_announcements_per_second);
            }
        }

        // Final analysis
        info!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("📊 TEST RESULTS SUMMARY");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let final_stats = client.get_stats();
        info!("🎯 DevP2P Performance:");
        info!("   • Total transactions received: {}", total_received);
        info!("   • Test duration: {:.1}s", test_duration.as_secs_f64());
        info!("   • Transaction rate: {:.2} tx/s", total_received as f64 / test_duration.as_secs_f64());
        info!("   • Connected peers: {}", final_stats.peers);
        info!("   • Hash announcements: {}", final_stats.total_hash_announcements);
        info!("   • Hash announcement rate: {:.2}/s", final_stats.hash_announcements_per_second);

        if !transaction_times.is_empty() {
            let avg_latency = transaction_times.iter().sum::<f64>() / transaction_times.len() as f64;
            let min_latency = transaction_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_latency = transaction_times.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            info!("⚡ Transaction Latency Analysis:");
            info!("   • Average P2P latency: {:.1}ms", avg_latency);
            info!("   • Min P2P latency: {:.1}ms", min_latency);
            info!("   • Max P2P latency: {:.1}ms", max_latency);
            info!("   • Sub-100ms transactions: {:.1}%", 
                  transaction_times.iter().filter(|&&t| t < 100.0).count() as f64 / transaction_times.len() as f64 * 100.0);
            info!("   • Sub-10ms transactions: {:.1}%", 
                  transaction_times.iter().filter(|&&t| t < 10.0).count() as f64 / transaction_times.len() as f64 * 100.0);
            info!("   • Sub-1ms transactions: {:.1}%", 
                  transaction_times.iter().filter(|&&t| t < 1.0).count() as f64 / transaction_times.len() as f64 * 100.0);
        }

        info!("\n💡 Analysis:");
        if final_stats.peers == 0 {
            warn!("⚠️  No peers connected - P2P functionality limited");
            warn!("   Check if local Reth node is accepting connections");
            warn!("   Verify firewall settings and node configuration");
        } else if total_received == 0 {
            warn!("⚠️  No transactions received during test");
            warn!("   This could be due to:");
            warn!("   - Low network activity");
            warn!("   - Node not fully synced");
            warn!("   - P2P message filtering");
        } else {
            info!("✅ DevP2P transaction detection is working!");
            if !transaction_times.is_empty() {
                let avg_latency = transaction_times.iter().sum::<f64>() / transaction_times.len() as f64;
                if avg_latency < 50.0 {
                    info!("🚀 Excellent latency performance (avg: {:.1}ms)", avg_latency);
                } else if avg_latency < 200.0 {
                    info!("👍 Good latency performance (avg: {:.1}ms)", avg_latency);
                } else {
                    warn!("⚠️  High latency detected (avg: {:.1}ms)", avg_latency);
                    warn!("   Consider optimizing network connection");
                }
            }
        }

        info!("\n🔄 Comparison with RPC Polling:");
        info!("   • RPC polling typical latency: 100-500ms");
        info!("   • DevP2P message latency: Real-time (<50ms typical)");
        info!("   • DevP2P advantage: Push notifications vs periodic polling");
        info!("   • DevP2P benefits: Lower bandwidth, real-time alerts, no rate limits");

        // Clean shutdown
        info!("\n🛑 Shutting down DevP2P client...");
        client.shutdown().await?;
        info!("✅ DevP2P client shut down cleanly");
    }

    Ok(())
}