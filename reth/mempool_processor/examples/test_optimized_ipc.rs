/// Test Optimized IPC Performance
/// 
/// Run with:
///   cargo run --example test_optimized_ipc

use mempool_processor::mempool_fetcher::ipc_socket::optimized_client::test_optimized_performance;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    test_optimized_performance().await
}