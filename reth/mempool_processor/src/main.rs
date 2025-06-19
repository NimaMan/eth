/// Main entry point for mempool_processor
/// 
/// This binary simply prints usage information and redirects to the
/// appropriate specialized binary based on your use case.

fn main() {
    eprintln!("Mempool Processor - Please use a specific binary:");
    eprintln!();
    eprintln!("Production binaries:");
    eprintln!("  mempool_signal_detection_full_tx_ipc - Full TX IPC signal detection (ACTIVE)");
    eprintln!("  mempool_tracker - Basic mempool tracking");
    eprintln!("  mempool_websocket_full - WebSocket-based monitoring");
    eprintln!();
    eprintln!("Performance testing:");
    eprintln!("  mempool_latency_benchmark - Benchmark mempool latency");
    eprintln!("  measure_mempool_timing - Measure mempool timing");
    eprintln!("  measure_full_mempool - Full mempool measurements");
    eprintln!("  measure_optimized_mempool - Optimized mempool measurements");
    eprintln!("  measure_streaming_performance - Streaming performance test");
    eprintln!("  realtime_latency_validator - Validate real-time latency");
    eprintln!();
    eprintln!("Utilities:");
    eprintln!("  metrics_api_server - HTTP API for metrics");
    eprintln!();
    eprintln!("For production use, run:");
    eprintln!("  cargo run --bin mempool_signal_detection_full_tx_ipc");
    
    std::process::exit(1);
}