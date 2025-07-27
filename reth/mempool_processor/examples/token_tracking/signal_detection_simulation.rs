// examples/token_tracking/signal_detection_simulation.rs
//
// Example simulating the complete signal detection flow:
// 1. Load token data from Python
// 2. Simulate mempool transactions from creators/owners
// 3. Show how signals would be generated

use mempool_processor::token_tracking::{
    AddressTrackingCache, 
    SignalIntegration,
    AddressRole,
};
use mempool_processor::signal_engine::function_detector::DetectedFunction;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting signal detection simulation");
    
    // Create cache and signal integration
    let cache = AddressTrackingCache::new();
    let signal_integration = SignalIntegration::new(cache.clone());
    
    // Simulate loading token data from Python
    info!("📊 Step 1: Loading sample token data");
    await_setup_sample_tokens(&cache).await;
    
    // Show current cache state
    let stats = cache.get_stats().await;
    info!("Cache loaded: {} addresses, {} tokens, {} pools", 
        stats.tracked_addresses, stats.tracked_tokens, stats.tracked_pools);
    
    // Simulate mempool transactions
    info!("🔄 Step 2: Simulating mempool transactions");
    
    let test_cases = vec![
        // Case 1: Creator removing liquidity (CRITICAL)
        TestCase {
            from_address: "0xcreator1111111111111111111111111111111111",
            functions: vec![DetectedFunction {
                selector: [0xba, 0xa2, 0xab, 0xde],
                name: "removeLiquidity".to_string(),
                signature: "removeLiquidity(uint256,uint256,uint256,uint256,address,uint256)".to_string(),
            }],
            description: "Creator removing liquidity",
        },
        
        // Case 2: Owner enabling trading (MEDIUM)
        TestCase {
            from_address: "0xowner22222222222222222222222222222222222",
            functions: vec![DetectedFunction {
                selector: [0x8a, 0x8c, 0x52, 0x3c],
                name: "enableTrading".to_string(),
                signature: "enableTrading()".to_string(),
            }],
            description: "Owner enabling trading",
        },
        
        // Case 3: Unknown address with critical function (NO SIGNAL)
        TestCase {
            from_address: "0xunknown3333333333333333333333333333333333",
            functions: vec![DetectedFunction {
                selector: [0xba, 0xa2, 0xab, 0xde],
                name: "removeLiquidity".to_string(),
                signature: "removeLiquidity(uint256,uint256,uint256,uint256,address,uint256)".to_string(),
            }],
            description: "Unknown address removing liquidity",
        },
        
        // Case 4: Creator with multiple functions
        TestCase {
            from_address: "0xcreator1111111111111111111111111111111111",
            functions: vec![
                DetectedFunction {
                    selector: [0xa9, 0x05, 0x9c, 0xbb],
                    name: "transfer".to_string(),
                    signature: "transfer(address,uint256)".to_string(),
                },
                DetectedFunction {
                    selector: [0x09, 0x5e, 0xa7, 0xb3],
                    name: "approve".to_string(),
                    signature: "approve(address,uint256)".to_string(),
                },
            ],
            description: "Creator with normal functions",
        },
        
        // Case 5: High-risk creator
        TestCase {
            from_address: "0xscammer444444444444444444444444444444444",
            functions: vec![DetectedFunction {
                selector: [0xba, 0xa2, 0xab, 0xde],
                name: "removeLiquidity".to_string(),
                signature: "removeLiquidity(uint256,uint256,uint256,uint256,address,uint256)".to_string(),
            }],
            description: "High-risk scammer removing liquidity",
        },
    ];
    
    for (i, test_case) in test_cases.iter().enumerate() {
        info!("\n🧪 Test Case {}: {}", i + 1, test_case.description);
        
        let signals = signal_integration.process_mempool_transaction(
            &test_case.from_address,
            &format!("0x{:064x}", i),
            &test_case.functions,
            1700000000 + i as u64,
        ).await;
        
        if signals.is_empty() {
            info!("  📊 No signals generated (address not tracked or function not critical)");
        } else {
            for signal in &signals {
                warn!("  🚨 SIGNAL: {} calling {} on token {} (Severity: {:?})", 
                    signal.creator_address,
                    signal.function_name,
                    signal.token_address,
                    signal.severity
                );
            }
        }
        
        // Show if address is tracked
        if let Some(addr_info) = cache.get_address_info(&test_case.from_address).await {
            info!("  📍 Address tracked: {} tokens, {} function calls", 
                addr_info.tokens.len(), 
                addr_info.function_history.len()
            );
        } else {
            info!("  📍 Address not tracked");
        }
    }
    
    // Show final cache state
    info!("\n📈 Final Cache State:");
    let final_stats = cache.get_stats().await;
    info!("  Tracked Addresses: {}", final_stats.tracked_addresses);
    info!("  High Risk Addresses: {}", final_stats.high_risk_addresses);
    
    // Show function call history for tracked addresses
    info!("\n📞 Function Call History:");
    for test_case in &test_cases {
        if let Some(addr_info) = cache.get_address_info(&test_case.from_address).await {
            if !addr_info.function_history.is_empty() {
                info!("  {}: {} calls", test_case.from_address, addr_info.function_history.len());
                for call in addr_info.function_history.iter().take(3) {
                    info!("    {} - {}", call.function_name, call.tx_hash);
                }
            }
        }
    }
    
    info!("\n✅ Signal detection simulation completed");
    
    Ok(())
}

struct TestCase {
    from_address: &'static str,
    functions: Vec<DetectedFunction>,
    description: &'static str,
}

async fn setup_sample_tokens(cache: &AddressTrackingCache) {
    // Token 1: Normal token with separate creator and owner
    cache.update_token_data(
        "0xtoken1111111111111111111111111111111111111",
        "0xcreator1111111111111111111111111111111111",
        "0xowner22222222222222222222222222222222222",
        vec![
            ("0xpool1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a".to_string(), 15.5, 2000000.0),
            ("0xpool1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b".to_string(), 5.2, 800000.0),
        ],
        true,
        false,
    ).await;
    
    // Token 2: Scam token (same person is creator and owner)  
    cache.update_token_data(
        "0xtoken2222222222222222222222222222222222222",
        "0xscammer444444444444444444444444444444444",
        "0xscammer444444444444444444444444444444444",
        vec![
            ("0xpool2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a".to_string(), 2.1, 5000000.0),
        ],
        false,
        true,
    ).await;
    
    // Mark the scammer as high risk
    cache.mark_address_high_risk(
        "0xscammer444444444444444444444444444444444", 
        "Known scammer with multiple rugpull tokens"
    ).await;
    
    info!("✅ Setup complete: 2 tokens, 3 pools, 3 tracked addresses");
}