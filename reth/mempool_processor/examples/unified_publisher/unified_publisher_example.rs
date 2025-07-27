//! Example of using the Unified Publisher system

use mempool_processor::publishers::{
    UnifiedPublisher, UnifiedSignal, SignalType, SignalData, 
    TaxManipulationData, PublisherConfig, Severity, SignalSource,
    SignalRouter, HighValueRule,
};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create publisher with custom config
    let config = PublisherConfig {
        endpoint: "tcp://127.0.0.1:5560".to_string(),
        high_water_mark: 10000,
        linger_ms: 0,
        send_timeout_ms: 1000,
        non_blocking: true,
        min_severity: Severity::Low,
        min_confidence: 0.5,
    };

    let publisher = UnifiedPublisher::new(config)?;
    
    // Create a signal router
    let mut router = SignalRouter::new();
    
    // Add high priority tokens
    router.add_high_priority_token("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()); // USDC
    
    // Add custom rule for high value transactions
    router.add_rule(Box::new(HighValueRule { eth_threshold: 10.0 }));

    // Example 1: Tax Manipulation Signal
    println!("Publishing tax manipulation signal...");
    
    let tax_data = TaxManipulationData {
        current_buy_tax: 5.0,
        current_sell_tax: 5.0,
        predicted_buy_tax: 50.0,
        predicted_sell_tax: 95.0,
        manipulator_address: "0x742d35Cc6634C0532925a3b844Bc9e7095833a06".to_string(),
        function_selector: "0x032dc6a2".to_string(),
        pattern: "HoneypotSetup".to_string(),
    };

    let mut tax_signal = UnifiedSignal::new(
        SignalType::TaxManipulation,
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
        "0x742d35Cc6634C0532925a3b844Bc9e7095833a06".to_string(),
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        SignalData::TaxManipulation(tax_data),
        "Detected honeypot pattern: sell tax increasing to 95%".to_string(),
    );
    
    // Set additional fields
    tax_signal.base.severity = Severity::Critical;
    tax_signal.base.confidence = 0.95;
    tax_signal.base.source = SignalSource::TaxDecoder;
    tax_signal.base.detection_latency_us = 2500;
    tax_signal.base.gas_price_gwei = Some(30.5);

    // Check routing decision
    let routing = router.route(&tax_signal);
    println!("Routing decision: {:?}", routing);

    // Publish the signal
    publisher.publish(&tax_signal)?;
    println!("✅ Tax manipulation signal published on topic: {}", tax_signal.topic());

    // Example 2: Simple signal using generic data
    println!("\nPublishing trading enabled signal...");
    
    let trading_data = serde_json::json!({
        "function_name": "enableTrading",
        "pool_address": "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984",
        "initial_liquidity_eth": 10.5,
        "initial_liquidity_tokens": 1000000.0,
    });

    publisher.publish_simple(
        SignalType::TradingEnabled,
        "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
        "0x514910771AF9Ca656af840dff83E8264EcF986CA",
        trading_data,
        "Trading enabled on new LINK pool with 10.5 ETH liquidity",
    )?;

    println!("✅ Trading enabled signal published");

    // Show statistics
    let stats = publisher.get_stats();
    println!("\nPublisher Statistics:");
    println!("- Total published: {}", stats.total_published);
    println!("- Signals by type:");
    for (signal_type, count) in &stats.signals_by_type {
        println!("  - {}: {}", signal_type, count);
    }
    println!("- Errors: {}", stats.errors);
    println!("- Buffer drops: {}", stats.buffer_full_drops);

    // Keep running for a moment to ensure messages are sent
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}