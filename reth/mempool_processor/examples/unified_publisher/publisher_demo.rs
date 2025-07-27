//! Unified Publisher Demo
//! 
//! Algorithm:
//! 1. Initialize unified publisher on port 5560
//! 2. Create signal router with custom rules
//! 3. Generate various signal types to demonstrate:
//!    - Tax manipulation (immediate, no simulation)
//!    - Liquidity removal (immediate + triggers simulation)
//!    - Trading enabled (needs simulation confirmation)
//!    - Pool drain (critical alert)
//! 4. Show routing decisions and publishing flow
//! 5. Display statistics

use mempool_processor::publishers::{
    UnifiedPublisher, UnifiedSignal, SignalType, SignalData, Severity, SignalSource,
    PublisherConfig, SignalRouter, HighValueRule, ScamPatternRule,
    TaxManipulationData, LiquidityRemovalData, PoolDrainData, TradingStatusData,
};
use std::collections::HashSet;
use eyre::Result;
use tracing::{info, warn};

fn generate_sample_signals() -> Vec<UnifiedSignal> {
    let mut signals = vec![];
    
    // 1. Tax Manipulation Signal (Critical)
    let tax_signal = {
        let mut signal = UnifiedSignal::new(
            SignalType::TaxManipulation,
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            "0x742d35Cc6634C0532925a3b844Bc9e7095833a06".to_string(),
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(), // USDC
            SignalData::TaxManipulation(TaxManipulationData {
                current_buy_tax: 5.0,
                current_sell_tax: 5.0,
                predicted_buy_tax: 50.0,
                predicted_sell_tax: 99.0,
                manipulator_address: "0x742d35Cc6634C0532925a3b844Bc9e7095833a06".to_string(),
                function_selector: "0x032dc6a2".to_string(),
                pattern: "HoneypotSetup".to_string(),
            }),
            "Honeypot pattern detected: sell tax increasing to 99%".to_string(),
        );
        signal.base.severity = Severity::Critical;
        signal.base.confidence = 0.95;
        signal.base.source = SignalSource::TaxDecoder;
        signal.base.gas_price_gwei = Some(35.5);
        signal.base.block_number = Some(19123456);
        signal
    };
    signals.push(tax_signal);
    
    // 2. Liquidity Removal Signal (High)
    let liquidity_signal = {
        let mut signal = UnifiedSignal::new(
            SignalType::LiquidityRemoval,
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE".to_string(),
            "0x514910771AF9Ca656af840dff83E8264EcF986CA".to_string(), // LINK
            SignalData::LiquidityRemoval(LiquidityRemovalData {
                function_name: "removeLiquidityETH".to_string(),
                liquidity_token_amount: Some(1000000.0),
                eth_amount: Some(50.5),
                token_amount: Some(100000.0),
                percentage: Some(75.0),
                pool_impact: None,
            }),
            "Large liquidity removal: 75% of pool being withdrawn".to_string(),
        );
        signal.base.severity = Severity::High;
        signal.base.confidence = 0.88;
        signal.base.pool_address = Some("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984".to_string());
        signal.base.value_eth = Some(50.5);
        signal
    };
    signals.push(liquidity_signal);
    
    // 3. Trading Enabled Signal (Medium)
    let trading_signal = {
        let mut signal = UnifiedSignal::new(
            SignalType::TradingEnabled,
            "0x567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234".to_string(),
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(), // DAI
            SignalData::TradingStatus(TradingStatusData {
                function_name: "enableTrading".to_string(),
                enabled: true,
                pool_has_liquidity: true,
                initial_liquidity_eth: Some(10.0),
                initial_liquidity_tokens: Some(10000.0),
                simulation_confirmed: false,
            }),
            "Trading enabled on DAI pool with 10 ETH liquidity".to_string(),
        );
        signal.base.severity = Severity::Medium;
        signal.base.confidence = 0.75;
        signal.base.pool_address = Some("0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11".to_string());
        signal
    };
    signals.push(trading_signal);
    
    // 4. Pool Drain Signal (Critical)
    let drain_signal = {
        let mut signal = UnifiedSignal::new(
            SignalType::PoolDrain,
            "0x9876543210fedcba9876543210fedcba9876543210fedcba9876543210fedcba".to_string(),
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(), // Uniswap Router
            "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984".to_string(), // UNI
            SignalData::PoolDrain(PoolDrainData {
                current_eth_reserve: 100.0,
                new_eth_reserve: 0.5,
                eth_drained: 99.5,
                drain_percentage: 99.5,
                current_token_reserve: 1000000.0,
                new_token_reserve: 10000.0,
                is_complete_drain: true,
            }),
            "SCAM ALERT: Pool drained 99.5% - only 0.5 ETH remaining!".to_string(),
        );
        signal.base.severity = Severity::Critical;
        signal.base.confidence = 0.99;
        signal.base.source = SignalSource::SimulationEngine;
        signal.base.pool_address = Some("0xd3d2E2692501A5c9Ca623199D38826e513033a17".to_string());
        signal
    };
    signals.push(drain_signal);
    
    signals
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("=== Unified Publisher Demo ===");
    
    // Create publisher configuration
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
    info!("✅ Publisher initialized on tcp://127.0.0.1:5560");
    
    // Create signal router with rules
    let mut router = SignalRouter::new();
    
    // Add high priority tokens
    router.add_high_priority_token("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()); // USDC
    router.add_high_priority_token("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string()); // USDT
    
    // Add watchlist addresses (known scammers)
    router.add_watchlist_address("0x742d35Cc6634C0532925a3b844Bc9e7095833a06".to_string());
    
    // Add custom rules
    router.add_rule(Box::new(HighValueRule { eth_threshold: 20.0 }));
    
    let mut scam_functions = HashSet::new();
    scam_functions.insert("renounceownership".to_string());
    scam_functions.insert("blacklist".to_string());
    router.add_rule(Box::new(ScamPatternRule { scam_functions }));
    
    info!("📋 Router configured with priority tokens and custom rules");
    
    // Generate and process signals
    let signals = generate_sample_signals();
    info!("\n🚀 Publishing {} signals...\n", signals.len());
    
    for (i, signal) in signals.iter().enumerate() {
        info!("Signal {}: {} - {}", i + 1, signal.base.signal_type.topic(), signal.base.signal_id);
        
        // Get routing decision
        let routing = router.route(signal);
        info!("  Routing Decision:");
        info!("    - Publish immediately: {}", routing.publish_immediately);
        info!("    - Requires simulation: {}", routing.requires_simulation);
        info!("    - Wait for simulation: {}", routing.wait_for_simulation);
        if !routing.trigger_signals.is_empty() {
            info!("    - Triggers: {:?}", routing.trigger_signals);
        }
        if let Some(severity) = routing.severity_override {
            info!("    - Severity override: {:?}", severity);
        }
        
        // Publish the signal
        match publisher.publish(signal) {
            Ok(_) => info!("  ✅ Published successfully on topic: {}", signal.topic()),
            Err(e) => warn!("  ❌ Failed to publish: {}", e),
        }
        
        info!("");
    }
    
    // Display statistics
    let stats = publisher.get_stats();
    info!("📊 Publisher Statistics:");
    info!("  - Total published: {}", stats.total_published);
    info!("  - Signals by type:");
    for (signal_type, count) in &stats.signals_by_type {
        info!("    - {}: {}", signal_type, count);
    }
    info!("  - Errors: {}", stats.errors);
    info!("  - Buffer drops: {}", stats.buffer_full_drops);
    
    // Keep running for a moment
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    info!("\n✅ Demo completed. Signals are being published on tcp://127.0.0.1:5560");
    info!("Run the multipart_subscriber example to receive these signals.");
    
    Ok(())
}