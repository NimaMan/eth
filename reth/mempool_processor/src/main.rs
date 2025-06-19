/// Main entry point for mempool_processor
/// 
/// This binary simply prints usage information and redirects to the
/// appropriate specialized binary based on your use case.

fn main() {
    eprintln!("Mempool Processor - Please use a specific binary:");
    eprintln!();
    eprintln!("Production binaries:");
    eprintln!("  mempool_signal_detection_ipc_optimized - IPC-optimized detection (WORKING)");
    eprintln!("  mempool_tracker - Basic mempool tracking");
    eprintln!("  mempool_websocket_full - WebSocket-based monitoring");
    eprintln!();
    eprintln!("Work in progress:");
    eprintln!("  mempool_signal_detection_unified - Unified detection (needs fixes)");
    eprintln!();
    eprintln!("Performance testing:");
    eprintln!("  mempool_latency_benchmark - Benchmark mempool latency");
    eprintln!("  signal_detection_performance_benchmark - Benchmark signal detection");
    eprintln!();
    eprintln!("Utilities:");
    eprintln!("  metrics_api_server - HTTP API for metrics");
    eprintln!();
    eprintln!("For production use, run:");
    eprintln!("  cargo run --bin mempool_signal_detection_unified");
    eprintln!();
    eprintln!("Or use the convenience script:");
    eprintln!("  ./run_unified_signal_detection.sh");
    
    std::process::exit(1);
}